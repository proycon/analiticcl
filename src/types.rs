use ibig::UBig;
use std::collections::HashMap;

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



#[derive(Clone,PartialEq,Debug)]
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

#[derive(Clone,Debug)]
pub struct SearchParameters {
    /// Maximum edit distance (levenshtein-damarau). The maximum edit distance according to Levenshtein-Damarau. Insertions, deletions, substitutions and transposition all have the same cost (1). It is recommended to set this value slightly lower than the maximum anagram distance
    pub max_anagram_distance: u8,

    /// Maximum edit distance (levenshtein-damarau). The maximum edit distance according to Levenshtein-Damarau. Insertions, deletions, substitutions and transposition all have the same cost (1). It is recommended to set this value slightly lower than the maximum anagram distance.
    pub max_edit_distance: u8,

    /// Number of matches to return per input (set to 0 for unlimited if you want to exhaustively return every possibility within the specified anagram and edit distance)
    pub max_matches: usize,

    /// Require scores to meet this threshold, they are pruned otherwise
    pub score_threshold: f64,

    /// Cut-off threshold: if a score in the ranking is a specific factor greater than the best score, the ranking will be cut-off at that point and the score not included. Should be set to a value like 2.
    pub cutoff_threshold: f64,

    /// Determines when to stop searching for matches. Setting this can speed up the process at the
    /// cost of lower accuracy
    pub stop_criterion: StopCriterion,

    /// Maximum ngram order (1 for unigrams, 2 for bigrams, etc..). This also requires you to load actual ngram frequency lists to have any effect.
    pub max_ngram: u8,

    /// Maximum number of candidate sequences to take along to the language modelling stage
    pub max_seq: usize,

    /// Use only a single-thread instead of leveraging multiple cores (lowers resource use and
    /// performance)
    pub single_thread: bool,

    /// Weight attributed to the language model
    pub lm_weight: f32,
    /// Weight attributed to the variant model
    pub variantmodel_weight: f32
}

impl Default for SearchParameters {
    fn default() -> Self {
        Self {
            max_anagram_distance: 3,
            max_edit_distance: 3,
            max_matches: 20,
            score_threshold: 0.25,
            cutoff_threshold: 2.0,
            stop_criterion: StopCriterion::Exhaustive,
            max_ngram: 2,
            single_thread: false,
            max_seq: 250,
            lm_weight: 1.0,
            variantmodel_weight: 1.0,
        }
    }
}

impl SearchParameters {
    pub fn with_edit_distance(mut self, distance: u8) -> Self {
        self.max_edit_distance = distance;
        self
    }
    pub fn with_anagram_distance(mut self, distance: u8) -> Self {
        self.max_anagram_distance = distance;
        self
    }
    pub fn with_max_matches(mut self, matches: usize) -> Self {
        self.max_matches = matches;
        self
    }
    pub fn with_score_threshold(mut self, threshold: f64) -> Self {
        self.score_threshold = threshold;
        self
    }
    pub fn with_cutoff_threshold(mut self, threshold: f64) -> Self {
        self.cutoff_threshold = threshold;
        self
    }
    pub fn with_stop_criterion(mut self, criterion: StopCriterion) -> Self {
        self.stop_criterion = criterion;
        self
    }
    pub fn with_max_ngram(mut self, max_ngram: u8) -> Self {
        self.max_ngram = max_ngram;
        self
    }
    pub fn with_max_seq(mut self, max_seq: usize) -> Self {
        self.max_seq = max_seq;
        self
    }
    pub fn with_single_thread(mut self) -> Self {
        self.single_thread = true;
        self
    }
    pub fn with_lm_weight(mut self, weight: f32) -> Self {
        self.lm_weight = weight;
        self
    }
    pub fn with_variantmodel_weight(mut self, weight: f32) -> Self {
        self.variantmodel_weight = weight;
        self
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

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum StopCriterion {
    Exhaustive,

    /// Stop when we find an exact match with a lexical weight higher or equal than the specified value here
    StopAtExactMatch(f32),
}

impl StopCriterion {
    pub fn stop_at_exact_match(&self) -> bool {
        match self {
            Self::StopAtExactMatch(_)  => true,
            _ => false
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

///A simple lower-order n-gram type that does not require heap allocation
#[derive(Clone,Hash,PartialEq,Eq,PartialOrd)]
pub enum NGram {
    Empty,
    UniGram(VocabId),
    BiGram(VocabId, VocabId),
    TriGram(VocabId, VocabId, VocabId),
}

impl NGram {
    pub fn from_list(v: &[VocabId]) -> Result<Self, &'static str> {
        match v.len() {
            0 => Ok(NGram::Empty),
            1 => Ok(NGram::UniGram(v[0])),
            2 => Ok(NGram::BiGram(v[0],v[1])),
            3 => Ok(NGram::TriGram(v[0],v[1],v[2])),
            _ => Err("Only supporting unigrams, bigrams and trigrams")
        }
    }

    pub fn from_option_list(v: &[Option<VocabId>]) -> Result<Self, &'static str> {
        match v {
            [] => Ok(NGram::Empty),
            [Some(a)] => Ok(NGram::UniGram(*a)),
            [Some(a),Some(b)] => Ok(NGram::BiGram(*a,*b)),
            [Some(a),Some(b),Some(c)] => Ok(NGram::TriGram(*a,*b,*c)),
            _ => Err("Only supporting unigrams, bigrams and trigrams")
        }
    }

    pub fn to_vec(&self) -> Vec<VocabId> {
        match *self {
            NGram::Empty => {
                vec!()
            },
            NGram::UniGram(x) => {
                vec!(x)
            },
            NGram::BiGram(x,y) => {
                vec!(x,y)
            },
            NGram::TriGram(x,y,z) => {
                vec!(x,y,z)
            }
        }
    }

    pub fn new() -> Self {
        NGram::Empty
    }

    pub fn len(&self) -> usize {
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
                *self = NGram::UniGram(item);
                true
            },
            NGram::UniGram(x) => {
                *self = NGram::BiGram(x, item);
                true
            },
            NGram::BiGram(x,y) => {
                *self = NGram::TriGram(x,y, item);
                true
            }
            _ => false
        }
    }

    pub fn first(&self) -> Option<VocabId> {
        match *self {
            NGram::Empty => {
                None
            },
            NGram::UniGram(x) | NGram::BiGram(x,_) | NGram::TriGram(x,_,_) => {
                Some(x)
            }
        }
    }

    pub fn pop_first(&mut self) -> NGram {
        match *self {
            NGram::Empty => {
                NGram::Empty
            },
            NGram::UniGram(x) => {
                *self = NGram::Empty;
                NGram::UniGram(x)
            },
            NGram::BiGram(x,y) => {
                *self = NGram::UniGram(y);
                NGram::UniGram(x)
            }
            NGram::TriGram(x,y,z) => {
                *self = NGram::BiGram(y,z);
                NGram::UniGram(x)
            }
        }
    }

    pub fn pop_last(&mut self) -> NGram {
        match *self {
            NGram::Empty => {
                NGram::Empty
            },
            NGram::UniGram(x) => {
                *self = NGram::Empty;
                NGram::UniGram(x)
            },
            NGram::BiGram(x,y) => {
                *self = NGram::UniGram(x);
                NGram::UniGram(y)
            }
            NGram::TriGram(x,y,z) => {
                *self = NGram::BiGram(x,y);
                NGram::UniGram(z)
            }
        }
    }
}
