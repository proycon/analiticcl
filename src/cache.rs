use std::collections::HashSet;

use crate::types::*;

#[derive(Default)]
pub struct Cache {
    pub(crate) visited: HashSet<AnaValue>,
    visited_max_size: usize,
}

impl Cache {
    pub fn new(visited_max_size: usize) -> Cache {
        Cache {
            visited: HashSet::with_capacity(visited_max_size),
            visited_max_size: visited_max_size,
        }
    }

    pub fn clear(&mut self) {
        self.visited.clear();
    }

    pub fn check(&mut self) {
        if self.visited_max_size > 0 && (self.visited.len() > self.visited_max_size) {
            self.visited.clear();
        }
    }

}
