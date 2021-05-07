use std::io::{self, BufReader,BufRead,Read};
use std::time::SystemTime;
use std::collections::HashMap;

use crate::types::*;
use crate::vocab::*;
use crate::index::Variant;



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


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
///These are the symbols we use in the FST
///They refer back to our loaded lexicon
pub enum SymbolReference<'a> {
    Epsilon,
    Known(VocabId),
    Unknown(&'a str),

}

const EPSILON: SymbolReference<'_> = SymbolReference::Epsilon;

impl SymbolReference<'_> {
    pub fn is_known(&self) -> bool {
        match self {
            SymbolReference::Known(_) => true,
            _ => false,
        }
    }
}

///Our own SymbolTable to decode Labels from the FST
///This one is not tied to Strings like the one in rustfst
pub struct SymbolRefTable<'a> {
    decoder: Vec<SymbolReference<'a>>,
    encoder: HashMap<SymbolReference<'a>, usize>
}

impl SymbolRefTable<'_> {
    pub fn new() -> Self {
        Self {
            decoder: Vec::new(),
            encoder: HashMap::new(),
        }
    }
}

impl<'a> SymbolRefTable<'a> {
    pub fn symbol_from_match(&mut self, m: &Match<'a>) -> usize {
        let symbol = SymbolReference::Unknown(m.text);
        self.get_or_create(symbol)
    }

    pub fn symbol_from_vocabid(&mut self, vocabid: VocabId) -> usize {
        let symbol = SymbolReference::Known(vocabid);
        self.get_or_create(symbol)
    }

    pub fn get_or_create(&mut self, symbolref: SymbolReference<'a>) -> usize {
        if symbolref == EPSILON {
            0
        } else if let Some(symbol_index) = self.encoder.get(&symbolref) {
            *symbol_index
        } else {
            self.decoder.push(symbolref);
            self.encoder.insert(symbolref, self.decoder.len()+1);
            self.decoder.len() + 1   //+1 because epsilon is always at 0
        }
    }

    pub fn decode(&self, symbol: usize) -> Option<&SymbolReference<'a>> {
        if symbol == 0 {
            Some(&EPSILON)
        } else {
            self.decoder.get(symbol)
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
        begin = boundary.offset.end;
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



