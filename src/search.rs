use crate::types::*;
use crate::vocab::*;

pub const TRANSITION_SMOOTHING_LOGPROB: f32 = -13.815510557964274;

/// Byte Offset
#[derive(PartialEq,Clone,Debug)]
pub struct Offset {
    ///Begin offset
    pub begin: usize,
    ///End offset
    pub end: usize,
}

impl Offset {
    pub fn convert(&mut self, map: &Vec<Option<usize>>) {
        self.begin = map.get(self.begin).expect(format!("Bytes to unicode: Begin offset {} must exist in map",self.begin).as_str()).expect("Offset in map may not be None");
        self.end = map.get(self.end).expect(format!("Bytes to unicode: End offset {} must exist in map",self.end).as_str()).expect("Offset in map may not be None");
    }
}

/// Represents a match between the input text and the lexicon.
#[derive(Clone,Debug)]
pub struct Match<'a> {
    ///The text of this match, corresponding to the input text.
    pub text: &'a str,

    /// The byte offset where this match was found in the larger text
    pub offset: Offset,

    /// The variants for this match (sorted by decreasing distance score (first score), second score is frequency score)
    pub variants: Option<Vec<VariantResult>>,

    ///the variant that was selected after searching and ranking (if any)
    pub selected: Option<usize>,


    /// The tag that was assigned to this match (if any)
    pub tag: Option<u16>,
    /// The sequence number in a tagged sequence
    pub seqnr: Option<u8>,

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
            tag: None,
            seqnr: None,
            n: 0
        }
    }

    /// Empty matches are matches without variants
    pub fn is_empty(&self) -> bool {
        self.variants.is_none() || self.variants.as_ref().unwrap().is_empty()
    }

    /// Returns the solution if there is one.
    pub fn solution(&self) -> Option<&VariantResult> {
        if let Some(selected) = self.selected {
            self.variants.as_ref().expect("match must have variants when 'selected' is set").get(selected)
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
    pub context_score: f64,
    pub tags: Vec<Option<(u16,u8)>> //tag + sequence number
}

impl Sequence {
    pub fn new(variant_cost: f32) -> Self {
        Self {
            output_symbols: Vec::new(),
            variant_cost,
            lm_logprob: 0.0,
            perplexity: 0.0,
            context_score: 1.0,
            tags: Vec::new()
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


/// A redundant match is a higher order match which already scores a perfect distance score when its unigram
/// components are considered separately.
pub fn redundant_match<'a>(candidate: &Match<'a>, matches: &[Match<'a>]) -> bool {
    for refmatch in matches.iter() {
        if refmatch.n == 1 {
            if refmatch.offset.begin >= candidate.offset.begin && refmatch.offset.end <= candidate.offset.end {
                if let Some(variants) = &refmatch.variants {
                    if variants.is_empty() || variants.get(0).expect("variant").dist_score < 1.0 {
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

#[derive(Clone,Debug)]
pub enum PatternMatch {
    /// Exact match with specific vocabulary
    Vocab(VocabId),
    /// Match with anything (?)
    Any,
    /// Match only if not found in any lexicon (^)
    NoLexicon,
    /// Match with a specific lexicon (@)
    FromLexicon(u8),
    /// Negation (^)
    Not(Box<PatternMatch>),
    /// Disjunction (|)
    Disjunction(Box<Vec<PatternMatch>>)
}


#[derive(Clone,Debug)]
pub struct ContextRule {
    /// Lexicon index
    pub pattern: Vec<PatternMatch>,
    /// Score (> 1.0) for bonus, (< 1.0) for penalty
    pub score: f32,
    pub tag: Option<u16>,
    pub tagoffset: Option<(u8,u8)> //begin,length
}

#[derive(Clone,Debug)]
pub struct PatternMatchResult {
    pub score: f32,
    pub tag: Option<u16>,
    pub seqnr: u8,
}

impl PatternMatch {
    pub fn matches(&self, sequence: &[(VocabId,u32)], index: usize) -> bool {
        match self {
            PatternMatch::Any => {
                return true;
            },
            PatternMatch::NoLexicon =>  {
                if let Some((vocabid, lexindex)) = sequence.get(index) {
                    if *lexindex == 0 || *vocabid == 0 {
                        return true;
                    }
                }
            },
            PatternMatch::Vocab(testvocabid) => {
                if let Some((vocabid, _lexindex)) = sequence.get(index) {
                    if testvocabid == vocabid {
                        return true;
                    }
                }
            },
            PatternMatch::FromLexicon(lextest) =>  {
                if let Some((_vocabid, lexindex)) = sequence.get(index) {
                    if lexindex & (1 << lextest) == 1 << lextest {
                        return true;
                    }
                }
            },
            PatternMatch::Not(pm) => {
                return !pm.matches(sequence, index);
            },
            PatternMatch::Disjunction(pms) => {
                for pm in pms.iter() {
                    if pm.matches(sequence, index) {
                        return true;
                    }
                }
            },
        };
        false
    }

    pub fn parse(s: &str, lexicons: &Vec<String>, encoder: &VocabEncoder) -> Result<Self,std::io::Error> {
        let s = s.trim();
        if s == "?" {
            Ok(Self::Any)
        } else if s == "^" {
            Ok(Self::NoLexicon)
        } else if s.starts_with("!(") && s.ends_with(")") {
            //negation over a disjunction
            let s = &s[2..s.len() - 1];
            let pm = Self::parse(s, lexicons, encoder)?;
            Ok(Self::Not(Box::new(pm)))
        } else if s.find("|").is_some() {
            let items_in: Vec<&str> = s.split("|").collect();
            let mut items_out: Vec<Self> = Vec::new();
            for item in items_in {
                match Self::parse(item, lexicons, encoder) {
                    Ok(pm) => items_out.push(pm),
                    Err(err) => return Err(err)
                };
            }
            Ok(Self::Disjunction(Box::new(items_out)))
        } else if s.starts_with("!") {
            //negation
            let s = &s[1..];
            let pm = Self::parse(s, lexicons, encoder)?;
            Ok(Self::Not(Box::new(pm)))
        } else if s.starts_with("@") {
            let source = &s[1..];
            let relsource = format!("/{}", source);
            for (i, lexicon) in lexicons.iter().enumerate() {
                if source == lexicon || lexicon.ends_with(&relsource) {
                    return Ok(Self::FromLexicon(i as u8));
                }
            }
            Err(std::io::Error::new(std::io::ErrorKind::Other, format!("WARNING: Context rule references lexicon or variant list '{}' but this source was not loaded", source)))
        } else {
            if let Some(vocab_id) = encoder.get(s) {
                return Ok(Self::Vocab(*vocab_id));
            }
            Err(std::io::Error::new(std::io::ErrorKind::Other, format!("WARNING: Context rule references word '{}' but this word does not occur in any lexicon", s)))
        }
    }
}


impl ContextRule {
    pub fn invert_score(&self) -> f32 {
        return 1.0 / self.score;
    }

    pub fn len(&self) -> usize {
        self.pattern.len()
    }

    ///Checks if the sequence of the contextrole is present in larger sequence
    ///provided as parameter. Returns the number of matches
    pub fn matches(&self, sequence: &[(VocabId,u32)], begin: usize, sequence_result: &mut Vec<Option<PatternMatchResult>>) -> bool {
        assert_eq!(sequence.len(), sequence_result.len());
        if begin + self.pattern.len() > sequence.len() {
            return false;
        }
        let mut found = true;
        for (cursor, contextmatch) in self.pattern.iter().enumerate() {
            if sequence_result[begin+cursor].is_some() || !contextmatch.matches(sequence, begin+cursor) {
                 found = false;
                 break;
            }
        }
        if found {
            for cursor in 0..self.pattern.len() {
                sequence_result[begin+cursor] =
                    Some(PatternMatchResult {
                        score: self.score,
                        tag: if self.tagoffset.is_none() {
                            self.tag
                        } else if cursor as u8 >= self.tagoffset.unwrap().0 && (cursor as u8) < self.tagoffset.unwrap().0 + self.tagoffset.unwrap().1 {
                            self.tag
                        } else {
                            None
                        },
                        seqnr: if let Some(tagoffset) = self.tagoffset {
                            cursor as u8 - tagoffset.0
                        } else {
                            cursor as u8
                        }
                    });
            }
            true
        } else {
            false
        }
    }
}


/// Remap all UTF-8 offsets to unicode codepoint offsets
pub(crate) fn remap_offsets_to_unicodepoints<'a>(text: &'a str, mut matches: Vec<Match<'a>>) -> Vec<Match<'a>> {
    let mut bytes2unicodepoints: Vec<Option<usize>> = Vec::new();
    let mut end = 0;
    for (unicodeoffset, c) in text.chars().enumerate() {
        bytes2unicodepoints.push(Some(unicodeoffset));
        for _ in 0..c.len_utf8()-1 {
            bytes2unicodepoints.push(None);
        }
        end = unicodeoffset+1;
    }
    //add an end offset
    bytes2unicodepoints.push(Some(end));
    for m in matches.iter_mut() {
        m.offset.convert(&bytes2unicodepoints);
    }
    matches
}
