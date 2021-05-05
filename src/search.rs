use std::io::{self, BufReader,BufRead,Read};
use std::time::SystemTime;

use crate::types::*;
use crate::vocab::*;


/// Byte Offset
#[derive(PartialEq,Debug)]
pub struct Offset {
    ///Begin offset
    pub begin: usize,
    ///End offset
    pub end: usize,
}

pub struct Match<'a> {
    ///The text of this match
    pub text: &'a str,

    /// The byte offset where this match was found in the larger text
    pub offset: Offset,

    /// The variants for this match (sorted)
    pub variants: Option<Vec<(VocabId, f64)>>
}

impl<'a> Match<'a> {
    pub fn new_empty(text: &'a str, offset: Offset) -> Self {
        Match {
            text,
            offset,
            variants: None,
        }
    }

    /// Empty matches are matches without variants
    pub fn is_empty(&self) -> bool {
        self.variants.is_none() || self.variants.as_ref().unwrap().is_empty()
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
pub fn find_ngrams<'a>(text: &'a str, boundaries: &[Match<'a>], strengths: &[BoundaryStrength], order: u8, offset: usize) -> Vec<(Match<'a>,u8)> {
    let mut ngrams = Vec::new();
    //TODO: Implement
    ngrams
}

/// Find one segmentation that maximizes the variant scores
pub fn consolidate_matches<'a>(matches: Vec<(Match<'a>,u8)>, boundaries: &[Match<'a>], strengths: &[BoundaryStrength], offset: usize) -> Vec<Match<'a>> {
    let mut segmentation = Vec::new();
    //TODO: Implement
    segmentation
}

