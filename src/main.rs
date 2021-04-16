extern crate clap;
extern crate num_bigint;

use std::collections::{HashMap,HashSet,VecDeque,BTreeMap};
use std::fs::File;
use std::io::{self, Write,Read,BufReader,BufRead,Error};
use std::ops::Deref;
use std::iter::{Extend,FromIterator};
use clap::{Arg, App, SubCommand};
use num_bigint::BigUint;
use num_traits::{Zero, One};

///Each type gets assigned an ID integer, carries no further meaning
type VocabId = u64;

///A normalized string encoded via the alphabet
type NormString = Vec<u8>;

const PRIMES: &[u32] = &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193, 197, 199, 211, 223, 227, 229, 233, 239, 241, 251, 257, 263, 269, 271, 277, 281, 283, 293, 307, 311, 313, 317, 331, 337, 347, 349, 353, 359, 367, 373, 379, 383, 389, 397, 401, 409, 419, 421, 431, 433, 439, 443, 449, 457, 461, 463, 467, 479, 487, 491, 499, 503, 509, 521, 523, 541, 547, 557, 563, 569, 571, 577, 587, 593, 599, 601, 607, 613, 617, 619, 631, 641, 643, 647, 653, 659, 661, 673, 677, 683, 691, 701, 709, 719, 727, 733, 739, 743, 751, 757, 761, 769, 773, 787, 797, 809, 811, 821, 823, 827, 829, 839, 853, 857, 859, 863, 877, 881, 883, 887, 907, 911, 919, 929, 937, 941, 947, 953, 967, 971, 977, 983, 991, 997];

#[derive(Clone)]
struct VocabValue {
    text: String,
    norm: NormString,
    frequency: u32,
    ///The number of words
    tokencount: u8,
}


///Map integers (indices correspond to VocabId) to string values (and optionally a frequency count)
type VocabDecoder = Vec<VocabValue>;

///Maps strings to integers
type VocabEncoder = HashMap<String, VocabId>;

///The anagram hash: uses a bag-of-characters representation where each bit flags the presence/absence of a certain character (the order of the bits are defined by Alphabet)
type AnaValue = BigUint;

///Defines the alphabet, index corresponds how things are encoded, multiple strings may be encoded
///in the same way
type Alphabet = Vec<Vec<String>>;



#[derive(Default)]
struct AnaIndexNode {
    ///Maps an anagram value to all existing instances that instantiate it
    instances: Vec<VocabId>,

    charcount: u16
}


type AnaIndex = HashMap<AnaValue,AnaIndexNode>;

struct VariantModel {
    decoder: VocabDecoder,
    encoder: VocabEncoder,

    alphabet: Alphabet,

    ///The main index, mapping anagrams to instances
    index: AnaIndex,

    ///A secondary sorted index
    ///indices of the outer vector correspond to the length of an anagram (in chars)  - 1
    ///Inner vector is always sorted
    sortedindex: BTreeMap<u16,Vec<AnaValue>>,

    ///Does the model have frequency information?
    have_freq: bool,

    debug: bool
}


///Trait for objects that can be anahashed (string-like)
trait Anahashable {
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
                            let charvalue = AnaValue::character(seqnr);
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
                let charvalue = AnaValue::character(alphabet.len());
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
                            result.push(i as u8);
                            skip = l-1;
                            break 'abciter;
                        }
                    }
                }
            }
            if !matched {
                //Highest one is reserved for UNK
                result.push(alphabet.len() as u8);
            }
        }
        result
    }

}

//Trait for objects that are anahashes
trait Anahash: One + Zero {
    fn character(seqnr: usize) -> AnaValue;
    fn empty() -> AnaValue;
    fn is_empty(&self) -> bool;
    fn insert(&self, value: &AnaValue) -> AnaValue;
    fn delete(&self, value: &AnaValue) -> Option<AnaValue>;
    fn contains(&self, value: &AnaValue) -> bool;
    fn iter(&self, alphabet_size: usize) -> AnaValueIterator;
    fn char_count(&self, alphabet_size: usize) -> u16;
}

