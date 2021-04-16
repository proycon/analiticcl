use std::collections::HashMap;

use crate::types::*;

#[derive(Clone)]
pub struct VocabValue {
    pub text: String,
    pub norm: NormString,
    pub frequency: u32,
    ///The number of words
    pub tokencount: u8,
}

///Map integers (indices correspond to VocabId) to string values (and optionally a frequency count)
pub type VocabDecoder = Vec<VocabValue>;

///Maps strings to integers
pub type VocabEncoder = HashMap<String, VocabId>; //TODO: obsolete?

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
