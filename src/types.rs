use ibig::UBig;
use std::collections::HashMap;
use std::str::FromStr;
use std::io::Error;
use std::io::ErrorKind;
use std::fmt;

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
    pub prefix: f64,
    pub suffix: f64,
    pub case: f64,
}

impl Default for Weights {
   fn default() -> Self {
       Self {
           ld: 0.5,
           lcs: 0.125,
           prefix: 0.125,
           suffix: 0.125,
           case: 0.125,
        }
   }
}

impl Weights {
    pub fn sum(&self) -> f64 {
        self.ld + self.lcs + self.prefix + self.suffix + self.case
    }
}

#[derive(Clone,Copy,Debug)]
pub enum DistanceThreshold {
    ///The distance threshold is expressed as a ratio of the total length of the text fragment under consideration, should be in range 0-1
    Ratio(f32),
    ///Absolute distance threshold
    Absolute(u8)
}

impl FromStr for DistanceThreshold {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, std::io::Error> {
        if let Ok(num) = s.parse::<u8>() {
            return Ok(Self::Absolute(num));
        } else if let Ok(num) = s.parse::<f32>() {
            if num >= 0.0 && num <= 1.0 {
                return Ok(Self::Ratio(num))
            }
        }
        Err(Error::new(ErrorKind::InvalidInput, "Input must be integer (absolute threshold) or float between 0.0 and 1.0 (ratio)"))
    }
}


#[derive(Clone,Debug)]
pub struct SearchParameters {
    /// Maximum anagram distance. The difference in characters (regardless of order)
    pub max_anagram_distance: DistanceThreshold,

    /// Maximum edit distance (levenshtein-damarau). The maximum edit distance according to Levenshtein-Damarau. Insertions, deletions, substitutions and transposition all have the same cost (1). It is recommended to set this value slightly lower than the maximum anagram distance.
    pub max_edit_distance: DistanceThreshold,

    /// Number of matches to return per input (set to 0 for unlimited if you want to exhaustively return every possibility within the specified anagram and edit distance)
    pub max_matches: usize,

    /// Require scores to meet this threshold, they are pruned otherwise
    pub score_threshold: f64,

    /// Cut-off threshold: if a score in the ranking is a specific factor greater than the best score, the ranking will be cut-off at that point and the score not included. Should be set to a value like 2.
    pub cutoff_threshold: f64,

    /// Determines when to stop searching for matches. Setting this can speed up the process at the
    /// cost of lower accuracy
    pub stop_criterion: StopCriterion,

    /// Maximum ngram order (1 for unigrams, 2 for bigrams, etc..).
    pub max_ngram: u8,

    /// Maximum ngram order for Language Models (2 for bigrams, etc..).
    pub lm_order: u8,

    /// Maximum number of candidate sequences to take along to the language modelling stage
    pub max_seq: usize,

    /// Use only a single-thread instead of leveraging multiple cores (lowers resource use and
    /// performance)
    pub single_thread: bool,

    /// Weight attributed to the language model in relation to the variant model (e.g. 2.0 = twice
    /// as much weight) when considering input context and rescoring.
    pub context_weight: f32,

    /// Weight attributed to the language model in finding the most likely sequence
    pub lm_weight: f32,

    /// Weight attributed to the frequency information in frequency reranking, in relation to
    /// the similarity component. 0 = disabled)
    pub freq_weight: f32,

    /// Weight attributed to the variant model in finding the most likely sequence
    pub variantmodel_weight: f32,

    /// Consolidate matches and extract a single most likely sequence, if set
    /// to false, all possible matches (including overlapping ones) are returned.
    pub consolidate_matches: bool
}

impl Default for SearchParameters {
    fn default() -> Self {
        Self {
            max_anagram_distance: DistanceThreshold::Absolute(3),
            max_edit_distance: DistanceThreshold::Absolute(3),
            max_matches: 20,
            score_threshold: 0.25,
            cutoff_threshold: 2.0,
            stop_criterion: StopCriterion::Exhaustive,
            max_ngram: 3,
            lm_order: 3,
            single_thread: false,
            max_seq: 250,
            context_weight: 0.0,
            lm_weight: 1.0,
            freq_weight: 0.0,
            variantmodel_weight: 1.0,
            consolidate_matches: true,
        }
    }
}

impl fmt::Display for SearchParameters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f," max_anagram_distance={:?}",self.max_anagram_distance);
        writeln!(f," max_edit_distance={:?}",self.max_edit_distance);
        writeln!(f," max_matches={}",self.max_matches);
        writeln!(f," score_threshold={}",self.score_threshold);
        writeln!(f," cutoff_threshold={}",self.cutoff_threshold);
        writeln!(f," max_ngram={}",self.max_ngram);
        writeln!(f," lm_order={}",self.lm_order);
        writeln!(f," single_thread={}",self.single_thread);
        writeln!(f," max_seq={}",self.max_seq);
        writeln!(f," freq_weight={}",self.freq_weight);
        writeln!(f," lm_weight={}",self.lm_weight);
        writeln!(f," variantmodel_weight={}",self.variantmodel_weight);
        writeln!(f," consolidate_matches={}",self.consolidate_matches)
    }
}