impl Anahash for AnaValue {
    /// Computes the Anagram value for the n'th entry in the alphabet
    fn character(seqnr: usize) -> AnaValue {
        BigUint::from(PRIMES[seqnr])
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

    /// Iterates over all characters in an anagram value
    /// Does not yield duplicates!
    fn iter(&self, alphabet_size: usize) -> AnaValueIterator {
        AnaValueIterator::new(self, alphabet_size)
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
    fn char_count(&self, alphabet_size: usize) -> u16 {
        if self.is_empty() {
            0
        } else {
            return 1 + self.iter(alphabet_size).next().expect("Char Iterator").char_count(alphabet_size)
        }
    }

}

/// Returns all AnaValues that are contained
/// when doing single deletion
struct AnaValueIterator<'a> {
    value: &'a AnaValue,
    alphabet_size: usize,
    iteration: usize,
}

impl<'a> AnaValueIterator<'a> {
    pub fn new(value: &'a AnaValue, alphabet_size: usize) -> AnaValueIterator {
        AnaValueIterator {
            value: value,
            alphabet_size: alphabet_size,
            iteration: 0
        }
    }
}

impl<'a> Iterator for AnaValueIterator<'a> {
    type Item = AnaValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.value == &AnaValue::one() || self.iteration == self.alphabet_size {
            None
        } else {
            self.iteration += 1;
            if let Some(result) = self.value.delete(&AnaValue::character(self.iteration-1)) {
                Some(result)
            } else {
                self.next() //recurse
            }
        }
    }
}








struct VocabParams {
    ///Column containing the Text (if any, 0-indexed)
    text_column: u8,
    ///Column containing the absolute frequency (if any, 0-indexed)
    freq_column: Option<u8>
}

impl Default for VocabParams {
    fn default() -> Self {
        Self {
            text_column: 0,
            freq_column: None,
        }
    }
}

////////////////////////////////////////////////////////////////////

/*

///Recursive iterator over anagram values
///Can be used to compute all deletions
///Never returns duplicates
struct AncestorIterator<'a> {
    alphabet_size: usize,
    queue: VecDeque<(AnaValue,usize)>, //(child,parent,depth)
    visited: HashSet<AnaValue>,
    index: Option<&'a AnaIndex>,
}

impl<'a> AncestorIterator<'a> {
    fn new(anavalue: AnaValue, index: Option<&'a AnaIndex>, alphabet_size: usize) -> Self {
        Self {
            alphabet_size: alphabet_size,
            index: index,
            queue: VecDeque::from(vec!((anavalue, 0))),
            visited: HashSet::new(),
        }
    }

    ///Tests if the specified value has already been queued
    fn queued(&self, refvalue: &AnaValue) -> bool {
        for item in self.queue.iter() {
            if *refvalue == item.0 {
                return true
            }
        }
        false
    }

}

impl<'a> Iterator for AncestorIterator<'a>
{
    type Item = (AnaValue, usize);

    fn next(&mut self) -> Option<Self::Item> {
        //Pop the next time to consider from the queue
        if let Some((anahash, depth)) = self.queue.pop_front() {
            //Do not expand items that already have parents in the index
            let expand = if let Some(index) = self.index {
                if let Some(node) = index.get(&anahash) {
                    node.parents.is_empty()
                } else {
                    true
                }
            } else {
                true
            };
            if expand {
                for deletion in anahash.iter(self.alphabet_size) {
                    let child = anahash.delete(&deletion);
                    if !self.visited.contains(&child) && !self.queued(&child) {
                        self.queue.push_back((child, depth));
                    }
                }
            }
            self.visited.insert(anahash.clone());
            Some((anahash, depth))
        } else {
            None
        }
    }

}
*/


////////////////////////////////////////////////////////////////////
/*
///Merges a sorted source vector into a sorted target vector, ignoring duplicates
fn merge_into<T: std::cmp::Ord + Clone>(target: &mut Vec<T>, source: &[T]) {
    let mut pos = 0;
    'outer: for elem in source.iter() {
        for refelem in &target[pos..] {
            if *refelem == *elem {
                break 'outer;
            } else if *refelem >= *elem {
                break;
            }
            pos += 1;
        }
        target.insert(pos, elem.clone());
    }
}

///Merges a sorted source vector into a sorted target vector, ignoring duplicates
fn merge_while_expanding<F>(target: &mut Vec<AnaValue>, source: Vec<AnaValue>, map_callback: F)
    where F: Fn(&AnaValue) -> Vec<AnaValue> {
    let mut pos = 0;
    'outer: for elem in source {
        for refelem in &target[pos..] {
            if *refelem == elem {
                break 'outer;
            } else if *refelem >= elem {
                break;
            }
            pos += 1;
        }
        target.insert(pos, elem);
        merge_while_expanding(target, map_callback(target.get(pos).unwrap()), map_callback);
    }
}
*/

