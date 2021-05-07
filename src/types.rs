use ibig::UBig;
use std::collections::HashMap;
use std::mem;

///Each type gets assigned an ID integer, carries no further meaning
pub type VocabId = u64;

pub type CharIndexType = u8;

pub type CharType = u32;

///A normalized string encoded via the alphabet
pub type NormString = Vec<CharIndexType>;

pub const PRIMES: &[CharType] = &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193, 197, 199, 211, 223, 227, 229, 233, 239, 241, 251, 257, 263, 269, 271, 277, 281, 283, 293, 307, 311, 313, 317, 331, 337, 347, 349, 353, 359, 367, 373, 379, 383, 389, 397, 401, 409, 419, 421, 431, 433, 439, 443, 449, 457, 461, 463, 467, 479, 487, 491, 499, 503, 509, 521, 523, 541, 547, 557, 563, 569, 571, 577, 587, 593, 599, 601, 607, 613, 617, 619, 631, 641, 643, 647, 653, 659, 661, 673, 677, 683, 691, 701, 709, 719, 727, 733, 739, 743, 751, 757, 761, 769, 773, 787, 797, 809, 811, 821, 823, 827, 829, 839, 853, 857, 859, 863, 877, 881, 883, 887, 907, 911, 919, 929, 937, 941, 947, 953, 967, 971, 977, 983, 991, 997];

///The anagram hash: uses a bag-of-characters representation where each bit flags the presence/absence of a certain character (the order of the bits are defined by Alphabet)
pub type AnaValue = UBig;

///Defines the alphabet, index corresponds how things are encoded, multiple strings may be encoded
///in the same way
pub type Alphabet = Vec<Vec<String>>;


///A simple lower-order n-gram type that does not require heap allocation
#[derive(Clone,Hash,PartialEq,Eq,PartialOrd)]
pub enum NGram {
    Empty,
    UniGram(VocabId),
    BiGram(VocabId, VocabId),
    TriGram(VocabId, VocabId, VocabId),
}

impl NGram {
    pub fn from_vec(v: Vec<VocabId>) -> Result<Self, &'static str> {
        match v.len() {
            0 => Ok(NGram::Empty),
            1 => Ok(NGram::UniGram(v[0])),
            2 => Ok(NGram::BiGram(v[0],v[1])),
            3 => Ok(NGram::TriGram(v[0],v[1],v[2])),
            _ => Err("Only supporting unigrams, bigrams and trigrams")
        }
    }

    pub fn new() -> Self {
        NGram::Empty
    }

    pub fn len(&self) -> u8 {
        match self {
            NGram::Empty => 0,
            NGram::UniGram(_) => 1,
            NGram::BiGram(..) => 2,
            NGram::TriGram(..) => 3,
        }
    }

    pub fn push(&mut self, item: VocabId) -> bool {
        match *self {
            NGram::Empty => {
                mem::replace(self, NGram::UniGram(item));
                true
            },
            NGram::UniGram(x) => {
                mem::replace(self, NGram::BiGram(x, item));
                true
            },
            NGram::BiGram(x,y) => {
                mem::replace(self, NGram::TriGram(x,y, item));
                true
            }
            _ => false
        }
    }
}

pub struct Weights {
    pub ld: f64,
    pub lcs: f64,
    pub freq: f64,
    pub prefix: f64,
    pub suffix: f64,
    pub lex: f64,
    pub case: f64,
}

impl Default for Weights {
   fn default() -> Self {
       Self {
           ld: 1.0,
           lcs: 1.0,
           freq: 1.0,
           prefix: 1.0,
           suffix: 1.0,
           lex: 1.0,
           case: 0.2,
        }
   }
}

impl Weights {
    pub fn sum(&self) -> f64 {
        self.ld + self.lcs + self.freq + self.prefix + self.suffix + self.lex + self.case
    }
}


#[derive(Debug,Clone)]
pub struct Distance {
    ///Levenshtein (or Damarau-Levenshtein) distance
    pub ld: CharIndexType,

    ///Longest common substring length
    pub lcs: u16,

    ///Common prefix length
    pub prefixlen: u16,

    ///Common suffix length
    pub suffixlen: u16,

    ///Absolute frequency count
    pub freq: u32,

    ///Lexicon weight (usually simply 1.0 if an item is in a validated lexicon, and 0.0 if in a
    ///background corpus)
    pub lex: f32,

    ///Is the casing different or not?
    pub samecase: bool,

    ///Some variants may be pre-scored already if they were found in an explicit variant list, the
    ///prescore component will be as a component in computing the final csore
    pub prescore: Option<f64>,
}

#[derive(Debug,Clone,Copy)]
pub enum StopCriterion {
    Exhaustive,
    StopAtExactMatch,
    Iterative(usize),
    IterativeStopAtExactMatch(usize),
}

impl StopCriterion {
    pub fn stop_at_exact_match(&self) -> bool {
        match self {
            Self::StopAtExactMatch | Self::IterativeStopAtExactMatch(_) => true,
            _ => false
        }
    }

    pub fn iterative(&self) -> usize {
        match self {
            Self::Iterative(matches) | Self::IterativeStopAtExactMatch(matches) => *matches,
            _ => 0
        }
    }
}

pub type VariantClusterId = u32;

#[derive(Debug,Clone,PartialEq,PartialOrd)]
pub enum VariantReference {
    VariantCluster(VariantClusterId),
    WeightedVariant((VocabId, f64))
}

pub type VariantClusterMap = HashMap<VariantClusterId, Vec<VocabId>>;