impl SearchParameters {
    pub fn with_edit_distance(mut self, distance: DistanceThreshold) -> Self {
        self.max_edit_distance = distance;
        self
    }
    pub fn with_anagram_distance(mut self, distance: DistanceThreshold) -> Self {
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
    pub fn with_context_weight(mut self, weight: f32) -> Self {
        self.context_weight = weight;
        self
    }
    pub fn with_lm_weight(mut self, weight: f32) -> Self {
        self.lm_weight = weight;
        self
    }
    pub fn with_lm_order(mut self, order: u8) -> Self {
        self.lm_order = order;
        self
    }
    pub fn with_variantmodel_weight(mut self, weight: f32) -> Self {
        self.variantmodel_weight = weight;
        self
    }
    pub fn with_consolidate_matches(mut self, value: bool) -> Self {
        self.consolidate_matches = value;
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
    StopAtExactMatch,
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
    QuadGram(VocabId, VocabId, VocabId, VocabId),
    QuintGram(VocabId, VocabId, VocabId, VocabId, VocabId),
}

impl NGram {
    pub fn from_list(v: &[VocabId]) -> Result<Self, &'static str> {
        match v.len() {
            0 => Ok(NGram::Empty),
            1 => Ok(NGram::UniGram(v[0])),
            2 => Ok(NGram::BiGram(v[0],v[1])),
            3 => Ok(NGram::TriGram(v[0],v[1],v[2])),
            4 => Ok(NGram::QuadGram(v[0],v[1],v[2],v[3])),
            5 => Ok(NGram::QuintGram(v[0],v[1],v[2],v[3],v[4])),
            _ => Err("Only supporting at most 5-grams")
        }
    }

    pub fn from_option_list(v: &[Option<VocabId>]) -> Result<Self, &'static str> {
        match v {
            [] => Ok(NGram::Empty),
            [Some(a)] => Ok(NGram::UniGram(*a)),
            [Some(a),Some(b)] => Ok(NGram::BiGram(*a,*b)),
            [Some(a),Some(b),Some(c)] => Ok(NGram::TriGram(*a,*b,*c)),
            [Some(a),Some(b),Some(c),Some(d)] => Ok(NGram::QuadGram(*a,*b,*c,*d)),
            [Some(a),Some(b),Some(c),Some(d),Some(e)] => Ok(NGram::QuintGram(*a,*b,*c,*d,*e)),
            _ => Err("Only supporting at most 5-grams")
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
            },
            NGram::QuadGram(a,b,c,d) => {
                vec!(a,b,c,d)
            },
            NGram::QuintGram(a,b,c,d,e) => {
                vec!(a,b,c,d,e)
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
            NGram::QuadGram(..) => 4,
            NGram::QuintGram(..) => 5,
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
            },
            NGram::TriGram(x,y,z) => {
                *self = NGram::QuadGram(x,y,z, item);
                true
            },
            NGram::QuadGram(a,b,c,d) => {
                *self = NGram::QuintGram(a,b,c,d, item);
                true
            },
            _ => false
        }
    }

    pub fn first(&self) -> Option<VocabId> {
        match *self {
            NGram::Empty => {
                None
            },
            NGram::UniGram(x) | NGram::BiGram(x,_) | NGram::TriGram(x,_,_) | NGram::QuadGram(x,_,_,_) | NGram::QuintGram(x,_,_,_,_) => {
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
            },
            NGram::TriGram(x,y,z) => {
                *self = NGram::BiGram(y,z);
                NGram::UniGram(x)
            },
            NGram::QuadGram(a,b,c,d) => {
                *self = NGram::TriGram(b,c,d);
                NGram::UniGram(a)
            },
            NGram::QuintGram(a,b,c,d,e) => {
                *self = NGram::QuadGram(b,c,d,e);
                NGram::UniGram(a)
            },
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
            },
            NGram::TriGram(x,y,z) => {
                *self = NGram::BiGram(x,y);
                NGram::UniGram(z)
            },
            NGram::QuadGram(a,b,c,d) => {
                *self = NGram::TriGram(a,b,c);
                NGram::UniGram(d)
            },
            NGram::QuintGram(a,b,c,d,e) => {
                *self = NGram::QuadGram(a,b,c,d);
                NGram::UniGram(e)
            },
        }
    }
}