///Compute levenshtein distance between two normalised strings
///Returns None if the maximum distance is exceeded
fn levenshtein(a: &[u8], b: &[u8], max_distance: u8) -> Option<u8> {
    //Freely adapted from levenshtein-rs (MIT licensed, 2016 Titus Wormer <tituswormer@gmail.com>)
    if a == b {
        return Some(0);
    }


    let length_a = a.len();
    let length_b = b.len();

    if length_a == 0 {
        if length_b > max_distance as usize {
            return None;
        } else {
            return Some(length_b as u8);
        }
    } else if length_a > length_b {
        if length_a - length_b > max_distance as usize {
            return None;
        }
    }
    if length_b == 0 {
        if length_a > max_distance as usize {
            return None;
        } else {
            return Some(length_a as u8);
        }
    } else if length_b > length_a {
        if length_b - length_a > max_distance as usize {
            return None;
        }
    }

    let mut cache: Vec<usize> = (1..).take(length_a).collect();
    let mut distance_a;
    let mut distance_b;
    let mut result = 0;

    for (index_b, elem_b) in b.iter().enumerate() {
        result = index_b;
        distance_a = index_b;

        for (index_a, elem_a) in a.iter().enumerate() {
            distance_b = if elem_a == elem_b {
                distance_a
            } else {
                distance_a + 1
            };

            distance_a = cache[index_a];

            result = if distance_a > result {
                if distance_b > result {
                    result + 1
                } else {
                    distance_b
                }
            } else if distance_b > distance_a {
                distance_a + 1
            } else {
                distance_b
            };

            cache[index_a] = result;
        }
    }

    if result > max_distance as usize {
        None
    } else {
        Some(result as u8)
    }
}

impl VariantModel {
    fn new(alphabet_file: &str, vocabulary_file: &str, vocabparams: Option<VocabParams>, debug: bool) -> VariantModel {
        let mut model = VariantModel {
            alphabet: Vec::new(),
            encoder: HashMap::new(),
            decoder: Vec::new(),
            index: HashMap::new(),
            sortedindex: BTreeMap::new(),
            have_freq: false,
            debug: debug,
        };
        model.read_alphabet(alphabet_file).expect("Error loading alphabet file");
        model.read_vocabulary(vocabulary_file, vocabparams).expect("Error loading vocabulary file");
        model
    }

    fn alphabet_size(&self) -> usize {
        self.alphabet.len() + 1 //+1 for UNK
    }

