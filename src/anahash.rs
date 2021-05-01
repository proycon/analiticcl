use ibig::UBig;
use ibig::ops::DivRem;
use num_traits::{Zero, One};
use std::collections::HashSet;

use crate::types::*;
use crate::iterators::*;

///Trait for objects that can be anahashed (string-like)
pub trait Anahashable {
    fn anahash(&self, alphabet: &Alphabet) -> AnaValue;
    fn normalize_to_alphabet(&self, alphabet: &Alphabet) -> NormString;
}

impl Anahashable for str {
    ///Compute the anahash for a given string, according to the alphabet
    fn anahash(&self, alphabet: &Alphabet) -> AnaValue {
        let mut hash: AnaValue = AnaValue::empty();
        let mut skip = 0;
        for (pos, _) in self.char_indices() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let mut matched = false;
            'abciter: for (seqnr, chars) in alphabet.iter().enumerate() {
                for element in chars.iter() {
                    let l = element.chars().count();
                    if let Some(slice) = self.get(pos..pos+l) {
                        if slice == element {
                            let charvalue = AnaValue::character(seqnr as CharIndexType);
                            hash = hash.insert(&charvalue);
                            matched = true;
                            skip = l-1;
                            break 'abciter;
                        }
                    }
                }
            }
            if !matched {
                //Highest one is reserved for UNK
                let charvalue = AnaValue::character(alphabet.len() as CharIndexType);
                hash = hash.insert(&charvalue);
            }
        }
        hash
    }


    ///Normalize a string via the alphabet
    fn normalize_to_alphabet(&self, alphabet: &Alphabet) -> NormString {
        let mut result = Vec::with_capacity(self.chars().count());
        let mut skip = 0;
        for (pos, _) in self.char_indices() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            //does greedy matching in order of appearance in the alphabet file
            let mut matched = false;
            'abciter: for (i, chars) in alphabet.iter().enumerate() {
                for element in chars.iter() {
                    let l = element.chars().count();
                    if let Some(slice) = self.get(pos..pos+l) {
                        if slice == element {
                            result.push(i as CharIndexType);
                            matched = true;
                            skip = l-1;
                            break 'abciter;
                        }
                    }
                }
            }
            if !matched {
                //Highest one is reserved for UNK
                result.push(alphabet.len() as CharIndexType + 1);
            }
        }
        result
    }

}


/// This trait can be applied to types
/// that can function as anahashes.
/// It can be implemented  for integer types.
pub trait Anahash: One + Zero {
    fn character(seqnr: CharIndexType) -> AnaValue;
    fn empty() -> AnaValue;
    fn is_empty(&self) -> bool;
    fn insert(&self, value: &AnaValue) -> AnaValue;
    fn delete(&self, value: &AnaValue) -> Option<AnaValue>;
    fn contains(&self, value: &AnaValue) -> bool;
    fn iter(&self, alphabet_size: CharIndexType) -> RecurseDeletionIterator;
    fn iter_parents(&self, alphabet_size: CharIndexType) -> DeletionIterator;
    fn iter_recursive(&self, alphabet_size: CharIndexType, params: &SearchParams) -> RecurseDeletionIterator;
    fn iter_recursive_external_cache<'a>(&self, alphabet_size: CharIndexType, params: &SearchParams, cache: &'a mut HashSet<AnaValue>) -> RecurseDeletionIterator<'a>;
    fn char_count(&self, alphabet_size: CharIndexType) -> u16;
    fn alphabet_upper_bound(&self, alphabet_size: CharIndexType) -> (CharIndexType, u16);
}

impl Anahash for AnaValue {
    /// Computes the Anagram value for the n'th entry in the alphabet
    fn character(seqnr: CharIndexType) -> AnaValue {
        UBig::from(PRIMES[seqnr as usize])
    }

    /// Insert the characters represented by the anagram value, returning the result
    fn insert(&self, value: &AnaValue) -> AnaValue {
        if self == &AnaValue::zero() {
            value.clone()
        } else {
            self * value
        }
    }

