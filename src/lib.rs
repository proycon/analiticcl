extern crate num_bigint;
extern crate sesdiff;

use std::fs::File;
use std::io::{self, Write,Read,BufReader,BufRead,Error,ErrorKind};
use std::collections::{HashMap,HashSet,BTreeMap};
use std::cmp::max;
use sesdiff::{EditScript,EditInstruction,shortest_edit_script};
use std::str::FromStr;

pub mod types;
pub mod anahash;
pub mod index;
pub mod iterators;
pub mod vocab;
pub mod distance;
pub mod confusables;
pub mod test;


pub use crate::types::*;
pub use crate::anahash::*;
pub use crate::index::*;
pub use crate::iterators::*;
pub use crate::vocab::*;
pub use crate::distance::*;
pub use crate::confusables::*;


pub struct VariantModel {
    pub decoder: VocabDecoder,
    pub encoder: VocabEncoder,

    pub alphabet: Alphabet,

    ///The main index, mapping anagrams to instances
    pub index: AnaIndex,

    ///A secondary sorted index
    ///indices of the outer vector correspond to the length of an anagram (in chars)  - 1
    ///Inner vector is always sorted
    pub sortedindex: BTreeMap<u16,Vec<AnaValue>>,

    ///Does the model have frequency information?
    pub have_freq: bool,

    ///Total sum of all frequencies in the lexicon
    pub freq_sum: usize,

    ///Weights used in scoring
    pub weights: Weights,

    /// Stores the names of the loaded lexicons, they will be referenced by index from individual
    /// items for provenance reasons
    lexicons: Vec<String>,

    confusables: Vec<Confusable>,

    debug: bool
}

impl VariantModel {
    pub fn new(alphabet_file: &str, weights: Weights, debug: bool) -> VariantModel {
        let mut model = VariantModel {
            alphabet: Vec::new(),
            encoder: HashMap::new(),
            decoder: Vec::new(),
            index: HashMap::new(),
            sortedindex: BTreeMap::new(),
            have_freq: false,
            freq_sum: 0,
            weights: weights,
            lexicons: Vec::new(),
            confusables: Vec::new(),
            debug: debug,
        };
        model.read_alphabet(alphabet_file).expect("Error loading alphabet file");
        model
    }

