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

    /// Ngrams for simple context-sensitive language modelling
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
        init_vocab(&mut model.decoder, &mut model.encoder);
        model
    }

    /// Instantiate a new variant model, explicitly passing an alphabet rather than loading one
    /// from file.
    pub fn new_with_alphabet(alphabet: Alphabet, weights: Weights, debug: bool) -> VariantModel {
        let mut model = VariantModel {
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
        };
        init_vocab(&mut model.decoder, &mut model.encoder);
        model
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



        // Hash all strings in the lexicon
        // and add them to the index
        let mut tmp_hashes: Vec<(AnaValue,VocabId)> = Vec::with_capacity(self.decoder.len());
        for (id, value)  in self.decoder.iter().enumerate() {
            if value.vocabtype == VocabType::NoIndex {
                //don't process special vocabulary types (bos, eos, etc)
                continue;
            }

            //get the anahash
            let anahash = value.text.anahash(&self.alphabet);
            if self.debug {
                eprintln!("   -- Anavalue={} VocabId={} Text={}", &anahash, id, value.text);
            }
            tmp_hashes.push((anahash, id as VocabId));
        }
        eprintln!(" - Found {} instances",tmp_hashes.len());


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

        eprintln!("Adding ngrams for simple language modelling...");

        //extra unigrams extracted from n-grams that need to be added to the vocabulary decoder
        let mut unseen_parts: Option<VocabEncoder> = Some(VocabEncoder::new());

        for id in 0..self.decoder.len() {
            //get the ngram and find any unseen parts
            let ngram = self.into_ngram(id as VocabId, &mut unseen_parts);

            let freq = self.decoder.get(id).unwrap().frequency;

            if ngram.len() > 1 {
                //reserve the space for the total counts
                for _ in self.freq_sum.len()..ngram.len() {
                    self.freq_sum.push(0);
                }
                //add to the totals for this order of ngrams
                self.freq_sum[ngram.len()-1] += freq as usize;
            } else {
                self.freq_sum[0] += freq as usize;
            }
            self.add_ngram(ngram, freq);
        }

        if let Some(unseen_parts) = unseen_parts {
            //add collected unseen n-gram parts to the decoder
            for (part, id) in unseen_parts {
                self.add_ngram(NGram::UniGram(id), 1);
                self.encoder.insert(part.clone(), id);
                self.decoder.push(VocabValue::new(part, VocabType::NoIndex));
            }
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
    pub fn read_vocabulary(&mut self, filename: &str, params: &VocabParams, lexicon_weight: f32, vocabtype: VocabType) -> Result<(), std::io::Error> {
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
                    self.add_to_vocabulary(text, Some(frequency), Some(lexicon_weight), self.lexicons.len() as u8, vocabtype);
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
                        let variantid = self.add_to_vocabulary(variant, None, Some(lexicon_weight), self.lexicons.len() as u8, VocabType::Normal);
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
                    let ref_id = self.add_to_vocabulary(reference, None, Some(lexicon_weight), self.lexicons.len() as u8, VocabType::Normal);
                    let mut iter = fields.iter();

                    while let (Some(variant), Some(score)) = (iter.next(), iter.next()) {
                        let score = score.parse::<f64>().expect("Scores must be a floating point value");
                        //all variants by definition are added to the lexicon
                        let variantid = self.add_to_vocabulary(variant, None, Some(lexicon_weight), self.lexicons.len() as u8, match intermediate {
                                true => VocabType::Intermediate,
                                false => VocabType::Normal
                        });
                        if variantid != ref_id {
                            if let Some(vocabvalue) = self.decoder.get_mut(ref_id as usize) {
                                let variantref = VariantReference::WeightedVariant((variantid,score) );
                                if intermediate {
                                    vocabvalue.vocabtype = VocabType::Intermediate;
                                }
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
    pub fn add_to_vocabulary(&mut self, text: &str, frequency: Option<u32>, lexicon_weight: Option<f32>, lexicon_index: u8, vocabtype: VocabType) -> VocabId {
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
            if vocab_id == &BOS || vocab_id == &EOS || vocab_id == &UNK {
                item.vocabtype = VocabType::NoIndex;
            } else if item.vocabtype == VocabType::Intermediate { //we only override the intermediate type, meaning something can become 'Normal' after having been 'Intermediate', but not vice versa
                item.vocabtype = vocabtype;
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
                vocabtype
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
                            if vocabitem.vocabtype == VocabType::Normal {
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

        if self.debug {
            eprintln!("(finding all matches in text: {})", text);
        }

        //Find the boundaries and classify their strength
        let boundaries = find_boundaries(text);
        let strengths = classify_boundaries(&boundaries);

        if self.debug {
            eprintln!("  (boundaries: {:?})", boundaries);
            eprintln!("  ( strenghts: {:?})", strengths);
        }

        let mut begin: usize = 0;

        //Compose the text into batches, each batch ends where a hard boundary is found
        for (i, (strength, boundary)) in strengths.iter().zip(boundaries.iter()).enumerate() {
            if *strength == BoundaryStrength::Hard {
                if self.debug {
                    eprintln!("  (found hard boundary at {}:{})", boundary.offset.begin, boundary.offset.end);
                }

                let boundaries = &boundaries[begin..i+1];

                //Gather all segments for this batch
                let mut all_segments: Vec<(Match<'a>,u8)> = Vec::new(); //second var in tuple corresponds to the ngram order
                for order in 1..=max_ngram {
                    all_segments.extend(find_match_ngrams(text, boundaries, order, 0).into_iter());
                }
                if self.debug {
                    eprintln!("  (processing ngrams: {:?})", all_segments);
                }

                //find variants for all segments in this batch (in parallel)
                all_segments.par_iter_mut().for_each(|(segment, _)| {
                    if self.debug {
                        eprintln!("   (----------- finding variants for: {} -----------)", segment.text);
                    }
                    let variants = self.find_variants(&segment.text, max_anagram_distance, max_edit_distance, max_matches, score_threshold, stop_criterion, None);
                    segment.variants = Some(variants);
                });

                //consolidate the matches, finding a single segmentation that has the best (highest
                //scoring) solution
                if max_ngram > 1 {
                    //(debug will be handled in the called method)
                    matches.extend(
                        self.most_likely_sequence(all_segments, boundaries, begin, boundary.offset.begin).into_iter()
                    );
                } else {
                    if self.debug {
                        eprintln!("(returning matches directly, no need to find most likely sequence for unigrams)");
                    }
                    matches.extend(
                        all_segments.into_iter().map(|(mut m,_)| {
                            m.selected = Some(0); //select the first (highest ranking) option
                            m
                        })
                    );
                }

                begin = boundary.offset.end; //(the hard boundary itself is not included in any variant/sequence matching)
            }

        }

        matches
    }


    /// Find the solution that maximizes the variant scores, decodes using a Weighted Finite State Transducer
    fn most_likely_sequence<'a>(&self, matches: Vec<(Match<'a>,u8)>, boundaries: &[Match<'a>], begin_offset: usize, end_offset: usize) -> Vec<Match<'a>> {
        if self.debug {
            eprintln!("(building FST for finding most likely sequence in range {}:{})", begin_offset, end_offset);
        }

        //Build a finite state transducer
        let mut fst = VectorFst::<TropicalWeight>::new();

        //add initial and final stage
        let start = fst.add_state();
        let end = fst.add_state();
        fst.set_start(start).expect("set start state");
        fst.set_final(end, 0.0).expect("set final state");


        //Maps states back to the index of matches
        let mut states: HashMap<usize,StateInfo<'a>> = HashMap::new();

        // Add FST states for all our matches
        //                           v--- inputsymbol
        //                                     v---- state id, double as output symbol
        //                                            v---- emission logprob
        let match_states: Vec<Vec<StateId>> = matches.iter().enumerate().map(|(i, (m, _order))| {

            //for each match we add FST states for all variants
            if m.variants.is_some() && !m.variants.as_ref().unwrap().is_empty() {
                m.variants.as_ref().unwrap().iter().enumerate().map(|(j, (variant, score))| {
                    let state_id = fst.add_state();
                    states.insert(state_id, StateInfo {
                        input: Some(m.text),
                        output: Some(*variant),
                        match_index: i,
                        variant_index: Some(j),
                        emission_logprob: score.ln() as f32,
                        offset: Some(m.offset.clone()),
                        tokencount: m.internal_boundaries(boundaries).iter().count() + 1 //could possibly be slightly optimised by computing earlier, but is relatively low cost
                    });
                    if self.debug {
                        eprintln!("   (added state {} (match {}, variant {}), input={}, output={}) ", state_id, i, j, m.text, self.decoder.get(*variant as usize).unwrap().text );
                    }
                    state_id
                }).collect()
            } else {
                //we have no variants at all, input = output
                //we use the maximum emission logprob (0), transition probability will be penalised
                //down to the uniform smoothing factor in later computations
                let state_id = fst.add_state();
                states.insert(state_id, StateInfo {
                    input: Some(m.text),
                    output: None, //this means we copy the input
                    match_index: i,
                    variant_index: None,
                    emission_logprob: 0.0,
                    offset: Some(m.offset.clone()),
                    tokencount: m.internal_boundaries(boundaries).iter().count() + 1 //could possibly be slightly optimised by computing earlier, but is relatively low cost
                });
                if self.debug {
                    eprintln!("   (added out-of-vocabulary state {}, input/output={}) ", state_id, m.text );
                }
                vec!(state_id)
            }
        }).collect();

        //dummy stateinfo for start and end state
        let dummy_stateinfo = StateInfo {
                            input: None,
                            output: None,
                            match_index: matches.len(), //a match_index because the start/end state is not tied to a match
                                                            //at least this way we can separate it
                                                            //easily form the rest
                            variant_index: None,
                            emission_logprob: 0.0, //the max
                            offset: None,
                            tokencount: 0,
        };

        if self.debug {
            eprintln!(" (added {} FST states)", states.len());
            eprintln!(" (adding transition from the start state)");
        }
        matches.iter().enumerate().for_each(|(i, (nextmatch, _))| {
            //Add transitions from the start state;
            if nextmatch.offset.begin == begin_offset {
                for state in match_states.get(i).expect("getting nextmatch") {
                    let stateinfo = states.get(state).expect("getting state info");
                    self.compute_fst_transition(&mut fst, *state, stateinfo, &dummy_stateinfo, start, start, end);
                }
            }
        });



        //For each boundary, add transition from all states directly left of the boundary
        //to all states directly right of the boundary
        for (b, boundary) in boundaries.iter().enumerate() {
          if boundary.offset.begin >= begin_offset && boundary.offset.end <= end_offset {

            //find all states that end at this boundary
            let prevstates = states.iter().filter_map(|(state,stateinfo)| {
                if stateinfo.offset.is_some() && stateinfo.offset.as_ref().unwrap().end == boundary.offset.begin {
                    Some(state)
                } else {
                    None
                }
            });

            //find all states that start at this boundary
            let nextstates: Vec<usize> = states.iter().filter_map(|(state,stateinfo)| {
                if stateinfo.offset.is_some() && stateinfo.offset.as_ref().unwrap().begin == boundary.offset.end {
                    Some(*state)
                } else {
                    None
                }
            }).collect();

            if self.debug {
                eprintln!("  (boundary #{}, nextmatches={})", b+1, nextstates.len());
            }

            let mut count = 0;

            //compute and add all state transitions
            for prevstate in prevstates {
                let prevstateinfo = states.get(prevstate).expect("getting prevstate info" );
                for nextstate in nextstates.iter() {
                    let stateinfo = states.get(nextstate).expect("getting nextstate info");
                    self.compute_fst_transition(&mut fst, *nextstate, stateinfo,prevstateinfo, *prevstate, start, end);
                    count += 1;
                }

                if prevstateinfo.offset.is_some() && prevstateinfo.offset.as_ref().unwrap().end == end_offset {
                    //Add transitions to the end state
                    if self.debug {
                        eprintln!("  (adding transition to end state)");
                    }
                    self.compute_fst_transition(&mut fst, end, &dummy_stateinfo, prevstateinfo, *prevstate, start, end);
                    count += 1;
                }

            }

            if self.debug {
                eprintln!("   (added {} transitions)", count);
            }
          }
        }


        let mut match_sequence = Vec::new();

        if self.debug {
            eprintln!(" (computed FST: {:?})", fst);
            eprintln!(" (finding shortest path)");
            fst.draw("/tmp/fst.dot", &DrawingConfig::default() );
        }
        let fst: VectorFst<TropicalWeight> = shortest_path(&fst).expect("computing shortest path fst");
        for path in fst.paths_iter() {
            if self.debug {
                eprintln!(" (shortest path: {:?})", path);
            }
            for (input_index, output_index) in path.ilabels.iter().zip(path.olabels.iter()) {
                //input labels use +1 because 0 means epsilon in FST context

                let match_index = input_index - 1; //input labels use +1 because 0 means epsilon in FST context
                eprintln!(" (match_index/ilabel={}, output_index/olabel={})", match_index, output_index);
                if let Some((m,_)) = matches.get(match_index) {
                    if *output_index == OOV_COPY_FROM_INPUT {
                        //output is the same as input, we just return the entire match
                        match_sequence.push(m.clone());
                        if self.debug {
                            eprintln!("  (returning: {} (unchanged/oov)", m.text);
                        }
                    } else {
                        let stateinfo = states.get(output_index).expect("get stateinfo for output");
                        let mut m = m.clone();
                        m.selected = stateinfo.variant_index;
                        if self.debug {
                            if m.selected.is_some() {
                                eprintln!("  (returning: {}->{})", m.text , self.match_to_str(&m));
                            } else {
                                eprintln!("  (returning: {} (unchanged/oov))", m.text);
                            }
                        }
                        match_sequence.push(m);
                    }
                }
            }
        }

        match_sequence
    }



    /// Computes and sets the transition probability between two states, i.e. the weight
    /// to assign to this transition in the Finite State Transducer.
    /// The transition probability depends only on two states (Markov assumption) which
    /// is a simplification of reality.
    fn compute_fst_transition<'a>(&self, fst: &mut VectorFst<TropicalWeight>, state: usize, stateinfo: &StateInfo<'a>, prevstateinfo: &StateInfo<'a>, prevstate: StateId, start: StateId, end: StateId) {
        if let Some(previous_output) = prevstateinfo.output {
            //normal transition with known previous output
            let prior = self.into_ngram(previous_output, &mut None);
            let (transition_logprob, output) = if let Some(vocab_id) = stateinfo.output {
                let ngram = self.into_ngram(vocab_id, &mut None);
                if self.debug {
                    eprintln!("   (adding transition {}->{}: {}->{})", prevstate, state, self.ngram_to_str(&prior), self.ngram_to_str(&ngram) );
                }
                (self.get_transition_logprob(ngram, prior), state)
            } else if state == end {
                //connect to the end state
                let ngram = NGram::UniGram(EOS);
                if self.debug {
                    eprintln!("   (adding transition {}->{}(=EOS): {}->{})", prevstate, state, self.ngram_to_str(&prior), self.ngram_to_str(&ngram) );
                }
                (self.get_transition_logprob(ngram, prior), 0)  //0=epsilon, no output after the last stage

            } else {
                if self.debug {
                    eprintln!("   (adding transition with out-of-vocabulary output (input=output): {}->{}: {}->{:?})", prevstate, state, self.ngram_to_str(&prior), stateinfo.input );
                }
                //we have no output, that means we copy from the input and can not compute a proper
                //transition
                if stateinfo.tokencount > 1 {
                    //if this is an n-gram, we also count the internal transitions as unknown
                    //transitions, so there is no bias towards selecting larger n-gram fragments
                    (TRANSITION_SMOOTHING_LOGPROB * stateinfo.tokencount as f32, OOV_COPY_FROM_INPUT)
                } else {
                    (TRANSITION_SMOOTHING_LOGPROB, OOV_COPY_FROM_INPUT)
                }
            };
            if self.debug {
                eprintln!("     (p={}, transition score={}, emission score={}, tokencount={})", transition_logprob + stateinfo.emission_logprob, transition_logprob, stateinfo.emission_logprob, stateinfo.tokencount);
            }
            fst.add_tr(prevstate, Tr::new(stateinfo.match_index+1, output, -1.0 * (transition_logprob + stateinfo.emission_logprob), state)).expect("adding transition");
                                                                         // ^-- we remove the sign
                                                                         // from the logprob
                                                                         // because shortest_path minimizes
                                                                         // instead of maximizes
        } else if prevstate == start {
            let (transition_logprob, output) = if let Some(vocab_id) = stateinfo.output {
                let ngram = self.into_ngram(vocab_id, &mut None);
                let prior = NGram::UniGram(BOS);
                if self.debug {
                    eprintln!("   (adding transition {}(=BOS)->{}: {}->{})", prevstate, state, self.ngram_to_str(&prior), self.ngram_to_str(&ngram) );
                }
                (self.get_transition_logprob(ngram, prior), state)
            } else {
                //we have no output, that means we copy from the input and can not compute a proper
                //transition
                if self.debug {
                    eprintln!("   (adding transition with out-of-vocabulary output (input=output) {}(=BOS)->{}: {}->{:?})", prevstate, state, "<bos>", stateinfo.input );
                }
                if stateinfo.tokencount > 1 {
                    //if this is an n-gram, we also count the internal transitions as unknown
                    //transitions, so there is no bias towards selecting larger n-gram fragments
                    (TRANSITION_SMOOTHING_LOGPROB * stateinfo.tokencount as f32, OOV_COPY_FROM_INPUT)
                } else {
                    (TRANSITION_SMOOTHING_LOGPROB, OOV_COPY_FROM_INPUT)
                }
            };
            if self.debug {
                eprintln!("     (p={}, transition score={}, emission score={}, tokencount={})", transition_logprob + stateinfo.emission_logprob, transition_logprob, stateinfo.emission_logprob, stateinfo.tokencount);
            }
            fst.add_tr(prevstate, Tr::new(stateinfo.match_index+1, output, -1.0 * (transition_logprob + stateinfo.emission_logprob), state)).expect("adding transition");
        } else {
            //we have no previous output symbol, we can not compute a transition, fall back to
            //smoothing
            let output = if stateinfo.output.is_some() {
                state
            } else {
                OOV_COPY_FROM_INPUT
            };
            let transition_logprob = if stateinfo.tokencount > 1 {
                    //if this is an n-gram, we also count the internal transitions as unknown
                    //transitions, so there is no bias towards selecting larger n-gram fragments
                    TRANSITION_SMOOTHING_LOGPROB * stateinfo.tokencount as f32
                } else {
                    TRANSITION_SMOOTHING_LOGPROB
            };
            if self.debug {
                let ilabel = if let Some(v) = prevstateinfo.output {
                    self.decoder.get(v as usize).unwrap().text.as_str()
                } else {
                    prevstateinfo.input.as_ref().unwrap_or(&"NULL")
                };
                let olabel = if let Some(v) = stateinfo.output {
                    self.decoder.get(v as usize).unwrap().text.as_str()
                } else {
                    stateinfo.input.as_ref().unwrap_or(&"NULL")
                };
                eprintln!("   (adding transition from out-of-vocabulary output: {}->{}: {}->{})", prevstate, state, ilabel, olabel);
                eprintln!("     (p={}, transition score={}, emission score={}, tokens={})", transition_logprob + stateinfo.emission_logprob, transition_logprob ,  stateinfo.emission_logprob, stateinfo.tokencount);
            }
            fst.add_tr(prevstate, Tr::new(stateinfo.match_index+1, output, -1.0 * (transition_logprob + stateinfo.emission_logprob), state)).expect("adding transition");
        }
    }

    /// Add an ngram for language modelling
    pub fn add_ngram(&mut self, ngram: NGram, frequency: u32) {
        if let Some(ngram) = self.ngrams.get_mut(&ngram) {
            //update the count for this ngram
            *ngram += frequency;
        } else {
            //add the new ngram
            self.ngrams.insert( ngram, frequency );
        }
    }

    /// Decompose a known vocabulary Id into an Ngram
    fn into_ngram(&self, word: VocabId, unseen_parts: &mut Option<VocabEncoder>) -> NGram {
        let word_dec = self.decoder.get(word as usize).expect("word does not exist in decoder");
        let mut iter = word_dec.text.split(" ");
        match word_dec.tokencount {
            0 => NGram::Empty,
            1 => NGram::UniGram(
                self.encode_token(iter.next().expect("ngram part"), false, unseen_parts)
            ),
            2 => NGram::BiGram(
                self.encode_token(iter.next().expect("ngram part"), false, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), false, unseen_parts)
            ),
            3 => NGram::TriGram(
                self.encode_token(iter.next().expect("ngram part"), false, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), false, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), false, unseen_parts)
            ),
            _ => panic!("Can only deal with n-grams up to order 3")
        }
    }

    /// Encode one token, optionally returning either UNK or putting it in ``unseen`` if it is new.
    /// Use in ngram construction
    fn encode_token(&self, token: &str, use_unk: bool, unseen: &mut Option<VocabEncoder>) -> VocabId {
        if let Some(vocab_id) = self.encoder.get(token) {
            *vocab_id
        } else if use_unk {
            UNK
        } else if let Some(unseen) = unseen.as_mut() {
            if let Some(vocab_id) = unseen.get(token) {
                *vocab_id
            } else {
                let vocab_id: VocabId = self.decoder.len()  as VocabId + unseen.len() as VocabId;
                unseen.insert(token.to_string(), vocab_id);
                vocab_id
            }
        } else {
            panic!("Token does not exist in vocabulary (and returning unknown tokens or adding new ones was not set)");
        }
    }

    /// Compute the probability of a transition between two words: $P(w_x|w_(x-1))$
    /// If either parameter is a high-order n-gram, this function will extract the appropriate
    /// unigrams on either side for the computation.
    /// The final probability is returned as a logprob (base e), if not the bigram or prior are
    /// not found, a uniform smoothing number is returned.
    fn get_transition_logprob(&self, mut ngram: NGram, mut prior: NGram) -> f32 {
        if ngram == NGram::Empty || prior == NGram::Empty {
            return TRANSITION_SMOOTHING_LOGPROB;
        }

        if prior.len() > 1 {
            prior = prior.pop_last(); //we can only handle one word of history, discard the rest
        }

        let word = ngram.pop_first();

        let priorcount = if let Some(priorcount) = self.ngrams.get(&prior) {
            *priorcount
        } else {
            1
        };

        //Do we have a joint probability for the bigram that forms the transition?
        let bigram = NGram::BiGram(prior.first().unwrap(), word.first().unwrap());

        let transition_logprob = if let Some(jointcount) = self.ngrams.get(&bigram) {
            if priorcount < *jointcount {
                (*jointcount as f32).ln()
            } else {
                (*jointcount as f32 / priorcount as f32).ln()
            }
        } else {
            TRANSITION_SMOOTHING_LOGPROB
        };

        if ngram.len() >= 1 {
            //recursion step for subquents parts of the ngram
            transition_logprob + self.get_transition_logprob(ngram, word)
        } else {
            transition_logprob
        }
    }

    /// Gives the text representation for this match, always uses the solution (if any) and falls
    /// back to the input text only when no solution was found.
    pub fn match_to_str<'a>(&'a self, m: &Match<'a>) -> &'a str {
        if let Some((vocab_id,_)) = m.solution() {
            self.decoder.get(vocab_id as usize).expect("solution should refer to a valid vocab id").text.as_str()
        } else {
            m.text
        }
    }

    /// Turns the ngram into a tokenised string; the tokens in the ngram will be separated by a space.
    pub fn ngram_to_str(&self, ngram: &NGram) -> String {
        let v: Vec<&str> = ngram.to_vec().into_iter().map(|v| self.decoder.get(v as usize).expect("ngram must contain valid vocab ids").text.as_str() ).collect();
        v.join(" ")
    }

    /// Converts a match to an NGram representation, this only works if all tokens in the ngram are
    /// in the vocabulary.
    pub fn match_to_ngram<'a>(&'a self, m: &Match<'a>, boundaries: &[Match<'a>]) -> Result<NGram, String> {
        let internal = m.internal_boundaries(boundaries);
        let parts = find_match_ngrams(m.text, internal, 1, 0);
        let mut ngram = NGram::Empty;
        for (part,_) in parts {
            if let Some(vocabid) = self.encoder.get(part.text) {
                ngram.push(*vocabid);
            } else {
                return Err(format!("unable to convert match to ngram, contains out-of-vocabulary token: {}", part.text));
            }
        }
        Ok(ngram)
    }


}
