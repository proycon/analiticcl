use num_bigint::BigUint;

///Each type gets assigned an ID integer, carries no further meaning
pub type VocabId = u64;

pub type CharIndexType = u8;

pub type CharType = u32;

///A normalized string encoded via the alphabet
pub type NormString = Vec<CharIndexType>;

pub const PRIMES: &[CharType] = &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193, 197, 199, 211, 223, 227, 229, 233, 239, 241, 251, 257, 263, 269, 271, 277, 281, 283, 293, 307, 311, 313, 317, 331, 337, 347, 349, 353, 359, 367, 373, 379, 383, 389, 397, 401, 409, 419, 421, 431, 433, 439, 443, 449, 457, 461, 463, 467, 479, 487, 491, 499, 503, 509, 521, 523, 541, 547, 557, 563, 569, 571, 577, 587, 593, 599, 601, 607, 613, 617, 619, 631, 641, 643, 647, 653, 659, 661, 673, 677, 683, 691, 701, 709, 719, 727, 733, 739, 743, 751, 757, 761, 769, 773, 787, 797, 809, 811, 821, 823, 827, 829, 839, 853, 857, 859, 863, 877, 881, 883, 887, 907, 911, 919, 929, 937, 941, 947, 953, 967, 971, 977, 983, 991, 997];

///The anagram hash: uses a bag-of-characters representation where each bit flags the presence/absence of a certain character (the order of the bits are defined by Alphabet)
pub type AnaValue = BigUint;

///Defines the alphabet, index corresponds how things are encoded, multiple strings may be encoded
///in the same way
pub type Alphabet = Vec<Vec<String>>;

pub struct Weights {
    pub ld: f64,
    pub lcs: f64,
    pub freq: f64,
    pub prefix: f64,
    pub suffix: f64
}

impl Default for Weights {
   fn default() -> Self {
       Self {
           ld: 1.0,
           lcs: 1.0,
           freq: 1.0,
           prefix: 1.0,
           suffix: 1.0
        }
   }
}

impl Weights {
    pub fn sum(&self) -> f64 {
        self.ld + self.lcs + self.freq + self.prefix + self.suffix
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
}
