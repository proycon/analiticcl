extern crate clap;
extern crate num_bigint;
extern crate primes;

use std::collections::{HashMap,VecDeque};
use std::fs::File;
use std::io::{Write,Read,BufReader,BufRead,Error};
use std::ops::Deref;
use std::iter::{Extend,FromIterator};
use clap::{Arg, App, SubCommand};
use num_bigint::BigUint;
use num_traits::{Zero, One};

///Each type gets assigned an ID integer, carries no further meaning
type VocabId = u64;

///A normalized string encoded via the alphabet
type NormString = Vec<u8>;

const PRIMES: &[usize] = &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193, 197, 199, 211, 223, 227, 229, 233, 239, 241, 251, 257, 263, 269, 271, 277, 281, 283, 293, 307, 311, 313, 317, 331, 337, 347, 349, 353, 359, 367, 373, 379, 383, 389, 397, 401, 409, 419, 421, 431, 433, 439, 443, 449, 457, 461, 463, 467, 479, 487, 491, 499, 503, 509, 521, 523, 541, 547, 557, 563, 569, 571, 577, 587, 593, 599, 601, 607, 613, 617, 619, 631, 641, 643, 647, 653, 659, 661, 673, 677, 683, 691, 701, 709, 719, 727, 733, 739, 743, 751, 757, 761, 769, 773, 787, 797, 809, 811, 821, 823, 827, 829, 839, 853, 857, 859, 863, 877, 881, 883, 887, 907, 911, 919, 929, 937, 941, 947, 953, 967, 971, 977, 983, 991, 997];

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
struct AnaTreeNode {
    ///Maps an anagram value to all existing instances that instantiate it
    instances: Vec<VocabId>,

    ///Maps an anagram value to all anagram values that delete a single character (deletions)
    parents: Vec<AnaValue>,

    ///Maps an anagram value to all anagram values that add a single character (insertions)
    children: Vec<AnaValue>,
}


type AnaTree = HashMap<AnaValue,AnaTreeNode>;

struct VariantModel {
    decoder: VocabDecoder,
    encoder: VocabEncoder,

    alphabet: Alphabet,

    tree: AnaTree,

    ///Does the model have frequency information?
    have_freq: bool
}


///Trait for objects that can be anahashed (string-like)
trait Anahashable {
    fn anahash(&self, alphabet: &Alphabet) -> AnaValue;
    fn normalize_to_alphabet(&self, alphabet: &Alphabet) -> NormString;
}

impl Anahashable for str {
    ///Compute the anahash for a given string, according to the alphabet
    fn anahash(&self, alphabet: &Alphabet) -> AnaValue {
        let mut hash: AnaValue = AnaValue::zero();
        for (pos, _) in self.char_indices() {
            for chars in alphabet.iter() {
                for element in chars.iter() {
                    let l = element.chars().count();
                    if let Some(slice) = self.get(pos..pos+l) {
                        if slice == element {
                            let charvalue = AnaValue::character(pos);
                            hash.insert(&charvalue);
                            break;
                        }
                    }
                }
            }
        }
        hash
    }


    ///Normalize a string via the alphabet
    fn normalize_to_alphabet(&self, alphabet: &Alphabet) -> NormString {
        let mut result = Vec::with_capacity(self.chars().count());
        for (pos, c) in self.char_indices() {
            //does greedy matching in order of appearance in the alphabet file
            for (i, chars) in alphabet.iter().enumerate() {
                for element in chars.iter() {
                    let l = element.chars().count();
                    if let Some(slice) = self.get(pos..pos+l) {
                        if slice == element {
                            result.push(i as u8);
                            break;
                        }
                    }
                }
            }
        }
        result
    }

}

//Trait for objects that are anahashes
trait Anahash: Zero {
    fn character(seqnr: usize) -> AnaValue;
    fn insert(&self, value: &AnaValue) -> AnaValue;
    fn delete(&self, value: &AnaValue) -> Option<AnaValue>;
    fn contains(&self, value: &AnaValue) -> bool;
    fn iter(&self, alphabet_size: usize) -> AnaValueIterator;
}

impl Anahash for AnaValue {
    /// Computes the Anagram value for the n'th entry in the alphabet
    fn character(seqnr: usize) -> AnaValue {
        BigUint::from(PRIMES[seqnr])
    }

    /// Insert the characters represented by the anagram value, returning the result
    fn insert(&self, value: &AnaValue) -> AnaValue {
        self * value
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
        if self > value {
            false
        } else {
            (self % value) == AnaValue::zero()
        }
    }

    /// Iterates over all characters in an anagram value
    /// Does not yield duplicates!
    fn iter(&self, alphabet_size: usize) -> AnaValueIterator {
        AnaValueIterator::new(self.clone(), alphabet_size)
    }

}

