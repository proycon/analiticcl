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
    pub fn solution(&self) -> Option<VocabId> {
        if let Some(selected) = self.selected {
            self.variants.as_ref().expect("match must have variants when 'selected' is set").get(selected).map(|x| x.0)
        } else {
            None
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
        boundaries.push(Match::new_empty(&text[b..], Offset {
            begin: b,
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

/// Find all ngrams in the text of the specified order, respecting the boundaries
pub fn find_ngrams<'a>(text: &'a str, boundaries: &[Match<'a>], order: u8, offset: usize) -> Vec<(Match<'a>,u8)> {
    let mut ngrams = Vec::new();

    let mut begin = offset;
    let mut i = 0;
    while let Some(boundary) = boundaries.get(i + order as usize - 1) {
        let ngram = Match::new_empty(&text[begin..boundary.offset.begin], Offset {
                begin,
                end: boundary.offset.begin,
        });
        eprintln!("Found ngram: {}", ngram.text);
        begin = boundaries.get(i).expect("boundary").offset.end;
        i += 1;
        ngrams.push((ngram,order));
    }

    //add the last one
    if begin < text.len() {
        let ngram = Match::new_empty(&text[begin..], Offset {
                begin,
                end: text.len(),
        });
        ngrams.push((ngram,order));
    }

    ngrams
}



