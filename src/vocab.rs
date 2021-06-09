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

    pub vocabtype: VocabType,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum VocabType {
    /// A normal vocabulary entry
    Normal,

    /// Marks this entry as intermediate; intermediate entries will only be used to find further explicitly provided variants
    /// and will never be returned as a solution by itself. For example, all erroneous variants in
    /// an errorlist are marked as intermediate.
    Intermediate,

    /// Reserved for items that will not be added to the index at all
    /// for language-model entries and for special tokens like BeginOfSentence, EndOfSentence
    NoIndex,
}

impl VocabValue {
    pub fn new(text: String, vocabtype: VocabType) -> Self {
        let tokencount = text.chars().filter(|c| *c == ' ').count() as u8;
        VocabValue {
            text: text,
            norm: Vec::new(),
            frequency: 1, //smoothing
            tokencount,
            lexweight: 0.0,
            lexindex: 0,
            variants: None,
            vocabtype,
        }
    }
}


///Map integers (indices correspond to VocabId) to string values (and optionally a frequency count)
pub type VocabDecoder = Vec<VocabValue>;

///Maps strings to integers
pub type VocabEncoder = HashMap<String, VocabId>;

///Frequency handling in case of duplicate items (may be across multiple lexicons), the
///``*IfMoreWeight`` variants only apply when the item to be added has a higher lexicon weight
///associated with it.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum FrequencyHandling {
    Sum,
    Max,
    Min,
    Replace,
    SumIfMoreWeight,
    MaxIfMoreWeight,
    MinIfMoreWeight,
    ReplaceIfMoreWeight,
}

#[derive(Clone,Debug)]
pub struct VocabParams {
    ///Column containing the Text (if any, 0-indexed)
    pub text_column: u8,
    ///Column containing the absolute frequency (if any, 0-indexed)
    pub freq_column: Option<u8>,
    ///Frequency handling in case of duplicate items (may be across multiple lexicons)
    pub freq_handling: FrequencyHandling,
    pub vocab_type: VocabType,
    /// Lexicon weight
    pub weight: f32,
    /// Lexicon index
    pub index: u8,
}

impl Default for VocabParams {
    fn default() -> Self {
        Self {
            text_column: 0,
            freq_column: Some(1),
            freq_handling: FrequencyHandling::Max,
            vocab_type: VocabType::Normal,
            weight: 1.0,
            index: 0,
        }
    }
}

impl VocabParams {
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }
    pub fn with_vocab_type(mut self, vocab_type: VocabType) -> Self {
        self.vocab_type = vocab_type;
        self
    }
}

pub const BOS: VocabId = 0;
pub const EOS: VocabId = 1;
pub const UNK: VocabId = 2;

/// Adds some initial special tokens, required for basic language modelling in the 'search' stage
pub(crate) fn init_vocab(decoder: &mut VocabDecoder, encoder: &mut HashMap<String, VocabId>) {
    decoder.push(VocabValue {
        text: "<bos>".to_string(),
        norm: vec!(),
        frequency: 0,
        tokencount: 1,
        lexweight: 0.0,
        lexindex: 0,
        variants: None,
        vocabtype: VocabType::NoIndex,
    });
    decoder.push(VocabValue {
        text: "<eos>".to_string(),
        norm: vec!(),
        frequency: 0,
        tokencount: 1,
        lexweight: 0.0,
        lexindex: 0,
        variants: None,
        vocabtype: VocabType::NoIndex,
    });
    decoder.push(VocabValue {
        text: "<unk>".to_string(),
        norm: vec!(),
        frequency: 0,
        tokencount: 1,
        lexweight: 0.0,
        lexindex: 0,
        variants: None,
        vocabtype: VocabType::NoIndex,
    });
    encoder.insert("<bos>".to_string(),BOS);
    encoder.insert("<eos>".to_string(),EOS);
    encoder.insert("<unk>".to_string(),UNK);
}
