use std::collections::HashMap;

use crate::types::*;

///A variant in the reverse index
#[derive(Debug)]
pub enum Variant {
    //The variant has an ID only if known in the model
    Known(VocabId),
    Unknown(String),
}


///Links items in the lexicon to variants offered at test time, with a float score
pub type ReverseIndex = HashMap<VocabId,Vec<(Variant,f64)>>;

