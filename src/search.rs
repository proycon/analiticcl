use crate::types::*;


pub const TRANSITION_SMOOTHING_LOGPROB: f32 = -13.815510557964274;

/// Byte Offset
#[derive(PartialEq,Clone,Debug)]
pub struct Offset {
    ///Begin offset
    pub begin: usize,
    ///End offset
    pub end: usize,
}

/// Represents a match between the input text and the lexicon.
#[derive(Clone,Debug)]
pub struct Match<'a> {
    ///The text of this match, corresponding to the input text.
    pub text: &'a str,

    /// The byte offset where this match was found in the larger text
    pub offset: Offset,

    /// The variants for this match (sorted by decreasing score)
    pub variants: Option<Vec<(VocabId, f64)>>,

    ///the variant that was selected after searching and ranking (if any)
    pub selected: Option<usize>,

    /// the index of the previous boundary, None if at start position
    pub prevboundary: Option<usize>,

    /// the index of the next boundary
    pub nextboundary: Option<usize>,

    /// The number of tokens (boundaries spanned)
    pub n: usize
}

impl<'a> Match<'a> {
    pub fn new_empty(text: &'a str, offset: Offset) -> Self {
        Match {
            text,
            offset,
            variants: None,
            selected: None,
            prevboundary: None,
            nextboundary: None,
            n: 0
        }
    }

    /// Empty matches are matches without variants
    pub fn is_empty(&self) -> bool {
        self.variants.is_none() || self.variants.as_ref().unwrap().is_empty()
    }

    /// Returns the solution if there is one
    pub fn solution(&self) -> Option<(VocabId,f64)> {
        if let Some(selected) = self.selected {
            self.variants.as_ref().expect("match must have variants when 'selected' is set").get(selected).map(|x| *x)
        } else {
            None
        }
    }

    /// Returns all boundaries that are inside this match
    pub fn internal_boundaries(&self, boundaries: &'a [Match<'_>]) -> &'a [Match<'_>] {
        let mut begin = None;
        let mut end = 0;
        for (i, boundary) in boundaries.iter().enumerate() {
            if boundary.offset.begin > self.offset.begin && boundary.offset.end < self.offset.end {
                if begin.is_none() {
                    begin = Some(i);
                } else {
                    end = i+1;
                }
            }
        }
        if begin.is_none() || begin.unwrap() >= end {
                &[]
        } else {
                &boundaries[begin.unwrap()..end]
        }
    }
}



#[derive(Clone,Debug)]
/// Refers to a match and its unigram context
pub struct Context<'a> {
    pub left: Option<&'a str>,
    pub right: Option<&'a str>
}



/// Intermediate datastructure tied to the Finite State Transducer used in most_likely_sequence()
/// Holds the output symbol for each FST state and allows relating output symbols back to the input
/// structures.
#[derive(PartialEq,PartialOrd,Clone,Debug)]
pub struct OutputSymbol {
    /// The vocabulary Id representing this output symbol, we reserve the special value 0 to
    /// indicate there is no vocabulary item associated, but the symbol is out-of-vocabulary
    /// and should be copied from the input as-is
    pub vocab_id: VocabId,
    /// Refers back to the index in the matches Vector that holds the Match that corresponds with
    /// input.
    pub match_index: usize,
    /// The variant in the Match that was selected
    pub variant_index: Option<usize>,
    /// Index of the next/right buondary in the boundaries vector
    pub boundary_index: usize,
    /// ID of this symbol (each symbol is unlike, but multiple symbols can refers to the same vocab_id).
    /// The 0 symbol is reserved for epsilon in the underlying FST implementation
    pub symbol: usize,
}


///A complete sequence of output symbols with associated emission and language model (log)
///probabilities.
#[derive(Clone,Debug)]
pub struct Sequence {
    pub output_symbols: Vec<OutputSymbol>,
    pub variant_cost: f32,
    pub lm_logprob: f32,
    pub perplexity: f64,
}

impl Sequence {
    pub fn new(variant_cost: f32) -> Self {
        Self {
            output_symbols: Vec::new(),
            variant_cost,
            lm_logprob: 0.0,
            perplexity: 0.0,
        }
    }

}

#[derive(PartialEq,PartialOrd,Copy,Clone,Debug)]
pub enum BoundaryStrength {
    None,
    /// A weak token boundary, the system is inclined to ignore it and keep the parts as one token
    Weak,
    /// A normal token boundary, the system may decide to undo it
    Normal,
    /// A hard boundary is one that is always respected
    Hard
}