    fn get_or_create_node<'a,'b>(&'a mut self, anahash: &'b AnaValue) -> &'a mut AnaIndexNode {
            if self.contains_key(anahash) {
                self.index.get_mut(anahash).expect("get_mut on node after check")
            } else {
                self.index.insert(anahash.clone(), AnaIndexNode {
                    instances: Vec::new(),
                    charcount: anahash.char_count(self.alphabet_size())
                });
                self.index.get_mut(&anahash).expect("get_mut on node after insert")
            }
    }

    fn train(&mut self) {
        eprintln!("Computing anagram values for all items in the lexicon...");

        let alphabet_size = self.alphabet_size();

        // Hash all strings in the lexicon
        // and add them to the index
        let mut tmp_hashes: Vec<(AnaValue,VocabId)> = Vec::with_capacity(self.decoder.len());
        for (id, value)  in self.decoder.iter().enumerate() {
            //get the anahash
            let anahash = value.text.anahash(&self.alphabet);
            if self.debug {
                eprintln!("   -- Anavalue={} VocabId={} Text={}", &anahash, id, value.text);
            }
            tmp_hashes.push((anahash, id as VocabId));
        }
        eprintln!(" - Found {} instances",tmp_hashes.len());

        eprintln!("Adding all instances to the index");
        for (anahash, id) in tmp_hashes {
            //add it to the index
            let node = self.get_or_create_node(&anahash);
            node.instances.push(id);
        }
        eprintln!(" - Found {} anagrams", self.index.len() );

        eprintln!("Creating sorted secondary index");
        for (anahash, node) in self.index.iter() {
            if !self.sortedindex.contains_key(&node.charcount) {
                self.sortedindex.insert(node.charcount, Vec::new());
            }
            let keys = self.sortedindex.get_mut(&node.charcount).expect("getting sorted index (1)");
            keys.push(anahash.clone());  //TODO: see if we can make this a reference later
        }

        eprintln!("Sorting secondary index");
        let mut sizes: Vec<u16> = self.sortedindex.keys().map(|x| *x).collect();
        sizes.sort_unstable();
        for size in sizes {
            let keys = self.sortedindex.get_mut(&size).expect("getting sorted index (2)");
            keys.sort_unstable();
            eprintln!(" - Found {} anagrams of length {}", keys.len(), size );
        }
    }

    fn compute_deletions(&self, target: &mut HashMap<AnaValue,Vec<AnaValue>>, queue: &[AnaValue], max_distance: u8)  {
        if self.debug {
            eprintln!("Computing deletions within distance {}...",max_distance);
        }

        let alphabet_size = self.alphabet_size();

        let mut queue: Vec<AnaValue> = Vec::from(queue);

        // Compute deletions for all instances, expanding
        // recursively also to anahashes which do not have instances
        // which are created on the fly
        for depth in 0..max_distance {
            queue.sort_unstable();
            let mut nextqueue: Vec<AnaValue> = Vec::new();
            let length = queue.len();
            for (i, anahash) in queue.iter().enumerate() {
              if !target.contains_key(anahash) {
                if self.debug {
                    eprintln!(" - Depth {}: @{}/{}",depth+1, i+1, length );
                }
                let newparents: Vec<AnaValue> = anahash.iter(alphabet_size).collect();
                target.insert(anahash.clone(), newparents );

                if depth + 1 < max_distance {
                    let mut total = 0;
                    let mut expanded = 0;
                    for p in target.get(&anahash).unwrap() {
                        total += 1;
                        if !target.contains_key(&p) { //no duplicates in the queue
                            expanded += 1;
                            nextqueue.push(p.clone());
                        }
                    }

                    if self.debug {
                        eprintln!(" - Queued {} extra nodes (out of {})", expanded, total );
                    }
                }
              }
            }
            let _oldqueue = std::mem::replace(&mut queue, nextqueue);
        }

    }



    ///Find all insertions within a certain distance
    /*
    fn expand_insertions(&self, target: &mut Vec<AnaValue>, query: AnaValue, hashes: Vec<AnaValue>, max_distance: u8) {
        merge_while_expanding(target, hashes, |anahash| {
            if let Some(children) = self.insertions.get(&anahash) {
                children.iter().map(|x| *x).filter(|x| query.sizediff(*x) <= max_distance).collect::<Vec<AnaValue>>(),
                self.expand_insertions(target,
                                       query,
                                       children.iter().map(|x| *x).filter(|x| query.sizediff(*x) <= max_distance).collect::<Vec<AnaValue>>(),
                                       max_distance);
            } else {
                vec!()
            }
        });
    }
    */



    fn contains_key(&self, key: &AnaValue) -> bool {
        self.index.contains_key(key)
    }

    fn has_instances(&self, key: &AnaValue) -> bool {
        if let Some(node) = self.index.get(key) {
            !node.instances.is_empty()
        } else {
            false
        }
    }



    ///Read the alphabet from a TSV file
    ///The file contains one alphabet entry per line, but may
    ///consist of multiple tab-separated alphabet entries on that line, which
    ///will be treated as the identical.
    ///The alphabet is not limited to single characters but may consist
    ///of longer string, a greedy matching approach will be used so order
    ///matters (but only for this)
    fn read_alphabet(&mut self, filename: &str) -> Result<(), std::io::Error> {
        if self.debug {
            eprintln!("Reading alphabet from {}...", filename);
        }
        let f = File::open(filename)?;
        let f_buffer = BufReader::new(f);
        for line in f_buffer.lines() {
            if let Ok(line) = line {
                if !line.is_empty() {
                    self.alphabet.push(line.split("\t").map(|x| x.to_owned()).collect());
                }

            }
        }
        if self.debug {
            eprintln!(" -- Read alphabet of size {}", self.alphabet.len());
        }
        Ok(())
    }

    ///Read vocabulary from a TSV file
    ///The parameters define what value can be read from what column
    fn read_vocabulary(&mut self, filename: &str, params: Option<VocabParams>) -> Result<(), std::io::Error> {
        if self.debug {
            eprintln!("Reading vocabulary from {}...", filename);
        }
        let params = params.unwrap_or_default();
        let f = File::open(filename)?;
        let f_buffer = BufReader::new(f);
        for line in f_buffer.lines() {
            if let Ok(line) = line {
                if !line.is_empty() {
                    let fields: Vec<&str> = line.split("\t").collect();
                    let text = fields.get(params.text_column as usize).expect("Expected text column not found");
                    let frequency = if let Some(freq_column) = params.freq_column {
                        self.have_freq = true;
                        fields.get(freq_column as usize).expect("Expected frequency column not found").parse::<u32>().expect("frequency should be a valid integer")
                    } else {
                        1
                    };
                    //self.encoder.insert(text.to_string(), self.decoder.len() as u64);
                    if self.debug {
                        eprintln!(" -- Adding to vocabulary: {}", text);
                    }
                    self.decoder.push(VocabValue {
                        text: text.to_string(),
                        norm: text.normalize_to_alphabet(&self.alphabet),
                        frequency: frequency,
                        tokencount: text.chars().filter(|c| *c == ' ').count() as u8 + 1
                    });
                }
            }
        }
        if self.debug {
            eprintln!(" - Read vocabulary of size {}", self.decoder.len());
        }
        Ok(())
    }

    /// Find variants in the vocabulary for a given string (in its totality), returns a vector of string,score pairs
    fn find_variants<'a>(&'a self, s: &str, max_anagram_distance: u8, max_edit_distance: u8) -> Vec<(&'a str, f64)> {

        //Compute the anahash
        let normstring = s.normalize_to_alphabet(&self.alphabet);
        let anahash = s.anahash(&self.alphabet);

        //Compute neighbouring anahashes and find the nearest anahashes in the model
        let anahashes = self.find_nearest_anahashes(&anahash, max_anagram_distance);

        //Get the instances pertaining to the collected hashes, within a certain maximum distance
        let variants = self.gather_instances(&anahashes, &normstring, max_edit_distance);

        self.score_and_resolve(variants, self.have_freq)
    }


    /// Resolve and score all variants
    fn score_and_resolve(&self, instances: Vec<(VocabId,u8)>, use_freq: bool) -> Vec<(&str,f64)> {
        let mut results: Vec<(&str,f64)> = Vec::new();
        let mut max_distance = 0;
        let mut max_freq = 0;
        for (vocab_id, distance) in instances.iter() {
            if *distance > max_distance {
                max_distance = *distance;
            }
            if use_freq {
                if let Some(vocabitem) = self.decoder.get(*vocab_id as usize) {
                    if vocabitem.frequency > max_freq {
                        max_freq = vocabitem.frequency;
                    }
                }
            }
        }
        for (vocab_id, distance) in instances.iter() {
            if let Some(vocabitem) = self.decoder.get(*vocab_id as usize) {
                let distance_score: f64 = 1.0 - (*distance as f64 / max_distance as f64);
                let freq_score: f64 = if use_freq {
                   vocabitem.frequency as f64 / max_freq as f64
                } else {
                    1.0
                };
                let score = distance_score * freq_score;
                results.push( (&vocabitem.text, score) );
            }
        }
        results.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); //sort by score, descending order
        results
    }

    /// Gather instances and their edit distances, given a search string (normalised to the alphabet) and anagram hashes
    fn gather_instances(&self, hashes: &[AnaValue], querystring: &[u8], max_edit_distance: u8) -> Vec<(VocabId,u8)> {
        let mut found_instances = Vec::new();
        for anahash in hashes {
            if let Some(node) = self.index.get(anahash) {
                for vocab_id in node.instances.iter() {
                    if let Some(vocabitem) = self.decoder.get(*vocab_id as usize) {
                        if let Some(distance) = levenshtein(querystring, &vocabitem.norm, max_edit_distance) {
                            found_instances.push((*vocab_id,distance));
                        }
                    }
                }
            }
        }
        found_instances.sort_unstable_by_key(|k| k.1 ); //sort by distance, ascending order
        found_instances
    }

    /// Find the nearest anahashes that exists in the model (computing anahashes in the
    /// neigbhourhood if needed). Note: this also returns anahashes that have no instances
    fn find_nearest_anahashes(&self, anahash: &AnaValue, max_distance: u8) -> Vec<AnaValue> {
        if self.index.contains_key(anahash) {
            //the easiest case, this anahash exists in the model
            vec!(anahash.clone())
        } else if max_distance > 0 {
            let mut parents = HashMap::new();
            self.compute_deletions(&mut parents, &[anahash.clone()], max_distance);
            if let Some(results) = parents.get(&anahash) {
                results.to_vec()
            } else {
                vec!()
            }
        } else {
            vec!()
        }
    }


}


