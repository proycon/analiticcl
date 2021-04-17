use std::collections::VecDeque;
use std::ops::Deref;
use num_bigint::BigUint;
use num_traits::{Zero, One};
use std::iter::{FromIterator,IntoIterator};

use crate::types::*;
use crate::anahash::*;

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