    pub fn new_with_alphabet(alphabet: Alphabet, weights: Weights, debug: bool) -> VariantModel {
        VariantModel {
            alphabet: alphabet,
            decoder: Vec::new(),
            encoder: HashMap::new(),
            index: HashMap::new(),
            sortedindex: BTreeMap::new(),
            have_freq: false,
            freq_sum: 0,
            weights: weights,
            lexicons: Vec::new(),
            confusables: Vec::new(),
            debug: debug,
        }
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

    pub fn build(&mut self) {
        if !self.have_freq {
            self.weights.freq = 0.0
        }

        eprintln!("Computing anagram values for all items in the lexicon...");

        let alphabet_size = self.alphabet_size();

        // Hash all strings in the lexicon
        // and add them to the index
        let mut tmp_hashes: Vec<(AnaValue,VocabId)> = Vec::with_capacity(self.decoder.len());
        for (id, value)  in self.decoder.iter().enumerate() {
            //get the anahash
            let anahash = value.text.anahash(&self.alphabet);
            self.freq_sum += value.frequency as usize;
            if self.debug {
                eprintln!("   -- Anavalue={} VocabId={} Text={}", &anahash, id, value.text);
            }
            tmp_hashes.push((anahash, id as VocabId));
        }
        eprintln!(" - Found {} instances",tmp_hashes.len());

        eprintln!("Adding all instances to the index...");
        for (anahash, id) in tmp_hashes {
            //add it to the index
            let node = self.get_or_create_node(&anahash);
            node.instances.push(id);
        }
        eprintln!(" - Found {} anagrams", self.index.len() );

        eprintln!("Creating sorted secondary index...");
        for (anahash, node) in self.index.iter() {
            if !self.sortedindex.contains_key(&node.charcount) {
                self.sortedindex.insert(node.charcount, Vec::new());
            }
            let keys = self.sortedindex.get_mut(&node.charcount).expect("getting sorted index (1)");
            keys.push(anahash.clone());  //TODO: see if we can make this a reference later
        }

        eprintln!("Sorting secondary index...");
        let mut sizes: Vec<u16> = self.sortedindex.keys().map(|x| *x).collect();
        sizes.sort_unstable();
        for size in sizes {
            let keys = self.sortedindex.get_mut(&size).expect("getting sorted index (2)");
            keys.sort_unstable();
            eprintln!(" - Found {} anagrams of length {}", keys.len(), size );
        }
    }

    pub fn contains_key(&self, key: &AnaValue) -> bool {
        self.index.contains_key(key)
    }

    ///Get all anagram instances for a specific entry
    pub fn get_anagram_instances(&self, text: &str) -> Vec<&VocabValue> {
        let anavalue = text.anahash(&self.alphabet);
        let mut instances: Vec<&VocabValue> = Vec::new();
        if let Some(node) = self.index.get(&anavalue) {
            for vocab_id in node.instances.iter() {
                instances.push(self.decoder.get(*vocab_id as usize).expect("vocab from decoder"));
            }
        }
        instances
    }

    ///Get an exact item in the lexicon (if it exists)
    pub fn get(&self, text: &str) -> Option<&VocabValue> {
        for instance in self.get_anagram_instances(text) {
            if instance.text == text {
                return Some(instance);
            }
        }
        None
    }

    ///Tests if the lexicon has a specific entry, by text
    pub fn has(&self, text: &str) -> bool {
        for instance in self.get_anagram_instances(text) {
            if instance.text == text {
                return true;
            }
        }
        false
    }

    ///Resolves a vocabulary ID
    pub fn get_vocab(&self, vocab_id: VocabId) -> Option<&VocabValue> {
        self.decoder.get(vocab_id as usize)
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


    ///Read a confusiblelist from a TSV file
    ///Contains edit scripts in the first columned (formatted in sesdiff style)
    ///and optionally a weight in the second column.
    ///favourable confusables have a weight > 1.0, unfavourable ones are < 1.0 (penalties)
    ///Weight values should be relatively close to 1.0 as they are applied to the entire score
    pub fn read_confusablelist(&mut self, filename: &str) -> Result<(), std::io::Error> {
        if self.debug {
            eprintln!("Reading confusables from {}...", filename);
        }
        let f = File::open(filename)?;
        let f_buffer = BufReader::new(f);
        for line in f_buffer.lines() {
            if let Ok(line) = line {
                if !line.is_empty() {
                    let fields: Vec<&str> = line.split("\t").collect();
                    let weight = if fields.len() >= 2 {
                        fields.get(1).unwrap().parse::<f64>().expect("score should be a float")
                    } else {
                        1.0
                    };
                    self.add_to_confusables(fields.get(0).unwrap(), weight)?;
                }
            }
        }
        if self.debug {
            eprintln!(" -- Read {} confusables", self.confusables.len());
        }
        Ok(())
    }

    pub fn add_to_confusables(&mut self, editscript: &str, weight: f64) -> Result<(), std::io::Error> {
        match EditScript::<String>::from_str(editscript) {
            Ok(editscript) => {
                self.confusables.push(Confusable {
                    editscript: editscript,
                    weight: weight
                });
                Ok(())
            },
            Err(err) => {
                return Err(Error::new(ErrorKind::Other, format!("{:?}",err)))
            }
        }
    }

    ///Read vocabulary (a lexicon or corpus-derived lexicon) from a TSV file
    ///May contain frequency information
    ///The parameters define what value can be read from what column
    pub fn read_vocabulary(&mut self, filename: &str, params: &VocabParams, lexicon_weight: f32) -> Result<(), std::io::Error> {
        if self.debug {
            eprintln!("Reading vocabulary from {}...", filename);
        }
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
                    self.add_to_vocabulary(text, Some(frequency), Some(lexicon_weight), self.lexicons.len() as u8);
                }
            }
        }
        if self.debug {
            eprintln!(" - Read vocabulary of size {}", self.decoder.len());
        }
        self.lexicons.push(filename.to_string());
        Ok(())
    }

    pub fn add_to_vocabulary(&mut self, text: &str, frequency: Option<u32>, lexicon_weight: Option<f32>, lexicon_index: u8) {
        let frequency = frequency.unwrap_or(1);
        let lexicon_weight = lexicon_weight.unwrap_or(1.0);
        if self.debug {
            eprintln!(" -- Adding to vocabulary: {}", text);
        }
        if let Some(vocab_id) = self.encoder.get(text) {
            let item = self.decoder.get_mut(*vocab_id as usize).expect(&format!("Retrieving existing vocabulary entry {}",vocab_id));
            item.frequency += frequency;
            if lexicon_weight > item.lexweight {
                item.lexweight = lexicon_weight;
            }
        } else {
            //item is new
            self.encoder.insert(text.to_string(), self.decoder.len() as u64);
            self.decoder.push(VocabValue {
                text: text.to_string(),
                norm: text.normalize_to_alphabet(&self.alphabet),
                frequency: frequency,
                tokencount: text.chars().filter(|c| *c == ' ').count() as u8 + 1,
                lexweight: lexicon_weight,
                lexindex: lexicon_index,
            });
        }
    }

    /// Find variants in the vocabulary for a given string (in its totality), returns a vector of vocabulaly ID and score pairs
    /// The resulting vocabulary Ids can be resolved through `get_vocab()`
    pub fn find_variants(&self, input: &str, max_anagram_distance: u8, max_edit_distance: u8, max_matches: usize) -> Vec<(VocabId, f64)> {

        //Compute the anahash
        let normstring = input.normalize_to_alphabet(&self.alphabet);
        let anahash = input.anahash(&self.alphabet);

        //Compute neighbouring anahashes and find the nearest anahashes in the model
        let anahashes = self.find_nearest_anahashes(&anahash, max_anagram_distance);

        //Get the instances pertaining to the collected hashes, within a certain maximum distance
        //and compute distances
        let variants = self.gather_instances(&anahashes, &normstring, max_edit_distance);

        self.score_and_rank(variants, input, max_matches)
    }


    /// Find the nearest anahashes that exists in the model (computing anahashes in the
    /// neigbhourhood if needed). Note: this also returns anahashes that have no instances
    pub fn find_nearest_anahashes<'a>(&'a self, focus: &AnaValue, max_distance: u8) -> HashSet<&'a AnaValue> {
        let mut nearest: HashSet<&AnaValue> = HashSet::new();

        if self.debug {
            eprintln!("(finding nearest anagram matches for focus anavalue {})", focus);
        }

        if let Some((matched_anahash, _node)) = self.index.get_key_value(focus) {
            //the easiest case, this anahash exists in the model!
            if self.debug {
                eprintln!(" (found exact match)");
            }
            nearest.insert(matched_anahash);
        }

        let (focus_upper_bound, focus_charcount) = focus.alphabet_upper_bound(self.alphabet_size());
        let focus_alphabet_size = focus_upper_bound + 1;
        let focus_highest_alphabet_char = AnaValue::character(focus_upper_bound);


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
        for (deletion,distance) in focus.iter_recursive(focus_alphabet_size+1, &SearchParams {
            max_distance: Some(max_distance as u32),
            breadthfirst: true,
            allow_empty_leaves: false,
            allow_duplicates: false,
            ..Default::default()
        }) {
            if self.debug {
                eprintln!(" (testing deletion at distance {} for anavalue {})", distance, deletion.value);
            }
            if let Some((matched_anahash, _node)) = self.index.get_key_value(&deletion) {
                if self.debug {
                    eprintln!("  (deletion matches)");
                }
                //This deletion exists in the model
                nearest.insert(matched_anahash);
            }

            if distance == max_distance as u32 { //no need to check for distances that are not the max
                let mut count = 0;
                let (deletion_upper_bound, deletion_charcount) = deletion.alphabet_upper_bound(self.alphabet_size());
                let search_charcount = deletion_charcount + distance as u16;
                let beginlength = nearest.len();
                if self.debug {
                    eprintln!("  (testing insertions for distance {} from deletion result anavalue {})", search_charcount, deletion.value);
                }
                //Find possible insertions starting from this deletion
                if let Some(sortedindex) = self.sortedindex.get(&search_charcount) {
                    nearest.extend( sortedindex.iter().filter(|candidate| {
                        if candidate.contains(&deletion.value) {//this is where the magic happens
                            count += 1;
                            true
                        } else {
                            false
                        }
                    }));
                }
                if self.debug {
                    eprintln!("  (added {} out of {} candidates, preventing duplicates)", nearest.len() - beginlength , count);
                }
            }

        }

        if self.debug {
            eprint!("(found {} anagram matches in total for focus anavalue {}: ", nearest.len(), focus);
            for av in nearest.iter() {
                eprint!(" {}", av);
            }
            eprintln!(")");
        }
        nearest
    }


    /// Gather instances and their edit distances, given a search string (normalised to the alphabet) and anagram hashes
    pub fn gather_instances(&self, nearest_anagrams: &HashSet<&AnaValue>, querystring: &[u8], max_edit_distance: u8) -> Vec<(VocabId,Distance)> {
        let mut found_instances = Vec::new();
        let mut pruned_instances = 0;
        for anahash in nearest_anagrams {
            if let Some(node) = self.index.get(anahash) {
                for vocab_id in node.instances.iter() {
                    if let Some(vocabitem) = self.decoder.get(*vocab_id as usize) {
                        if let Some(ld) = damerau_levenshtein(querystring, &vocabitem.norm, max_edit_distance) {
                            //we only get here if we make the max_edit_distance cut-off
                            let distance = Distance {
                                ld: ld,
                                lcs: if self.weights.lcs > 0.0 { longest_common_substring_length(querystring, &vocabitem.norm) } else { 0 },
                                prefixlen: if self.weights.prefix > 0.0 { common_prefix_length(querystring, &vocabitem.norm) } else { 0 },
                                suffixlen: if self.weights.suffix > 0.0 { common_suffix_length(querystring, &vocabitem.norm) } else { 0 },
                                freq: if self.weights.freq > 0.0 { vocabitem.frequency } else { 0 },
                                lex: if self.weights.lex > 0.0 { vocabitem.lexweight } else { 0.0 }
                            };
                            found_instances.push((*vocab_id,distance));
                        } else {
                            pruned_instances += 1;
                        }
                    }
                }
            }
        }
        //found_instances.sort_unstable_by_key(|k| k.1 ); //sort by distance, ascending order
        if self.debug {
            eprintln!("(found {} instances (pruned {}) over {} anagrams)", found_instances.len(), pruned_instances, nearest_anagrams.len());
        }
        found_instances
    }



    /// Rank and score all variants
    pub fn score_and_rank(&self, instances: Vec<(VocabId,Distance)>, input: &str, max_matches: usize ) -> Vec<(VocabId,f64)> {
        let mut results: Vec<(VocabId,f64)> = Vec::new();
        let mut max_distance = 0;
        let mut max_freq = 0;
        let mut max_prefixlen = 0;
        let mut max_suffixlen = 0;
        let weights_sum = self.weights.sum();

        if self.debug {
            eprintln!("(scoring and ranking {} instances)", instances.len());
        }

        //Collect maximum values
        for (vocab_id, distance) in instances.iter() {
            if distance.ld > max_distance {
                max_distance = distance.ld;
            }
            if distance.prefixlen > max_prefixlen {
                max_prefixlen = distance.prefixlen;
            }
            if distance.suffixlen > max_suffixlen {
                max_suffixlen = distance.suffixlen;
            }
            if self.have_freq {
                if let Some(vocabitem) = self.decoder.get(*vocab_id as usize) {
                    if vocabitem.frequency > max_freq {
                        max_freq = vocabitem.frequency;
                    }
                }
            }
        }

        //Compute scores
        for (vocab_id, distance) in instances.iter() {
            if let Some(vocabitem) = self.decoder.get(*vocab_id as usize) {
                let distance_score: f64 = 1.0 - (distance.ld as f64 / max_distance as f64);
                let lcs_score: f64 = distance.lcs as f64 / vocabitem.norm.len() as f64;
                let prefix_score: f64 = match max_prefixlen {
                    0 => 0.0,
                    max_prefixlen => distance.prefixlen as f64 / max_prefixlen as f64
                };
                let suffix_score: f64 = match max_suffixlen {
                    0 => 0.0,
                    max_suffixlen => distance.suffixlen as f64 / max_suffixlen as f64
                };
                let freq_score: f64 = if self.have_freq {
                   vocabitem.frequency as f64 / max_freq as f64
                } else {
                    1.0
                };
                let score = (
                    self.weights.ld * distance_score +
                    self.weights.freq * freq_score +  //weight will be 0 if there are no frequencies
                    self.weights.lcs * lcs_score +
                    self.weights.prefix * prefix_score +
                    self.weights.suffix * suffix_score +
                    self.weights.lex * vocabitem.lexweight as f64
                ) / weights_sum;
                if self.debug {
                    eprintln!("   (distance={:?}, score={})", distance, score);
                }
                results.push( (*vocab_id, score) );
            }
        }

        //Sort the results
        results.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); //sort by score, descending order



        //Crop the results at max_matches
        if max_matches > 0 && results.len() > max_matches {
            let last_score = results.get(max_matches - 1).expect("get last score").1;
            let cropped_score = results.get(max_matches).expect("get cropped score").1;
            if cropped_score < last_score {
                if self.debug {
                    eprintln!("   (truncating {} matches to {})", results.len(), max_matches);
                }
                //simplest case, crop at the max_matches
                results.truncate(max_matches);
            } else {
                //cropping at max_matches comes at arbitrary of equal scoring items,
                //we crop earlier instead:
                let mut early_cutoff = 0;
                let mut late_cutoff = 0;
                for (i, result) in results.iter().enumerate() {
                    if result.1 == cropped_score && early_cutoff == 0 {
                        early_cutoff = i;
                    }
                    if result.1 < cropped_score {
                        late_cutoff = i;
                        break;
                    }
                }
                if early_cutoff > 0 {
                    if self.debug {
                        eprintln!("   (truncating {} matches (early) to {})", results.len(), early_cutoff+1);
                    }
                    results.truncate(early_cutoff+1);
                } else if late_cutoff > 0 {
                    if self.debug {
                        eprintln!("   (truncating {} matches (late) to {})", results.len(), late_cutoff+1);
                    }
                    results.truncate(late_cutoff+1);
                }
            }
        }

        //rescore with confusable weights
        if !self.confusables.is_empty() {
            if self.debug {
                eprintln!("   (rescoring with confusable weights)");
            }
            for (vocab_id, score) in results.iter_mut() {
                *score *= self.compute_confusable_weight(input, *vocab_id);
            }
            results.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); //sort by score, descending order
        }

        results
    }

    /// compute weight over known confusables
    /// Should return 1.0 when there are no known confusables
    /// < 1.0 when there are unfavourable confusables
    /// > 1.0 when there are favourable confusables
    pub fn compute_confusable_weight(&self, input: &str, candidate: VocabId) -> f64 {
        let mut weight = 1.0;
        if let Some(candidate) = self.decoder.get(candidate as usize) {
            let editscript = shortest_edit_script(input, &candidate.text, false, false, false);
            for confusable in self.confusables.iter() {
                if confusable.found_in(&editscript) {
                    if self.debug {
                        eprintln!("   (input {} with candidate {} instantiates {:?})", input, &candidate.text, confusable);
                    }
                    weight *= confusable.weight;
                }
            }
        }
        weight
    }


    ///Adds the input item to the reverse index, as instantiation of the given vocabulary id
    pub fn add_to_reverse_index(&self, reverseindex: &mut ReverseIndex, input: &str, matched_vocab_id: VocabId, score: f64) {
        let variant = match self.encoder.get(input) {
            Some(known_vocab_id) => {
                if *known_vocab_id == matched_vocab_id {
                    //item is an exact match, add all
                    return;
                }
                Variant::Known(*known_vocab_id)
            },
            _ => Variant::Unknown(input.to_string())
        };
        if self.debug {
            eprintln!("   (adding variant {:?} to reverse index for match {})", variant, matched_vocab_id);
        }
        if let Some(existing_variants) = reverseindex.get_mut(&matched_vocab_id) {
            existing_variants.push((variant,score));
        } else {
            reverseindex.insert(matched_vocab_id, vec!((variant, score)));
        }
    }


}