/// Given a text string, identify at what points token boundaries
/// occur, for instance between alphabetic characters and punctuation.
/// The text string always ends with a boundary (but it may be a dummy one that covers no length).
pub fn find_boundaries<'a>(text: &'a str) -> Vec<Match<'a>> {
    let mut boundaries = Vec::new();

    //boundary begin
    let mut begin: Option<usize> = None;

    for (i,c) in text.char_indices() {
        if let Some(b) = begin {
            if c.is_alphabetic() {
                //boundary ends here
                boundaries.push(Match::new_empty(&text[b..i], Offset {
                    begin: b,
                    end: i
                }));
                begin = None;
            }
        } else {
            if !c.is_alphabetic() {
                //boundary starts here
                begin = Some(i);
            }
        }
    }

    //don't forget the last one
    if let Some(b) = begin {
        //either we finish the existing one
        boundaries.push(Match::new_empty(&text[b..], Offset {
            begin: b,
            end: text.len()
        }));
    } else {
        //or we add a dummy last one
        boundaries.push(Match::new_empty("", Offset {
            begin: text.len(),
            end: text.len()
        }));
    }

    boundaries
}

/// Classify the token boundaries as detected by `find_boundaries` as
/// either weak, normal or hard boundaries. This information determines
/// how eager the system is to split on certain boundaries.
pub fn classify_boundaries(boundaries: &Vec<Match<'_>>) -> Vec<BoundaryStrength> {
    let mut strengths = Vec::new();


    for (i, boundary) in boundaries.iter().enumerate() {
        let strength = if i == boundaries.len() - 1 {
            //last boundary is always a hard one
            BoundaryStrength::Hard
        } else if boundary.text.len() > 1 {
            //multichar boundaries are hard ones
            BoundaryStrength::Hard
        } else {
            match boundary.text {
                "'" | "-" | "_" => BoundaryStrength::Weak,
                _ => BoundaryStrength::Normal
            }
        };
        strengths.push(strength)
    }

    strengths
}

/// Find all ngrams in the text of the specified order, respecting the boundaries.
/// This will return a vector of Match instances, referring to the precise (untokenised) text.
pub fn find_match_ngrams<'a>(text: &'a str, boundaries: &[Match<'a>], order: u8, begin: usize, end: Option<usize>) -> Vec<Match<'a>> {
    let mut ngrams = Vec::new();

    let mut begin = begin;
    let end = end.unwrap_or(text.len());
    let mut i = 0;
    while let Some(boundary) = boundaries.get(i + order as usize - 1) {
        if boundary.offset.begin > end {
            break;
        }
        let matchtext = &text[begin..boundary.offset.begin];
        if !matchtext.is_empty() && matchtext != " " {
            let mut ngram = Match::new_empty(matchtext, Offset {
                    begin: begin,
                    end: boundary.offset.begin,
            });
            ngram.n = order as usize;
            ngrams.push(ngram);
        }
        begin = boundaries.get(i).expect("boundary").offset.end;
        i += 1;
    }

    //add the last one
    if begin < end {
        let matchtext = &text[begin..end];
        if !matchtext.is_empty() && matchtext != " " {
            let mut ngram = Match::new_empty(matchtext, Offset {
                    begin: begin,
                    end: end,
            });
            ngram.n = order as usize;
            if ngram.internal_boundaries(boundaries).iter().count() == order as usize {
                ngrams.push(ngram);
            }
        }
    }

    ngrams
}


/// A redundant match is a higher order match which already scores a perfect score when its unigram
/// components are considered separately.
pub fn redundant_match<'a>(candidate: &Match<'a>, matches: &[Match<'a>]) -> bool {
    for refmatch in matches.iter() {
        if refmatch.n == 1 {
            if refmatch.offset.begin >= candidate.offset.begin && refmatch.offset.end <= candidate.offset.end {
                if let Some(variants) = &refmatch.variants {
                    if variants.is_empty() || variants.get(0).expect("variant").1 < 1.0 {
                        return false; //non-perfect score, so not redundant
                    }
                } else {
                    return false; //no variants at all, so not redundant
                }
            }
        } else {
            break; //based on the assumption that all unigrams are at the beginning of the vector! (which should be valid in this implementation)
        }
    }
    true
}