    /// Delete the characters represented by the anagram value, returning the result
    /// Returns None of the anagram was not found
    fn delete(&self, value: &AnaValue) -> Option<AnaValue> {
        let (result, remainder) = self.div_rem(value);
        if remainder == AnaValue::zero() {
            Some(result)
        } else {
            None
        }
    }

    /// Tests if the anagram value contains the specified anagram value
    fn contains(&self, value: &AnaValue) -> bool {
        if value > self {
            false
        } else {
            (self % value) == AnaValue::zero()
        }
    }

    /// Iterate over all characters in this alphabet
    /// Returns DeletionResult instances that holds
    /// a `charindex` attribute indicating the index
    /// in the alphabet. If there are duplicates,
    /// this iterator returns them all.
    ///
    /// # Examples
    ///
    /// ```
    /// # use analiticcl::*;
    /// # use analiticcl::test::*;
    /// # use std::ops::Deref;
    /// # let (alphabet, alphabet_size) = get_test_alphabet();
    /// let anavalue: AnaValue = "house".anahash(&alphabet);
    /// let mut chars: Vec<AnaValue> = Vec::new();
    /// for (deletion, depth) in anavalue.iter(alphabet_size) {
    ///    chars.push(AnaValue::character(deletion.charindex));
    /// }
    /// ```
    fn iter(&self, alphabet_size: CharIndexType) -> RecurseDeletionIterator {
        RecurseDeletionIterator::new(self.clone(), alphabet_size, true, None, None, false,false,true, None)
    }

    /// Iterator over all the parents that are generated when applying all deletions within edit distance 1
    fn iter_parents(&self, alphabet_size: CharIndexType) -> DeletionIterator {
        DeletionIterator::new(self, alphabet_size)
    }

    /// Iterator over all the possible deletions within the specified anagram distance
    fn iter_recursive(&self, alphabet_size: CharIndexType, params: &SearchParams) -> RecurseDeletionIterator {
        RecurseDeletionIterator::new(self.clone(), alphabet_size, false, params.min_distance, params.max_distance, params.breadthfirst, !params.allow_duplicates, params.allow_empty_leaves, None )
    }

    /// Iterator over all the possible deletions within the specified anagram distance
    fn iter_recursive_external_cache<'a>(&self, alphabet_size: CharIndexType, params: &SearchParams, cache: &'a mut HashSet<AnaValue>) -> RecurseDeletionIterator<'a> {
        RecurseDeletionIterator::new(self.clone(), alphabet_size, false, params.min_distance, params.max_distance, params.breadthfirst, !params.allow_duplicates, params.allow_empty_leaves, Some(cache) )
    }

    /// The value of an empty anahash
    /// Also corresponds to the root of the index
    fn empty() -> AnaValue {
        AnaValue::one()
    }

    /// The value of an empty anahash
    /// Also corresponds to the root of the index
    fn is_empty(&self) -> bool {
        self == &AnaValue::empty() || self == &AnaValue::zero()
    }

    /// Computes the number of characters in this anagram
    fn char_count(&self, alphabet_size: CharIndexType) -> u16 {
        self.iter(alphabet_size).count() as u16
    }


    /// Returns the the upper bound of the alphabet size
    /// as used in this anavalue, which may be lower
    /// than the actual alphabet size.
    /// Returns a character index in the alphabet,
    /// also returns the character count as 2nd member of the tuple
    fn alphabet_upper_bound(&self, alphabet_size: CharIndexType) -> (CharIndexType, u16) {
        let mut maxcharindex = 0;
        let mut count = 0;
        for (result, _) in self.iter(alphabet_size) {
            count += 1;
            if result.charindex > maxcharindex {
                maxcharindex = result.charindex;
            }
        }
        (maxcharindex, count)
    }

}



/// Search parameters used to pass to the Anahash::iter_recursive() function
pub struct SearchParams {
    pub min_distance: Option<u32>,
    pub max_distance: Option<u32>,
    pub breadthfirst: bool,
    pub allow_duplicates: bool,
    pub allow_empty_leaves: bool,
}

impl Default for SearchParams {
    fn default() -> Self {
        SearchParams {
            min_distance: None,
            max_distance: None,
            breadthfirst: false,
            allow_duplicates: true,
            allow_empty_leaves: true,
        }
    }
}

