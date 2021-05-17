use std::io::{self, BufReader,BufRead,Read};
use std::time::SystemTime;
use std::collections::HashMap;

use crate::types::*;
use crate::vocab::*;
use crate::index::Variant;


pub const TRANSITION_SMOOTHING_LOGPROB: f32 = -13.815510557964274;

/// Byte Offset
#[derive(PartialEq,Clone,Debug)]
pub struct Offset {
    ///Begin offset
    pub begin: usize,
    ///End offset
    pub end: usize,
}

#[derive(Clone,Debug)]
pub struct Match<'a> {
    ///The text of this match
    pub text: &'a str,

    /// The byte offset where this match was found in the larger text
    pub offset: Offset,

    /// The variants for this match (sorted)
    pub variants: Option<Vec<(VocabId, f64)>>,

    ///the variant that was selected after searching and ranking
    pub selected: Option<usize>
}


impl<'a> Match<'a> {
    pub fn new_empty(text: &'a str, offset: Offset) -> Self {
        Match {
            text,
            offset,
            variants: None,
            selected: None,
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

///Indicates an output label is out of vocabulary and should simply be copied from input
pub(crate) const OOV_EMISSION_PROB: f32 = -2.3025850929940455; //p = 0.1



#[derive(PartialEq,PartialOrd,Clone,Debug)]
pub struct OutputSymbol {
    pub vocab_id: VocabId,
    pub match_index: usize,
    pub variant_index: Option<usize>,
    pub boundary_index: usize, //index of the next/right boundary
    pub symbol: usize,
}


#[derive(Clone,Debug)]
pub struct Sequence {
    pub output_symbols: Vec<OutputSymbol>,
    pub emission_logprob: f32,
    pub lm_logprob: f32,
}

impl Sequence {
    pub fn new(emission_logprob: f32) -> Self {
        Self {
            output_symbols: Vec::new(),
            emission_logprob,
            lm_logprob: 0.0,
        }
    }

}

#[derive(PartialEq,PartialOrd,Copy,Clone,Debug)]
pub enum BoundaryStrength {
    None,
    Weak,
    Normal,
    Hard
}



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

pub fn classify_boundaries(boundaries: &Vec<Match<'_>>) -> Vec<BoundaryStrength> {
    let mut strengths = Vec::new();


    for (i, boundary) in boundaries.iter().enumerate() {
        let strength = if i == boundaries.len() - 1 {
            //last boundary is always a hard one
            BoundaryStrength::Hard
        } else if boundary.text.len() > 1 {
            //multistring boundaries are hard ones
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
        let ngram = Match::new_empty(&text[begin..boundary.offset.begin], Offset {
                begin: begin,
                end: boundary.offset.begin,
        });
        begin = boundaries.get(i).expect("boundary").offset.end;
        i += 1;
        ngrams.push(ngram);
    }

    //add the last one
    if begin < end {
        let ngram = Match::new_empty(&text[begin..end], Offset {
                begin: begin,
                end: end,
        });
        if ngram.internal_boundaries(boundaries).iter().count() == order as usize {
            ngrams.push(ngram);
        }
    }

    ngrams
}



