extern crate ibig;
extern crate num_traits;
extern crate sesdiff;
extern crate rayon;
extern crate rustfst;

use std::fs::File;
use std::io::{BufReader,BufRead};
use std::collections::{HashMap,HashSet,BTreeMap};
use std::cmp::min;
use sesdiff::shortest_edit_script;
use std::time::SystemTime;
use rayon::prelude::*;
use rustfst::prelude::*;

pub mod types;
pub mod anahash;
pub mod index;
pub mod iterators;
pub mod vocab;
pub mod distance;
pub mod confusables;
pub mod cache;
pub mod search;
pub mod test;


pub use crate::types::*;
pub use crate::anahash::*;
pub use crate::index::*;
pub use crate::iterators::*;
pub use crate::vocab::*;
pub use crate::distance::*;
pub use crate::confusables::*;
pub use crate::cache::*;
pub use crate::search::*;


/// The VariantModel is the most high-level model of analiticcl, it holds
/// all data required for variant matching.
pub struct VariantModel {
    /// Maps Vocabulary IDs to their textual strings and other related properties
    pub decoder: VocabDecoder,

    /// Map strings to vocabulary IDs
    pub encoder: VocabEncoder,

    /// Defines the alphabet used for the variant model
    pub alphabet: Alphabet,

    ///The main index, mapping anagrams to instances
    pub index: AnaIndex,

    ///A secondary sorted index
    ///indices of the outer vector correspond to the length of an anagram (in chars)  - 1
    ///Inner vector is always sorted
    pub sortedindex: BTreeMap<u16,Vec<AnaValue>>,

    /// Joint n-gram probabilities, used for simple context-sensitive language modelling
    /// when finding the most probable sequence of variants
    pub ngrams: HashMap<NGram,u32>,

    ///Total frequency, index corresponds to n-1 size, so this holds the total count for unigrams, bigrams, etc.
    pub freq_sum: Vec<usize>,

    pub have_freq: bool,

    ///Weights used in scoring
    pub weights: Weights,

    /// Stores the names of the loaded lexicons, they will be referenced by index from individual
    /// items for provenance reasons
    pub lexicons: Vec<String>,

    /// Holds weighted confusable recipes that can be used in scoring and ranking
    pub confusables: Vec<Confusable>,

    ///Process confusables before pruning by max_matches
    pub confusables_before_pruning: bool,

    /// Groups clusters of variants (either from explicitly loaded variant files or in a later
    /// stage perhaps also computed)
    pub variantclusters: VariantClusterMap,

    pub debug: bool
}

impl VariantModel {
    /// Instantiate a new variant model
    pub fn new(alphabet_file: &str, weights: Weights, debug: bool) -> VariantModel {
        let mut model = VariantModel {
            alphabet: Vec::new(),
            encoder: HashMap::new(),
            decoder: Vec::new(),
            index: HashMap::new(),
            sortedindex: BTreeMap::new(),
            ngrams: HashMap::new(),
            freq_sum: vec!(0),
            have_freq: false,
            weights,
            lexicons: Vec::new(),
            confusables: Vec::new(),
            confusables_before_pruning: false,
            variantclusters: HashMap::new(),
            debug,
        };
        model.read_alphabet(alphabet_file).expect("Error loading alphabet file");
        model
    }

    /// Instantiate a new variant model, explicitly passing an alphabet rather than loading one
    /// from file.
    pub fn new_with_alphabet(alphabet: Alphabet, weights: Weights, debug: bool) -> VariantModel {
        VariantModel {
            alphabet: alphabet,
            decoder: Vec::new(),
            encoder: HashMap::new(),
            index: HashMap::new(),
            sortedindex: BTreeMap::new(),
            ngrams: HashMap::new(),
            freq_sum: vec!(0),
            have_freq: false,
            weights,
            lexicons: Vec::new(),
            confusables: Vec::new(),
            confusables_before_pruning: false,
            variantclusters: HashMap::new(),
            debug,
        }
    }

    /// Configure the model to match against known confusables prior to pruning on maximum weight.
    /// This may lead to better results but may have a significant performance impact.
    pub fn set_confusables_before_pruning(&mut self) {
        self.confusables_before_pruning = true;
    }

    /// Returns the size of the alphabet, this is typically +1 longer than the actual alphabet file
    /// as it includes the UNKNOWN symbol.
    pub fn alphabet_size(&self) -> CharIndexType {
        self.alphabet.len() as CharIndexType + 1 //+1 for UNK
    }

