extern crate num_bigint;

use std::fs::File;
use std::io::{self, Write,Read,BufReader,BufRead,Error};
use std::collections::{HashMap,BTreeMap};

pub mod types;
pub mod anahash;
pub mod index;
pub mod vocab;
pub mod distance;
pub mod test;


pub use crate::types::*;
pub use crate::anahash::*;
pub use crate::index::*;
pub use crate::vocab::*;
pub use crate::distance::*;


pub struct VariantModel {
    pub decoder: VocabDecoder,
    //pub encoder: VocabEncoder,

    pub alphabet: Alphabet,

    ///The main index, mapping anagrams to instances
    pub index: AnaIndex,

    ///A secondary sorted index
    ///indices of the outer vector correspond to the length of an anagram (in chars)  - 1
    ///Inner vector is always sorted
    pub sortedindex: BTreeMap<u16,Vec<AnaValue>>,

    ///Does the model have frequency information?
    pub have_freq: bool,

    debug: bool
}

impl VariantModel {
    pub fn new(alphabet_file: &str, vocabulary_file: &str, vocabparams: Option<VocabParams>, debug: bool) -> VariantModel {
        let mut model = VariantModel {
            alphabet: Vec::new(),
            //encoder: HashMap::new(),
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

    pub fn alphabet_size(&self) -> CharIndexType {
        self.alphabet.len() as CharIndexType + 1 //+1 for UNK
    }

    pub fn get_or_create_node<'a,'b>(&'a mut self, anahash: &'b AnaValue) -> &'a mut AnaIndexNode {
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

    pub fn train(&mut self) {
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

    /*
    fn compute_deletions(&self, target: &mut HashMap<AnaValue,Vec<AnaValue>>, queue: &[AnaValue], max_distance: u8)  {
        //TODO: REMOVE, redundant
        //
        //
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
                let newparents: Vec<AnaValue> = anahash.iter_parents(alphabet_size).map(|x| x.clone()).collect();
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
    */



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



    pub fn contains_key(&self, key: &AnaValue) -> bool {
        self.index.contains_key(key)
    }


    ///Read the alphabet from a TSV file
    ///The file contains one alphabet entry per line, but may
    ///consist of multiple tab-separated alphabet entries on that line, which
    ///will be treated as the identical.
    ///The alphabet is not limited to single characters but may consist
    ///of longer string, a greedy matching approach will be used so order
    ///matters (but only for this)
    pub fn read_alphabet(&mut self, filename: &str) -> Result<(), std::io::Error> {
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
    pub fn read_vocabulary(&mut self, filename: &str, params: Option<VocabParams>) -> Result<(), std::io::Error> {
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
    pub fn find_variants<'a>(&'a self, s: &str, max_anagram_distance: u8, max_edit_distance: u8) -> Vec<(&'a str, f64)> {

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
    pub fn score_and_resolve(&self, instances: Vec<(VocabId,u8)>, use_freq: bool) -> Vec<(&str,f64)> {
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
    pub fn gather_instances(&self, hashes: &[&AnaValue], querystring: &[u8], max_edit_distance: u8) -> Vec<(VocabId,u8)> {
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
    pub fn find_nearest_anahashes<'a>(&'a self, focus: &AnaValue, max_distance: u8) -> Vec<&'a AnaValue> {
        let mut nearest: Vec<&AnaValue> = Vec::new();

        if self.debug {
            eprintln!("(finding nearest anagram matches for focus anavalue {})", focus);
        }

        if let Some((matched_anahash, _node)) = self.index.get_key_value(focus) {
            //the easiest case, this anahash exists in the model!
            if self.debug {
                eprintln!(" (found exact match)");
            }
            nearest.push(matched_anahash);
        }

        let (focus_alphabet_size, focus_charcount) = focus.alphabet_upper_bound(self.alphabet_size());
        let focus_highest_alphabet_char = AnaValue::character(focus_alphabet_size);


        //Find anagrams reachable through insertions within the the maximum distance
        for distance in 1..=max_distance {
            let mut count = 0;
            let search_charcount = focus_charcount + distance as u16;
            if self.debug {
                eprintln!(" (testing insertion at distance {}, charcount {})", distance, search_charcount);
            }
            if let Some(sortedindex) = self.sortedindex.get(&search_charcount) {
                nearest.extend( sortedindex.iter().filter(|candidate| {
                    if candidate.contains(focus) {//this is where the magic happens
                        count += 1;
                        true
                    } else {
                        false
                    }
                }));
            }
            if self.debug {
                eprintln!(" (found {} candidates)", count);
            }
        }

        /*
        //Compute upper bounds for each of the distances
        let mut av_upper_bounds: Vec<AnaValue>; //indices correspond to distance - 1  (so 0 for AV distance 1)
        let mut av_lower_bounds: Vec<AnaValue>; //indices correspond to distance - 1  (so 0 for AV distance 1)
        let mut upperbound_value = *focus;
        for i in 0..max_distance {
            upperbound_value = upperbound_value.insert(&focus_highest_alphabet_char);
            lowerbound_value = lowerbound_value.delete(&focus_highest_alphabet_char);
            av_upper_bounds.push(upperbound_value);
        }
        */

        // Do a breadth first search for deletions
        for (deletion,distance) in focus.iter_deletions(focus_alphabet_size, Some(max_distance as u32), true) {
            if self.debug {
                eprintln!(" (testing deletion at distance {} for anavalue {})", distance, deletion.value);
            }
            if let Some((matched_anahash, _node)) = self.index.get_key_value(&deletion) {
                if self.debug {
                    eprintln!("  (deletion matches)");
                }
                //This deletion exists in the model
                nearest.push(matched_anahash);
            }

            if distance == max_distance as u32 { //no need to check for distances that are not the max
                let mut count = 0;
                let search_charcount = focus_charcount + distance as u16;
                //Find possible insertions starting from this deletion
                if let Some(sortedindex) = self.sortedindex.get(&search_charcount) {
                    nearest.extend( sortedindex.iter().filter(|candidate| {
                        if candidate.contains(focus) {//this is where the magic happens
                            count += 1;
                            true
                        } else {
                            false
                        }
                    }));
                }
                if self.debug {
                    eprintln!("  (added {} candidates)", count);
                }
            }

        }

        if self.debug {
            eprintln!("(found {} anagram matches in total for focus anavalue {})", nearest.len(), focus);
        }
        nearest
    }


}

