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
        if begin.is_none() {
                &[]
        } else {
                &boundaries[begin.unwrap()..end]
        }
    }
}


pub struct StateInfo<'a> {
    pub input: Option<&'a str>,
    pub output: Option<VocabId>,
    pub match_index: usize,
    pub variant_index: Option<usize>,
    pub emission_logprob: f32,
    pub offset: Option<Offset>
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
pub fn find_match_ngrams<'a>(text: &'a str, boundaries: &[Match<'a>], order: u8, offset: usize) -> Vec<(Match<'a>,u8)> {
    let mut ngrams = Vec::new();

    let mut begin = 0;
    let mut i = 0;
    while let Some(boundary) = boundaries.get(i + order as usize - 1) {
        let ngram = Match::new_empty(&text[begin..boundary.offset.begin], Offset {
                begin: begin + offset,
                end: boundary.offset.begin + offset,
        });
        eprintln!("Found ngram: {}", ngram.text);
        begin = boundaries.get(i).expect("boundary").offset.end;
        i += 1;
        ngrams.push((ngram,order));
    }

    //add the last one
    if begin < text.len() {
        let ngram = Match::new_empty(&text[begin..], Offset {
                begin: begin + offset,
                end: text.len() + offset,
        });
        ngrams.push((ngram,order));
    }

    ngrams
}



