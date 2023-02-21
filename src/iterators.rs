use num_traits::One;
use std::collections::{HashSet, VecDeque};
use std::iter::IntoIterator;
use std::ops::Deref;

use crate::anahash::*;
use crate::types::*;

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
            iteration: 0,
        }
    }
}

#[derive(Clone, Debug)]
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
                    charindex: charindex,
                })
            } else {
                self.next() //recurse
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////

pub enum VisitedMap<'a> {
    Internal(HashSet<AnaValue>),
    External(&'a mut HashSet<AnaValue>),
}

impl VisitedMap<'_> {
    pub fn contains(&self, key: &AnaValue) -> bool {
        match self {
            VisitedMap::Internal(map) => map.contains(key),
            VisitedMap::External(map) => map.contains(key),
        }
    }

    pub fn insert(&mut self, key: AnaValue) -> bool {
        match self {
            VisitedMap::Internal(map) => map.insert(key),
            VisitedMap::External(map) => map.insert(key),
        }
    }
}

pub struct RecurseDeletionIterator<'a> {
    queue: VecDeque<(DeletionResult, u32)>, //second tuple argument is the depth at which the iterator starts
    alphabet_size: CharIndexType,
    singlebeam: bool, //caps the queue at every expansion
    breadthfirst: bool,
    mindepth: u32,
    maxdepth: Option<u32>, //max depth

    ///Allow returning empty leaves at the maximum depth of the search (needed if you want to
    ///inspect the charindex)
    empty_leaves: bool,

    ///Ensure all returned items are unique, no duplicates are yielded
    unique: bool,

    ///Used to keep track of visited values if unique is set
    visited: VisitedMap<'a>,
}

impl<'a> RecurseDeletionIterator<'a> {
    pub fn new(
        value: AnaValue,
        alphabet_size: CharIndexType,
        singlebeam: bool,
        mindepth: Option<u32>,
        maxdepth: Option<u32>,
        breadthfirst: bool,
        unique: bool,
        empty_leaves: bool,
        external_visited_map: Option<&'a mut HashSet<AnaValue>>,
    ) -> RecurseDeletionIterator<'a> {
        let queue: Vec<(DeletionResult, u32)> = vec![(
            DeletionResult {
                value: value,
                charindex: 0,
            },
            0,
        )];
        RecurseDeletionIterator {
            queue: VecDeque::from(queue),
            alphabet_size: alphabet_size,
            singlebeam: singlebeam,
            breadthfirst: breadthfirst,
            mindepth: mindepth.unwrap_or(1),
            maxdepth: maxdepth,
            unique: unique,
            empty_leaves: empty_leaves,
            visited: match external_visited_map {
                Some(mapref) => VisitedMap::External(mapref),
                None => VisitedMap::Internal(HashSet::new()),
            },
        }
    }
}

impl Iterator for RecurseDeletionIterator<'_> {
    type Item = (DeletionResult, u32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.breadthfirst {
            //------------------ breadth first search --------------------
            if let Some((node, depth)) = self.queue.pop_front() {
                if self.unique && self.visited.contains(&node.value) {
                    return self.next(); //node was already visited, recurse to next
                }

                if self.maxdepth.is_none() || depth < self.maxdepth.expect("get maxdepth") {
                    let iter_children = DeletionIterator::new(&node.value, self.alphabet_size);
                    if self.unique {
                        let visited = &self.visited; //borrow outside closure otherwise borrow checker gets confused
                        self.queue.extend(
                            iter_children
                                .filter(|child| !visited.contains(&child.value))
                                .map(|child| (child, depth + 1)),
                        );
                    } else {
                        self.queue
                            .extend(iter_children.map(|child| (child, depth + 1)));
                    }
                }

                //don't yield the root element (or empty leaves if we don't want them), just recurse in that case
                if (depth < self.mindepth) || (!self.empty_leaves && node.value.is_empty()) {
                    self.next()
                } else {
                    if self.unique {
                        self.visited.insert(node.value.clone());
                    }
                    Some((node, depth))
                }
            } else {
                None
            }
        } else {
            //------------------ depth first search  (pre-order) --------------------
            if let Some((node, depth)) = self.queue.pop_back() {
                //note: pop from back instead of front here
                if self.maxdepth.is_none() || depth < self.maxdepth.expect("get maxdepth") {
                    if self.unique && self.visited.contains(&node.value) {
                        return self.next(); //node was already visited, recurse to next
                    }

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

                        if self.unique {
                            let visited = &self.visited; //borrow outside closure otherwise borrow checker gets confused
                            self.queue.extend(
                                children
                                    .filter(|child| !visited.contains(&child.value))
                                    .map(|child| (child, depth + 1)),
                            );
                        } else {
                            self.queue.extend(children.map(|child| (child, depth + 1)));
                        }
                    }
                }

                //don't yield the root element (or empty leaves if we don't want them), just recurse in that case
                if (depth < self.mindepth) || (!self.empty_leaves && node.value.is_empty()) {
                    self.next()
                } else {
                    if self.unique {
                        self.visited.insert(node.value.clone());
                    }
                    Some((node, depth))
                }
            } else {
                None
            }
        }
    }
}