/// Iterates over all characters in an anagram value
/// Does not yield duplicates
struct AnaValueIterator {
    value: AnaValue,
    alphabet_size: usize,
    iteration: usize,
}

impl AnaValueIterator {
    pub fn new(value: AnaValue, alphabet_size: usize) -> AnaValueIterator {
        AnaValueIterator {
            value: value,
            alphabet_size: alphabet_size,
            iteration: 0
        }
    }
}

impl<'a> Iterator for AnaValueIterator {
    type Item = AnaValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.value == AnaValue::zero() || self.iteration == self.alphabet_size {
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






///Read the alphabet from a TSV file
///The file contains one alphabet entry per line, but may
///consist of multiple tab-separated alphabet entries on that line, which
///will be treated as the identical.
///The alphabet is not limited to single characters but may consist
///of longer string, a greedy matching approach will be used so order
///matters (but only for this)
fn read_alphabet(filename: &str) -> Result<Alphabet, std::io::Error> {
    let mut alphabet: Alphabet = Vec::new();
    let f = File::open(filename)?;
    let f_buffer = BufReader::new(f);
    for line in f_buffer.lines() {
        if let Ok(line) = line {
            if !line.is_empty() {
                alphabet.push(line.split("\t").map(|x| x.to_owned()).collect());
            }
        }
    }
    Ok(alphabet)
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
    tree: Option<&'a AnaTree>,
}

impl<'a> AncestorIterator<'a> {
    fn new(anavalue: AnaValue, tree: Option<&'a AnaTree>, alphabet_size: usize) -> Self {
        Self {
            alphabet_size: alphabet_size,
            tree: tree,
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
            //Do not expand items that already have parents in the tree
            let expand = if let Some(tree) = self.tree {
                if let Some(node) = tree.get(&anahash) {
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
    fn new(alphabet_file: &str, vocabulary_file: &str, vocabparams: Option<VocabParams>) -> VariantModel {
        let mut model = VariantModel {
            alphabet: read_alphabet(alphabet_file).expect("Error loading alphabet file"),
            encoder: HashMap::new(),
            decoder: Vec::new(),
            tree: HashMap::new(),
            have_freq: false,
        };
        model.read_vocabulary(vocabulary_file, vocabparams).expect("Error loading vocabulary file");
        model
    }

    fn get_or_create_node<'a,'b>(&'a mut self, anahash: &'b AnaValue) -> &'a mut AnaTreeNode {
            if self.contains_key(anahash) {
                self.tree.get_mut(anahash).expect("get_mut on node after check")
            } else {
                self.tree.insert(anahash.clone(), AnaTreeNode::default());
                self.tree.get_mut(&anahash).expect("get_mut on node after insert")
            }
    }

    fn train(&mut self) {
        eprintln!("Computing anagram values for all items in the lexicon...");

        let alphabet_size = self.alphabet.len();

        {
            // Hash all strings in the lexicon
            // and add them to the tree
            let mut tmp_hashes: Vec<(AnaValue,VocabId)> = Vec::with_capacity(self.encoder.len());
            for (s, id)  in self.encoder.iter() {
                //get the anahash
                let anahash = s.anahash(&self.alphabet);
                tmp_hashes.push((anahash, *id));
            }

            eprintln!("Adding all instances to the tree");
            for (anahash, id) in tmp_hashes {
                //add it to the tree
                let node = self.get_or_create_node(&anahash);
                node.instances.push(id);
            }
        }

        eprintln!("Computing deletions...");

        // Create a queue of all anahash keys currently in the tree
        // (which is the ones having instances)
        let mut sorted_keys: Vec<&AnaValue> = self.tree.keys().collect();
        //Sort them first to make the next algorithm a more efficient
        sorted_keys.sort();

        let mut queue: VecDeque<AnaValue> = VecDeque::from_iter(sorted_keys.into_iter().map(|x| x.clone()));


        // Compute deletions for all instances, expanding
        // recursively also to anahashes which do not have instances
        // which are created on the fly
        // so we have complete route for all anahashes
        while let Some(anahash) = queue.pop_front() {
            let node = self.get_or_create_node(&anahash);
            node.parents = anahash.iter(alphabet_size).collect::<Vec<AnaValue>>();

            let node = self.tree.get(&anahash).expect("getting node immutably after creation"); //needed to lose the mutability and prevent conflicts
            for parent in node.parents.iter() {
                //expand only if the node hasn't been expanded before
                let expand = if let Some(parentnode) = self.tree.get(parent) {
                    parentnode.parents.is_empty()
                } else {
                    true
                };
                if expand {
                    if !queue.contains(&parent) { //no duplicates in the queue
                        queue.push_front(parent.clone()); //we push to the front so have the benefit of the ordering
                    }
                }
            }
        }

        eprintln!("Computing insertions...");

        // Insertions are simply the reverse of deletions
        let mut insertions: Vec<(AnaValue,AnaValue)> = Vec::new();
        for (anahash, node) in self.tree.iter() {
            for parent in node.parents.iter() {
                insertions.push((parent.clone(), anahash.clone()));
            }
        }

        for (parent, child) in insertions {
            let parentnode = self.get_or_create_node(&parent);
            parentnode.children.push(child.clone());
        }

        eprintln!("Sorting tree nodes...");

        // Sort the insertions in a separate step
        for (_, node) in self.tree.iter_mut() {
            node.parents.sort_unstable();
            node.children.sort_unstable();
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
        self.tree.contains_key(key)
    }

    fn has_instances(&self, key: &AnaValue) -> bool {
        if let Some(node) = self.tree.get(key) {
            !node.instances.is_empty()
        } else {
            false
        }
    }

    fn contains(&self, s: &str) -> bool {
        self.encoder.contains_key(s)
    }



    ///Read vocabulary from a TSV file
    ///The parameters define what value can be read from what column
    fn read_vocabulary(&mut self, filename: &str, params: Option<VocabParams>) -> Result<(), std::io::Error> {
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
                    self.encoder.insert(text.to_string(), self.decoder.len() as u64);
                    self.decoder.push(VocabValue {
                        text: text.to_string(),
                        norm: text.normalize_to_alphabet(&self.alphabet),
                        frequency: frequency,
                        tokencount: text.chars().filter(|c| *c == ' ').count() as u8 + 1
                    });
                }
            }
        }
        Ok(())
    }

    /*
    /// Find variants in the vocabulary for a given string (in its totality), returns a vector of string,score pairs
    fn find_variants<'a>(&'a self, s: &str, max_anagram_distance: u8, max_edit_distance: u8) -> Vec<(&'a str, f64)> {

        //Compute the anahash
        let normstring = s.normalize_to_alphabet(&self.alphabet);
        let anahash = s.anahash(&self.alphabet);

        //Find the nearest anahashes in the model
        let anahashes = self.find_nearest_anahashes(&anahash, max_anagram_distance);

        //Expand anahashes using insertions
        let mut expanded_anahashes = Vec::new();
        self.expand_insertions(&mut expanded_anahashes, anahash, &anahashes, max_anagram_distance);

        //Get the instances pertaining to the collected hashes, within a certain maximum distance
        let variants: Vec<(VocabId,u8)> = self.gather_instances(&expanded_anahashes, &normstring, max_edit_distance);

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
            if let Some(instances) = self.instances.get(anahash) {
                for vocab_id in instances {
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

    /// Find the nearest anahashes that exists in the model
    fn find_nearest_anahashes(&self, anahash: &AnaValue, max_distance: u8) -> Vec<AnaValue> {
        if self.contains_anahash(*anahash) {
            //the easiest case, this anahash exists in the model
            vec!(*anahash)
        } else if max_distance > 0 {
            let mut results = Vec::new();
            let parents: Vec<AnaValue> = self.compute_deletions(*anahash, AnahashExpandMode::All);
            for anahash in parents {
                merge_into::<AnaValue>(&mut results, &self.find_nearest_anahashes(&anahash, max_distance - 1) )
            }
            results
        } else {
            vec!()
        }
    }
    */


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
                        .required(true))
                    .get_matches();

    eprintln!("Loading model resources...");
    let mut model = VariantModel::new(
        args.value_of("alphabet").unwrap(),
        args.value_of("lexicon").unwrap(),
        Some(VocabParams::default())
    );

    eprintln!("Training model...");
    model.train();

    let max_anagram_distance: u8 = args.value_of("max_anagram_distance").unwrap().parse::<u8>().expect("Anagram distance should be an integer between 0 and 255");
    let max_edit_distance: u8 = args.value_of("max_edit_distance").unwrap().parse::<u8>().expect("Anagram distance should be an integer between 0 and 255");

    let files: Vec<_> = args.values_of("files").unwrap().collect();
    for filename in files {
        let f = File::open(filename).expect(format!("ERROR: Unable to open file {}", filename).as_str());
        let f_buffer = BufReader::new(f);
        for line in f_buffer.lines() {
            if let Ok(line) = line {
                /*
                let variants = model.find_variants(&line, max_anagram_distance, max_edit_distance);
                print!("{}",line);
                for (variant, score) in variants {
                    print!("\t{}\t{}\t",variant, score);
                }
                println!();
                */
            }
        }

    }
}