fn process_tsv(model: &VariantModel, input: &str, max_anagram_distance: u8, max_edit_distance: u8) {
    let variants = model.find_variants(&input, max_anagram_distance, max_edit_distance);
    print!("{}",input);
    for (variant, score) in variants {
        print!("\t{}\t{}\t",variant, score);
    }
    println!();
}

fn main() {
    let args = App::new("Analiticcl")
                    .version("0.1")
                    .author("Maarten van Gompel (proycon) <proycon@anaproy.nl>")
                    .about("Spelling variant matching")
                    //snippet hints --> addargb,addargs,addargi,addargf,addargpos
                    .arg(Arg::with_name("lexicon")
                        .long("lexicon")
                        .short("l")
                        .help("Lexicon against which all matches are made")
                        .takes_value(true)
                        .required(true))
                    .arg(Arg::with_name("alphabet")
                        .long("alphabet")
                        .short("a")
                        .help("Alphabet file")
                        .takes_value(true)
                        .required(true))
                    .arg(Arg::with_name("max_anagram_distance")
                        .long("max-anagram-distance")
                        .short("A")
                        .help("Maximum anagram distance. This impacts the size of the search space")
                        .takes_value(true)
                        .default_value("3"))
                    .arg(Arg::with_name("max_edit_distance")
                        .long("max-edit-distance")
                        .short("d")
                        .help("Maximum edit distance (levenshtein)")
                        .takes_value(true)
                        .default_value("3"))
                    .arg(Arg::with_name("files")
                        .help("Input files")
                        .takes_value(true)
                        .multiple(true)
                        .required(false))
                    .arg(Arg::with_name("debug")
                        .long("debug")
                        .short("D")
                        .help("Debug")
                        .required(false))
                    .arg(Arg::with_name("printindex")
                        .long("printindex")
                        .short("I")
                        .help("Output the entire index")
                        .required(false))
                    .get_matches();

    eprintln!("Loading model resources...");
    let mut model = VariantModel::new(
        args.value_of("alphabet").unwrap(),
        args.value_of("lexicon").unwrap(),
        Some(VocabParams::default()),
        args.is_present("debug")

    );

    eprintln!("Training model...");
    model.train();

    let max_anagram_distance: u8 = args.value_of("max_anagram_distance").unwrap().parse::<u8>().expect("Anagram distance should be an integer between 0 and 255");
    let max_edit_distance: u8 = args.value_of("max_edit_distance").unwrap().parse::<u8>().expect("Anagram distance should be an integer between 0 and 255");

    if args.is_present("printindex") {
        for (anahash, indexnode) in model.index.iter() {
            if !indexnode.instances.is_empty() {
                print!("{}", anahash);
                for instance in indexnode.instances.iter() {
                    let vocabvalue = model.decoder.get(*instance as usize).expect("decoding instance");
                    print!("\t{}", vocabvalue.text);
                }
                println!()
            }
        }

    } else {
        eprintln!("Testing against model...");
        let files: Vec<_> = if args.is_present("files") {
            args.values_of("files").unwrap().collect()
        } else {
            eprintln!("(accepting standard input)");
            vec!("-")
        };
        for filename in files {
            match filename {
                "-" | "STDIN" | "stdin"  => {
                    let stdin = io::stdin();
                    let f_buffer = BufReader::new(stdin);
                    for line in f_buffer.lines() {
                        if let Ok(line) = line {
                            process_tsv(&model, &line, max_anagram_distance, max_edit_distance);
                        }
                    }
                },
                _ =>  {
                    let f = File::open(filename).expect(format!("ERROR: Unable to open file {}", filename).as_str());
                    let f_buffer = BufReader::new(f);
                    for line in f_buffer.lines() {
                        if let Ok(line) = line {
                            process_tsv(&model, &line, max_anagram_distance, max_edit_distance);
                        }
                    }
                }
            }
        }
    }
}