    /// Get an item from the index or insert it if it doesn't exist yet
    pub fn get_or_create_index<'a,'b>(&'a mut self, anahash: &'b AnaValue) -> &'a mut AnaIndexNode {
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

    /// Build the anagram index (and secondary index) so the model
    /// is ready for variant matching
    pub fn build(&mut self) {
        if !self.have_freq {
            self.weights.freq = 0.0
        }

        eprintln!("Computing anagram values for all items in the lexicon...");


        //extra unigrams extracted from n-grams that need to be added to the vocabulary decoder
        let mut parts: Vec<String> = Vec::new();

        // Hash all strings in the lexicon
        // and add them to the index
        let mut tmp_hashes: Vec<(AnaValue,VocabId)> = Vec::with_capacity(self.decoder.len());
        for (id, value)  in self.decoder.iter().enumerate() {
            //get the anahash
            let anahash = value.text.anahash(&self.alphabet);
            let ngram_order = value.text.chars().filter(|c| *c == ' ').count();
            if ngram_order > 1 {
                //decompose the ngram into its parts

                //reserve the space for the total counts
                for _ in self.freq_sum.len()..ngram_order {
                    self.freq_sum.push(0);
                }
                //add to the totals for this order of ngrams
                self.freq_sum[ngram_order-1] += value.frequency as usize;

                //ensure all individual components of the ngram are in the vocabulary
                //encoder/decoder, they do NOT have to be in the main lexicon index necessarily
                //we need them mainly for computing transition probabilities
                let mut ngram = NGram::new();
                for part in value.text.split(" ") {
                    if let Some(part_id) = self.encoder.get(part) {
                        ngram.push(*part_id);
                    } else {
                        let part_id: VocabId = self.decoder.len() as VocabId + parts.len() as VocabId;
                        parts.push(part.to_string());
                        self.encoder.insert(part.to_string(), part_id);
                        ngram.push(part_id);
                    }
                }

                if let Some(ngram) = self.ngrams.get_mut(&ngram) {
                    //update the count for this ngram
                    *ngram += value.frequency;
                } else {
                    //add the new ngram
                    self.ngrams.insert( ngram, value.frequency );
                }
            } else {
                self.freq_sum[0] += value.frequency as usize;
            }
            if self.debug {
                eprintln!("   -- Anavalue={} VocabId={} Text={}", &anahash, id, value.text);
            }
            tmp_hashes.push((anahash, id as VocabId));
        }
        eprintln!(" - Found {} instances",tmp_hashes.len());

        //add collected n-gram parts to the decoder (as stubs because
        //they're not in a lexicon)
        for part in parts {
            self.decoder.push(VocabValue::new_stub(part));
        }

        eprintln!("Adding all instances to the index...");
        for (anahash, id) in tmp_hashes {
            //add it to the index
            let node = self.get_or_create_index(&anahash);
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

    /// Tests if the anagram value exists in the index
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
                    self.alphabet.push(line.split("\t").map(|x|
                            match x {
                                "\\s" => " ".to_owned(),
                                "\\t" => "\t".to_owned(),
                                "\\n" => "\n".to_owned(),
                                _ => x.to_owned()
                            }).collect());
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

    /// Add a confusable
    pub fn add_to_confusables(&mut self, editscript: &str, weight: f64) -> Result<(), std::io::Error> {
        let confusable = Confusable::new(editscript, weight)?;
        self.confusables.push(confusable);
        Ok(())
    }

    ///Read vocabulary (a lexicon or corpus-derived lexicon) from a TSV file
    ///May contain frequency information
    ///The parameters define what value can be read from what column
    pub fn read_vocabulary(&mut self, filename: &str, params: &VocabParams, lexicon_weight: f32) -> Result<(), std::io::Error> {
        if self.debug {
            eprintln!("Reading vocabulary from {}...", filename);
        }
        let beginlen = self.decoder.len();
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
            eprintln!(" - Read vocabulary of size {}", self.decoder.len() - beginlen);
        }
        self.lexicons.push(filename.to_string());
        Ok(())
    }

    ///Read a variants list of equally weighted variants from a TSV file
    ///Each line simply contains tab-separated variants and all entries on a single line are
    ///considered variants. Consumed much less memory than weighted variants.
    pub fn read_variants(&mut self, filename: &str, lexicon_weight: f32) -> Result<(), std::io::Error> {
        if self.debug {
            eprintln!("Reading variants from {}...", filename);
        }
        let beginlen = self.variantclusters.len();
        let f = File::open(filename)?;
        let f_buffer = BufReader::new(f);
        for line in f_buffer.lines() {
            if let Ok(line) = line {
                if !line.is_empty() {
                    let variants: Vec<&str> = line.split("\t").collect();
                    let mut ids: Vec<VocabId> = Vec::new();
                    let clusterid = self.variantclusters.len() as VariantClusterId;
                    for variant in variants.iter() {
                        //all variants by definition are added to the combined lexicon
                        let variantid = self.add_to_vocabulary(variant, None, Some(lexicon_weight), self.lexicons.len() as u8);
                        ids.push(variantid);
                        if let Some(vocabvalue) = self.decoder.get_mut(variantid as usize) {
                            let variantref = VariantReference::VariantCluster(clusterid);
                            if vocabvalue.variants.is_none() {
                                vocabvalue.variants = Some(vec!(variantref));
                            } else if let Some(variantrefs) = vocabvalue.variants.as_mut() {
                                if !variantrefs.contains(&variantref) {
                                    variantrefs.push(variantref);
                                }
                            }
                        }
                    }
                    self.variantclusters.insert(clusterid, ids);
                }
            }
        }
        if self.debug {
            eprintln!(" - Read variants list, added {} new variant clusters", self.variantclusters.len() - beginlen);
        }
        self.lexicons.push(filename.to_string());
        Ok(())
    }

    ///Read a weighted variant list from a TSV file. Contains a canonical/reference form in the
    ///first column, and variants with score (two columns) in the following columns. Consumes much more
    ///memory than equally weighted variants.
    pub fn read_weighted_variants(&mut self, filename: &str, lexicon_weight: f32, intermediate: bool) -> Result<(), std::io::Error> {
        if self.debug {
            eprintln!("Reading variants from {}...", filename);
        }
        let mut count = 0;
        let f = File::open(filename)?;
        let f_buffer = BufReader::new(f);
        for line in f_buffer.lines() {
            if let Ok(line) = line {
                if !line.is_empty() {
                    let fields: Vec<&str> = line.split("\t").collect();

                    let reference = fields.get(0).expect("first item");
                    let ref_id = self.add_to_vocabulary(reference, None, Some(lexicon_weight), self.lexicons.len() as u8);
                    let mut iter = fields.iter();

                    while let (Some(variant), Some(score)) = (iter.next(), iter.next()) {
                        let score = score.parse::<f64>().expect("Scores must be a floating point value");
                        //all variants by definition are added to the lexicon
                        let variantid = self.add_to_vocabulary(variant, None, Some(lexicon_weight), self.lexicons.len() as u8);
                        if variantid != ref_id {
                            if let Some(vocabvalue) = self.decoder.get_mut(ref_id as usize) {
                                let variantref = VariantReference::WeightedVariant((variantid,score) );
                                vocabvalue.intermediate = intermediate;
                                if vocabvalue.variants.is_none() {
                                    vocabvalue.variants = Some(vec!(variantref));
                                    count += 1;
                                } else if let Some(variantrefs) = vocabvalue.variants.as_mut() {
                                    if !variantrefs.contains(&variantref) {
                                        variantrefs.push(variantref);
                                        count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if self.debug {
            eprintln!(" - Read weighted variants list, added {} references", count);
        }
        self.lexicons.push(filename.to_string());
        Ok(())
    }



    /// Adds an entry in the vocabulary
    pub fn add_to_vocabulary(&mut self, text: &str, frequency: Option<u32>, lexicon_weight: Option<f32>, lexicon_index: u8) -> VocabId {
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
                item.lexindex = lexicon_index;
            }
            *vocab_id
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
                variants: None,
                intermediate: false,
            });
            self.decoder.len() as VocabId - 1
        }
    }

    /// Find variants in the vocabulary for a given string (in its totality), returns a vector of vocabulaly ID and score pairs
    /// The resulting vocabulary Ids can be resolved through `get_vocab()`
    pub fn find_variants(&self, input: &str, max_anagram_distance: u8, max_edit_distance: u8, max_matches: usize, score_threshold: f64, stop_criterion: StopCriterion, cache: Option<&mut Cache>) -> Vec<(VocabId, f64)> {

        //Compute the anahash
        let normstring = input.normalize_to_alphabet(&self.alphabet);
        let anahash = input.anahash(&self.alphabet);

        //dynamically computed maximum distance, this will override max_edit_distance
        //when the number is smaller (for short input strings)
        let max_dynamic_distance: u8 = (normstring.len() as f64 / 2.0).floor() as u8;

        //Compute neighbouring anahashes and find the nearest anahashes in the model
        let anahashes = self.find_nearest_anahashes(&anahash, &normstring,
                                                    min(max_anagram_distance, max_dynamic_distance),
                                                    stop_criterion,
                                                    if let Some(cache) = cache {
                                                       Some(&mut cache.visited)
                                                    } else {
                                                       None
                                                    });

        //Get the instances pertaining to the collected hashes, within a certain maximum distance
        //and compute distances
        let variants = self.gather_instances(&anahashes, &normstring, input, min(max_edit_distance, max_dynamic_distance));

        self.score_and_rank(variants, input, max_matches, score_threshold)
    }


    /// Find the nearest anahashes that exists in the model (computing anahashes in the
    /// neigbhourhood if needed).
    pub fn find_nearest_anahashes<'a>(&'a self, focus: &AnaValue, normstring: &Vec<u8>, max_distance: u8,  stop_criterion: StopCriterion, cache: Option<&mut HashSet<AnaValue>>) -> HashSet<&'a AnaValue> {
        let mut nearest: HashSet<&AnaValue> = HashSet::new();

        let begintime = if self.debug {
            eprintln!("(finding nearest anagram matches for focus anavalue {})", focus);
            Some(SystemTime::now())
        } else {
            None
        };

        if let Some((matched_anahash, node)) = self.index.get_key_value(focus) {
            //the easiest case, this anahash exists in the model!
            if self.debug {
                eprintln!(" (found exact match)");
            }
            nearest.insert(matched_anahash);
            if stop_criterion.stop_at_exact_match() {
                for vocab_id in node.instances.iter() {
                    if let Some(value) = self.decoder.get(*vocab_id as usize) {
                        if &value.norm == normstring {
                            if self.debug {
                                eprintln!(" (stopping early)");
                            }
                            return nearest;
                        }
                    }
                }
            }
        }

        let (focus_upper_bound, focus_charcount) = focus.alphabet_upper_bound(self.alphabet_size());
        let focus_alphabet_size = focus_upper_bound + 1;
        let highest_alphabet_char = AnaValue::character(self.alphabet_size()+1);


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

        //Compute upper bounds and lower bounds for each of the distances
        let mut av_upper_bounds: Vec<AnaValue> = Vec::new(); //indices correspond to distance - 1  (so 0 for AV distance 1)
        let mut av_lower_bounds: Vec<AnaValue> = Vec::new(); //indices correspond to distance - 1  (so 0 for AV distance 1)
        let mut upperbound_value: AnaValue = AnaValue::empty();
        let mut lowerbound_value: AnaValue = AnaValue::empty();
        let mut lowerbound_highest_alphabet_char = AnaValue::character(focus_upper_bound);
        let mut lowerbound_alphabet_size = focus_alphabet_size;
        for i in 0..max_distance {
            upperbound_value = if i == 0 {
                focus.insert(&highest_alphabet_char)
            } else {
                upperbound_value.insert(&highest_alphabet_char)
            };
            lowerbound_value = if i == 0 {
                focus.delete(&lowerbound_highest_alphabet_char).unwrap_or(AnaValue::empty())
            } else {
                lowerbound_value.delete(&lowerbound_highest_alphabet_char).unwrap_or(AnaValue::empty())
            };
            let x = lowerbound_value.alphabet_upper_bound(lowerbound_alphabet_size);
            lowerbound_highest_alphabet_char = AnaValue::character(x.0);
            lowerbound_alphabet_size = x.1 as u8 + 1;
            av_upper_bounds.push(upperbound_value.clone());
            av_lower_bounds.push(lowerbound_value.clone());
        }
        let av_upper_bounds: Vec<AnaValue> = av_upper_bounds;
        let av_lower_bounds: Vec<AnaValue> = av_lower_bounds;

        if self.debug {
            eprintln!(" (Computed upper bounds: {:?})", av_upper_bounds);
            eprintln!(" (Computed lower bounds: {:?})", av_lower_bounds);
        }

        let mut lastdistance = 0;
        let searchparams = SearchParams {
            max_distance: Some(max_distance as u32),
            breadthfirst: true,
            allow_empty_leaves: false,
            allow_duplicates: false,
            ..Default::default()
        };


        let iterator = if let Some(cache) = cache {
            focus.iter_recursive_external_cache(focus_alphabet_size+1, &searchparams, cache)
        } else {
            focus.iter_recursive(focus_alphabet_size+1, &searchparams)
        };

        // Do a breadth first search for deletions
        for (deletion,distance) in iterator {
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

            if stop_criterion.iterative() > 0 && lastdistance < distance {
                //have we gathered enough candidates already?
                if nearest.len() >= stop_criterion.iterative() {
                    if self.debug {
                        eprintln!("  (stopping early after distance {}, we have enough matches)", lastdistance);
                    }
                    break;
                }
            }

            if stop_criterion.iterative() > 0 || distance == max_distance as u32 { //no need to check for distances that are not the max
                let mut count = 0;
                let (_deletion_upper_bound, deletion_charcount) = deletion.alphabet_upper_bound(self.alphabet_size());
                let search_charcount = deletion_charcount + distance as u16;
                let beginlength = nearest.len();
                if self.debug {
                    eprintln!("  (testing insertions for distance {} from deletion result anavalue {})", search_charcount, deletion.value);
                }
                //Find possible insertions starting from this deletion
                if let Some(sortedindex) = self.sortedindex.get(&search_charcount) {
                    for candidate in sortedindex.iter() {
                        if candidate > &av_upper_bounds[distance as usize -1] {
                            break;
                        } else if candidate >= &av_lower_bounds[distance as usize - 1] {
                            if candidate.contains(&deletion.value) {//this is where the magic happens
                                count += 1;
                                nearest.insert(candidate);
                            }
                        }
                    }

                    /*
                    nearest.extend( sortedindex.iter().filter(|candidate| {
                        if candidate.contains(&deletion.value) {//this is where the magic happens
                            count += 1;
                            true
                        } else {
                            false
                        }
                    }));*/
                }
                if self.debug {
                    eprintln!("  (added {} out of {} candidates, preventing duplicates)", nearest.len() - beginlength , count);
                }
            }
            lastdistance = distance;
        }

        if self.debug {
            let endtime = SystemTime::now();
            let duration = endtime.duration_since(begintime.expect("begintime")).expect("clock can't go backwards").as_micros();
            eprint!("(found {} anagram matches in total (in {} μs) for focus anavalue {}: ", nearest.len(), duration, focus);
            for av in nearest.iter() {
                eprint!(" {}", av);
            }
            eprintln!(")");
        }
        nearest
    }


    /// Gather instances and their edit distances, given a search string (normalised to the alphabet) and anagram hashes
    pub fn gather_instances(&self, nearest_anagrams: &HashSet<&AnaValue>, querystring: &[u8], query: &str, max_edit_distance: u8) -> Vec<(VocabId,Distance)> {
        let mut found_instances = Vec::new();
        let mut pruned_instances = 0;

        let begintime = if self.debug {
            Some(SystemTime::now())
        } else {
            None
        };

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
                                lex: if self.weights.lex > 0.0 { vocabitem.lexweight } else { 0.0 },
                                samecase: if self.weights.case > 0.0 { vocabitem.text.chars().next().expect("first char").is_lowercase() == query.chars().next().expect("first char").is_lowercase() } else { true },
                                prescore: None,
                            };
                            //match will be added to found_instances at the end of the block (we
                            //need to borrow the distance for a bit still)

                            //Does this vocabulary item make explicit references to variants?
                            //If so, we add those too. This is only the case if the user loaded
                            //variantlists/error lists.
                            if let Some(variantrefs) = &vocabitem.variants {
                                for variantref in variantrefs.iter() {
                                    match variantref {
                                        VariantReference::VariantCluster(cluster_id) => {
                                            if let Some(variants) = self.variantclusters.get(cluster_id) {
                                                //add all variants in the cluster
                                                for variant_id in variants.iter() {
                                                    //we clone do not recompute the distance to the
                                                    //variant, all variants are considered of
                                                    //equal-weight, we use the originally computed
                                                    //distance:
                                                    found_instances.push((*variant_id, distance.clone()));
                                                }
                                            }
                                        },
                                        VariantReference::WeightedVariant((vocab_id, score)) => {
                                            let mut variantdistance = distance.clone();
                                            variantdistance.prescore = Some(*score);
                                            found_instances.push((*vocab_id,variantdistance));
                                        }
                                    }
                                }
                            }

                            //add the original match
                            if !vocabitem.intermediate {
                                found_instances.push((*vocab_id,distance));
                            }
                        } else {
                            pruned_instances += 1;
                        }
                    }
                }
            }
        }
        //found_instances.sort_unstable_by_key(|k| k.1 ); //sort by distance, ascending order
        if self.debug {
            let endtime = SystemTime::now();
            let duration = endtime.duration_since(begintime.expect("begintime")).expect("clock can't go backwards").as_micros();
            eprintln!("(found {} instances (pruned {}) over {} anagrams in {} μs)", found_instances.len(), pruned_instances, nearest_anagrams.len(), duration);
        }
        found_instances
    }



    /// Rank and score all variants
    pub fn score_and_rank(&self, instances: Vec<(VocabId,Distance)>, input: &str, max_matches: usize, score_threshold: f64 ) -> Vec<(VocabId,f64)> {
        let mut results: Vec<(VocabId,f64)> = Vec::new();
        let mut max_distance = 0;
        let mut max_freq = 0;
        let mut max_prefixlen = 0;
        let mut max_suffixlen = 0;
        let weights_sum = self.weights.sum();

        let begintime = if self.debug {
            eprintln!("(scoring and ranking {} instances)", instances.len());
            Some(SystemTime::now())
        } else {
            None
        };

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
                let distance_score: f64 = if max_distance == 0 {
                    0.0
                } else {
                    1.0 - (distance.ld as f64 / max_distance as f64)
                };
                let lcs_score: f64 = distance.lcs as f64 / vocabitem.norm.len() as f64;
                let prefix_score: f64 = match max_prefixlen {
                    0 => 0.0,
                    max_prefixlen => distance.prefixlen as f64 / max_prefixlen as f64
                };
                let suffix_score: f64 = match max_suffixlen {
                    0 => 0.0,
                    max_suffixlen => distance.suffixlen as f64 / max_suffixlen as f64
                };
                let freq_score: f64 = if self.have_freq && max_freq > 0 {
                   vocabitem.frequency as f64 / max_freq as f64
                } else {
                    1.0
                };
                let mut score = (
                    self.weights.ld * distance_score +
                    self.weights.freq * freq_score +  //weight will be 0 if there are no frequencies
                    self.weights.lcs * lcs_score +
                    self.weights.prefix * prefix_score +
                    self.weights.suffix * suffix_score +
                    self.weights.lex * vocabitem.lexweight as f64 +
                    if distance.samecase { self.weights.case } else { 0.0 }
                ) / weights_sum;
                if let Some(prescore) = distance.prescore {
                    //variant is already pre-scored (it comes from an explicit weighted variant list), take the prescore into consideration:
                    score = (score + prescore) / 2.0;
                }
                if score.is_nan() {
                    //should never happen
                    panic!("Invalid score (NaN) computed for variant={}, distance={:?}, score={}", vocabitem.text, distance, score);
                }
                if score >= score_threshold {
                    results.push( (*vocab_id, score) );
                    if self.debug {
                        eprintln!("   (variant={}, distance={:?}, score={})", vocabitem.text, distance, score);
                    }
                } else {
                    if self.debug {
                        eprintln!("   (PRUNED variant={}, distance={:?}, score={})", vocabitem.text, distance, score);
                    }
                }
            }
        }

        //rescore with confusable weights (EARLY)
        if !self.confusables.is_empty() && self.confusables_before_pruning {
            self.rescore_confusables(&mut results, input);
        }

        //Sort the results
        results.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).expect(format!("partial cmp of {} and {}",a.1,b.1).as_str())); //sort by score, descending order



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

        //rescore with confusable weights (LATE, default)
        if !self.confusables.is_empty() && !self.confusables_before_pruning {
            self.rescore_confusables(&mut results, input);
        }

        if self.debug {
            let endtime = SystemTime::now();
            let duration = endtime.duration_since(begintime.expect("begintime")).expect("clock can't go backwards").as_micros();
            eprintln!(" (scored and ranked {} results in {} μs)", results.len(), duration);
        }

        results
    }

    /// Rescores the scored variants by testing against known confusables
    pub fn rescore_confusables(&self, results: &mut Vec<(VocabId,f64)>, input: &str) {
        if self.debug {
            eprintln!("   (rescoring with confusable weights)");
        }
        for (vocab_id, score) in results.iter_mut() {
            *score *= self.compute_confusable_weight(input, *vocab_id);
        }
        results.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).expect(format!("partial cmp of {} and {}",a.1,b.1).as_str())); //sort by score, descending order
    }

    /// compute weight over known confusables
    /// Should return 1.0 when there are no known confusables
    /// < 1.0 when there are unfavourable confusables
    /// > 1.0 when there are favourable confusables
    pub fn compute_confusable_weight(&self, input: &str, candidate: VocabId) -> f64 {
        let mut weight = 1.0;
        if let Some(candidate) = self.decoder.get(candidate as usize) {
            let editscript = shortest_edit_script(input, &candidate.text, false, false, false);
            if self.debug {
                eprintln!("   (editscript {} -> {}: {:?})", input, candidate.text, editscript);
            }
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

    ///Searches a text and returns all highest-ranking variants found in the text
    pub fn find_all_matches<'a>(&self, text: &'a str, max_anagram_distance: u8, max_edit_distance: u8, max_matches: usize, score_threshold: f64, stop_criterion: StopCriterion, max_ngram: u8) -> Vec<Match<'a>> {
        let mut matches = Vec::new();

        //Find the boundaries and classify their strength
        let boundaries = find_boundaries(text);
        let strengths = classify_boundaries(&boundaries);

        let mut begin: usize = 0;

        //Compose the text into batches, each batch ends where a hard boundary is found
        for (i, strength) in strengths.iter().enumerate() {
            if *strength == BoundaryStrength::Hard {

                let boundaries = &boundaries[begin..i];

                //Gather all segments for this batch
                let mut all_segments: Vec<(Match<'a>,u8)> = Vec::new(); //second var in tuple corresponds to the ngram order
                for order in 1..=max_ngram {
                    all_segments.extend(find_ngrams(text, boundaries, order, begin).into_iter());
                }

                //find variants for all segments in this batch (in parallel)
                all_segments.par_iter_mut().for_each(|(segment, _)| {
                    let variants = self.find_variants(&segment.text, max_anagram_distance, max_edit_distance, max_matches, score_threshold, stop_criterion, None);
                    segment.variants = Some(variants);
                });

                //consolidate the matches, finding a single segmentation that has the best (highest
                //scoring) solution
                if max_ngram > 1 {
                    matches.extend(
                        self.most_likely_sequence(all_segments, boundaries, begin).into_iter()
                    );
                } else {
                    matches.extend(
                        all_segments.into_iter().map(|(x,_)| x)
                    );
                }

                begin = i+1;
            }

        }

        matches
    }


    /// Find the solution that maximizes the variant scores, decodes using a Weighted Finite State Transducer
    fn most_likely_sequence<'a>(&self, matches: Vec<(Match<'a>,u8)>, boundaries: &[Match<'a>], offset: usize) -> Vec<Match<'a>> {

        //Build a finite state transducer
        let mut fst = VectorFst::<LogWeight>::new();

        //add initial and final stae
        let start = fst.add_state();
        let end = fst.add_state();
        fst.set_start(start).expect("set start state");
        fst.set_final(end, 0.0).expect("set final state");

        //local decoder and encoder for the symbols for the FST, ties to the larger encoder/decoder
        //through SymbolReference::Known(VocabId) wherever possible. A symbol simply corresponds to a
        //variant string in either input or output, we speak of symbols in the context of the FST
        let mut symboltable: SymbolRefTable<'a> = SymbolRefTable::new();

        // Add FST states for all our matches
        //                           v--- inputsymbol
        //                                          v---- output symbol
        //                                                 v---- logprob
        let match_states: Vec<Vec<(usize, StateId, usize, f32)>> = matches.iter().map(|(m, _order)| {
            let inputsymbol = symboltable.symbol_from_match(&m);
            //for each march we add FST states for all variants
            if m.variants.is_some() && !m.variants.as_ref().unwrap().is_empty() {
                m.variants.as_ref().unwrap().iter().map(|(variant, score)| {
                    let outputsymbol = symboltable.symbol_from_vocabid(*variant);
                    ( inputsymbol, fst.add_state(), outputsymbol, score.ln() as f32 )
                }).collect()
            } else {
                //we have no variants at all, input = output
                //we use the maximum emission logprob (0), transition probability will be penalised
                //down to the uniform smoothing factor in later computations
                vec!((inputsymbol, fst.add_state(), inputsymbol, 0.0_f32))
            }
        }).collect();

        //Add transitions from the start stage
        matches.iter().enumerate().for_each(|(i, (nextmatch, _))| {
            if nextmatch.offset.begin == 0 {
                for (symbol_in, nextstate, symbol_out, emission_logprob) in match_states.get(i).expect("getting nextmatch") {
                    self.compute_fst_transition(&mut fst, &symboltable, *symbol_in, *symbol_out, None, start, *nextstate, *emission_logprob, start, end);
                }
            }
        });


        //For each boundary, add transition from all states directly left of the boundary
        //to all states directly right of the boundary
        for boundary in boundaries.iter() {
            //find all matches that end at this boundary
            let prevmatches = matches.iter().enumerate().filter(|(_i, (prevmatch, _))| {
                prevmatch.offset.end == boundary.offset.begin
            });

            //find all matches that start at this boundary
            let nextmatches: Vec<(usize, &(Match<'a>, u8))> = matches.iter().enumerate().filter(|(_i, (nextmatch, _))| {
                nextmatch.offset.begin == boundary.offset.end
            }).collect();

            //compute and add all state transitions
            for (prevmatch_index, (prevmatch, prevorder)) in prevmatches {
                for (_, prevstate, prevsymbol_out, prevlogprob) in match_states.get(prevmatch_index).expect("getting prevmatch") {
                    for (nextmatch_index, (nextmatch, nextorder)) in nextmatches.iter() {
                        for (symbol_in, nextstate, symbol_out, emission_logprob) in match_states.get(*nextmatch_index).expect("getting nextmatch") {
                            self.compute_fst_transition(&mut fst, &symboltable, *symbol_in, *symbol_out,Some(*prevsymbol_out), *prevstate, *nextstate, *emission_logprob, start, end);
                        }
                    }
                }
            }

        }

        let fst: VectorFst<LogWeight> = shortest_path(&fst).expect("shortest path fst");
        for path in fst.paths_iter() {

        }

        let mut segmentation = Vec::new();
        //TODO: Implement
        segmentation
    }

    /// Computes and sets the transition probability between two states, i.e. the weight
    /// to assign to this transition in the Finite State Transducer.
    /// The transition probability depends only on two states (Markov assumption) which
    /// is a simplification of reality.
    fn compute_fst_transition(&self, fst: &mut VectorFst<LogWeight>, symboltable: &SymbolRefTable<'_>, symbol_in: usize, symbol_out: usize, prevsymbol_out: Option<usize>, prevstate: StateId, nextstate: StateId, emission_logprob: f32, start: StateId, end: StateId) {
        if let Some(prevsymbol_out) = prevsymbol_out {
            //we have a previous output symbol, we can compute a transition
            let symbol_out_dec = symboltable.decode(symbol_out).expect("decoding nextsymbol");
            let prevsymbol_out_dec = symboltable.decode(prevsymbol_out).expect("decoding prevsymbol");
            let transition_logprob = match (prevsymbol_out_dec, symbol_out_dec) {
                (SymbolReference::Known(prevsymbol), SymbolReference::Known(symbol)) => {
                    self.get_transition_logprob(*symbol, *prevsymbol)
                },
                _ => {
                    //if either or both of the symbols is not known, we fall back
                    //to the uniform smoothing probability for the transition
                    TRANSITION_SMOOTHING_LOGPROB
                }
            };
            fst.add_tr(prevstate, Tr::new(symbol_in, symbol_out , transition_logprob + emission_logprob, nextstate)).expect("adding transition");
        } else {
            //we have a no previous output symbol, we can not compute a transition, fall back to
            //smoothing
            let symbol_out_dec = symboltable.decode(symbol_out).expect("decoding nextsymbol");
            fst.add_tr(prevstate, Tr::new(symbol_in, symbol_out , TRANSITION_SMOOTHING_LOGPROB + emission_logprob, nextstate)).expect("adding transition");
        }
    }

    /// Compute the probability of a transition between two words: $P(w_x|w_(x-1))$
    /// If either parameter is a high-order n-gram, this function will extract the appropriate
    /// unigrams on either side for the computation.
    /// The final probability is returned as a logprob (base e), if not the bigram or prior are
    /// not found, a uniform smoothing number is returned.
    fn get_transition_logprob(&self, word: VocabId, prev: VocabId) -> f32 {
        let word_dec = self.decoder.get(word as usize).expect("getting word");
        let prev_dec = self.decoder.get(prev as usize).expect("getting previous word");

        let nextword = if word_dec.tokencount > 1 {
            //next word is an n-gram, grab only the first word
            let firstword = word_dec.text.split(" ").next().expect("getting first word");
            *self.encoder.get(firstword).expect("ngram part should be in encoder")
        } else {
            word
        };

        let prevword = if prev_dec.tokencount > 1 {
            //previous word is an n-gram, grab the last part
            let lastword = word_dec.text.split(" ").last().expect("getting last word");
            *self.encoder.get(lastword).expect("ngram part should be in encoder")
        } else {
            prev
        };

        //Do we have a joint probability for the bigram that forms the transition?
        let bigram = NGram::BiGram(prevword, nextword);

        if let Some(jointcount) = self.ngrams.get(&bigram) {
            //find the prior
            let prior_dec = if prev_dec.tokencount > 1 {
                self.decoder.get(prevword as usize).expect("getting prior")  //all unigrams should be in the decoder
            } else {
                &prev_dec //we already had this one, use it
            };
            (*jointcount as f32 / prior_dec.frequency as f32).ln()
        } else {
            TRANSITION_SMOOTHING_LOGPROB
        }
    }

}
