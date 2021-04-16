use std::collections::HashMap;

use crate::types::*;
use crate::vocab::*;

pub type AnaIndex = HashMap<AnaValue,AnaIndexNode>;

#[derive(Default)]
pub struct AnaIndexNode {
    ///Maps an anagram value to all existing instances that instantiate it
    pub instances: Vec<VocabId>,
    pub charcount: u16
}

