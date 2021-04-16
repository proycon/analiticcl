use std::collections::VecDeque;
use std::ops::Deref;
use num_bigint::BigUint;
use num_traits::{Zero, One};
use std::iter::{FromIterator,IntoIterator};

use crate::types::*;

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
                            skip = l-1;
                            break 'abciter;
                        }
                    }
                }
            }
            if !matched {
                //Highest one is reserved for UNK
                result.push(alphabet.len() as CharIndexType);
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
    fn iter_deletions(&self, alphabet_size: CharIndexType, max_distance: Option<u32>, breadthfirst: bool) -> RecurseDeletionIterator;
    fn char_count(&self, alphabet_size: CharIndexType) -> u16;
    fn alphabet_upper_bound(&self, alphabet_size: CharIndexType) -> (CharIndexType, u16);
}

impl Anahash for AnaValue {
    /// Computes the Anagram value for the n'th entry in the alphabet
    fn character(seqnr: CharIndexType) -> AnaValue {
        BigUint::from(PRIMES[seqnr as usize])
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
        if self.contains(value) {
            Some(self / value)
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
    ///    assert_eq!(chars.len(), depth as usize);
    /// }
    /// assert_eq!(chars.len(), 5);
    /// assert_eq!(chars.get(0).unwrap(), &"u".anahash(&alphabet));
    /// assert_eq!(chars.get(1).unwrap(), &"s".anahash(&alphabet));
    /// assert_eq!(chars.get(2).unwrap(), &"o".anahash(&alphabet));
    /// assert_eq!(chars.get(3).unwrap(), &"h".anahash(&alphabet));
    /// assert_eq!(chars.get(4).unwrap(), &"e".anahash(&alphabet));
    /// ```
    fn iter(&self, alphabet_size: CharIndexType) -> RecurseDeletionIterator {
        RecurseDeletionIterator::new(self.clone(), alphabet_size, true, None, false)
    }

    /// Iterator over all the parents that are generated when applying all deletions within edit distance 1
    fn iter_parents(&self, alphabet_size: CharIndexType) -> DeletionIterator {
        DeletionIterator::new(self, alphabet_size)
    }

    /// Iterator over all the possible deletions within the specified anagram distance
    fn iter_deletions(&self, alphabet_size: CharIndexType, max_distance: Option<u32>, breadthfirst: bool) -> RecurseDeletionIterator {
        RecurseDeletionIterator::new(self.clone(), alphabet_size, false, max_distance, breadthfirst)
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

///////////////////////////////////////////////////////////////////////////////////////

/// Returns all AnaValues that are formed
/// when doing single deletion. This
/// is the most basic iterator form
/// from which most others are derived.
///
/// The iterator yields values in order
/// of descending alphabet index.
///
/// So given an anagram value for abcd it will yield
/// anagram values abc, abd, acd, bcd
pub struct DeletionIterator<'a> {
    value: &'a AnaValue,
    alphabet_size: CharIndexType,
    iteration: usize,
}

impl<'a> DeletionIterator<'a> {
    pub fn new(value: &'a AnaValue, alphabet_size: CharIndexType) -> DeletionIterator {
        DeletionIterator {
            value: value,
            alphabet_size: alphabet_size,
            iteration: 0
        }
    }
}

#[derive(Clone,Debug)]
pub struct DeletionResult {
    pub value: AnaValue,
    pub charindex: CharIndexType,
}

impl Deref for DeletionResult {
    type Target = AnaValue;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a> Iterator for DeletionIterator<'a> {
    type Item = DeletionResult;

    fn next(&mut self) -> Option<Self::Item> {
        if self.value == &AnaValue::one() || self.iteration == self.alphabet_size as usize {
            None
        } else {
            let charindex: CharIndexType = self.alphabet_size - (self.iteration as u8) - 1;
            self.iteration += 1;
            if let Some(result) = self.value.delete(&AnaValue::character(charindex)) {
                Some(DeletionResult {
                    value: result,
                    charindex: charindex
                })
            } else {
                self.next() //recurse
            }
        }
    }
}


///////////////////////////////////////////////////////////////////////////////////////

pub struct RecurseDeletionIterator {
    queue: VecDeque<(DeletionResult,u32)>, //second tuple argument is the depth at which the iterator starts
    alphabet_size: CharIndexType,
    singlebeam: bool, //caps the queue at every expansion
    breadthfirst: bool,
    maxdepth: Option<u32>, //max depth
}

impl RecurseDeletionIterator {
    pub fn new(value: AnaValue, alphabet_size: CharIndexType, singlebeam: bool, maxdepth: Option<u32>, breadthfirst: bool) -> RecurseDeletionIterator {
        eprintln!("DEBUG NEW");
        let queue: Vec<(DeletionResult,u32)> =  vec!((DeletionResult { value: value, charindex: 0 },0));
        RecurseDeletionIterator {
            queue: VecDeque::from(queue),
            alphabet_size: 0,
            singlebeam: singlebeam,
            breadthfirst: breadthfirst,
            maxdepth: maxdepth,
        }
    }
}


impl Iterator for RecurseDeletionIterator {
    type Item = (DeletionResult,u32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.breadthfirst {
            //breadth first search
            if let Some((node, depth)) = self.queue.pop_front() {
                if self.maxdepth.is_none() || depth < self.maxdepth.expect("get maxdepth") {
                    let iter_children = DeletionIterator::new(&node.value, self.alphabet_size);
                    self.queue.extend(iter_children.map(|child| (child, depth + 1)));
                }

                //don't yield the root element, just recurse in that case
                if depth == 0 {
                    self.next()
                } else {
                    Some((node,depth))
                }
            } else {
                None
            }
        } else {
            //depth first search  (pre-order)
            if let Some((node, depth)) = self.queue.pop_back() {
                if self.maxdepth.is_none() || depth < self.maxdepth.expect("get maxdepth") {
                    let mut iter_children = DeletionIterator::new(&node.value, self.alphabet_size);

                    if self.singlebeam {
                        // single beam, just dive to the bottom in a single line and stop
                        if let Some(child) = iter_children.next() {
                            self.queue.push_back((child, depth + 1));
                        }
                    } else {
                        //reverse the order in which we obtained them
                        let children = iter_children.collect::<Vec<_>>();
                        let children = children.into_iter().rev();

                        self.queue.extend(children.map(|child| (child, depth + 1)));
                    }
                }

                //don't yield the root element, just recurse in that case
                if depth == 0 {
                    self.next()
                } else {
                    Some((node,depth))
                }
            } else {
                None
            }
        }
    }

}

