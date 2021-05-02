use std::collections::HashMap;

use crate::types::*;

#[derive(Clone,Debug)]
pub struct VocabValue {
    pub text: String,

    /// A version of the text normalized to the alphabet
    pub norm: NormString,

    /// The absolute frequency count
    pub frequency: u32,

    /// The number of words
    pub tokencount: u8,

    /// The weight assigned by the lexicon as a whole
    /// (usually 1.0 for validated lexicons and 0.0 for backgrond corpora)
    pub lexweight: f32,

    /// The first lexicon index which matches
    pub lexindex: u8,

    /// Pointer to other vocabulary items that are considered a variant
    /// of this one (with a certain score between 0 and 1). This structure is used when loading variant/error lists
    /// and not in normal operation.
    pub variants: Option<Vec<VariantReference>>,

    /// Marks this entry as intermediate; intermediate entries will only be used to find further explicitly provided variants
    /// and will never be returned as a solution by itself. For example, all erroneous variants in
    /// an errorlist are marked as intermediate.
    pub intermediate: bool,
}

///Map integers (indices correspond to VocabId) to string values (and optionally a frequency count)
pub type VocabDecoder = Vec<VocabValue>;

///Maps strings to integers
pub type VocabEncoder = HashMap<String, VocabId>;

pub struct VocabParams {
    ///Column containing the Text (if any, 0-indexed)
    pub text_column: u8,
    ///Column containing the absolute frequency (if any, 0-indexed)
    pub freq_column: Option<u8>
}

impl Default for VocabParams {
    fn default() -> Self {
        Self {
            text_column: 0,
            freq_column: None,
        }
    }
}

