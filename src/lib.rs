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
use std::sync::Arc;
use std::convert::TryFrom;
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
    pub fn read_vocabulary(&mut self, filename: &str, params: &VocabParams) -> Result<(), std::io::Error> {
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
                    self.add_to_vocabulary(text, Some(frequency), params);
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
    pub fn read_variants(&mut self, filename: &str, params: Option<&VocabParams>) -> Result<(), std::io::Error> {
        let params = if let Some(params) = params {
            let mut p = params.clone();
            p.index = self.lexicons.len() as u8;
            p
        } else {
            VocabParams {
                index: self.lexicons.len() as u8,
                ..Default::default()
            }
        };

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
                        let variantid = self.add_to_vocabulary(variant, None, &params);
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
    pub fn read_weighted_variants(&mut self, filename: &str, params: Option<&VocabParams>, intermediate: bool) -> Result<(), std::io::Error> {
        let params = if let Some(params) = params {
            let mut p = params.clone();
            p.index = self.lexicons.len() as u8;
            p
        } else {
            VocabParams {
                index: self.lexicons.len() as u8,
                ..Default::default()
            }
        };
        let intermediate_params = if intermediate {
            let mut p = params.clone();
            p.vocab_type = VocabType::Intermediate;
            p
        } else {
            params.clone()
        };

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
                    let ref_id = self.add_to_vocabulary(reference, None, &params);
                    let mut iter = fields.iter();

                    while let (Some(variant), Some(score)) = (iter.next(), iter.next()) {
                        let score = score.parse::<f64>().expect("Scores must be a floating point value");
                        //all variants by definition are added to the lexicon
                        let variantid = self.add_to_vocabulary(variant, None,  match intermediate {
                                true => &intermediate_params,
                                false => &params
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
    pub fn add_to_vocabulary(&mut self, text: &str, frequency: Option<u32>, params: &VocabParams) -> VocabId {
        let frequency = frequency.unwrap_or(1);
        if self.debug {
            eprintln!(" -- Adding to vocabulary: {}  ({})", text, frequency);
        }
        if let Some(vocab_id) = self.encoder.get(text) {
            let item = self.decoder.get_mut(*vocab_id as usize).expect(&format!("Retrieving existing vocabulary entry {}",vocab_id));
            match params.freq_handling {
                FrequencyHandling::Sum => {
                    item.frequency += frequency;
                },
                FrequencyHandling::Max => {
                    item.frequency = if frequency > item.frequency { frequency } else { item.frequency };
                },
                FrequencyHandling::Min => {
                    item.frequency = if frequency < item.frequency { frequency } else { item.frequency };
                },
                FrequencyHandling::Replace => {
                    item.frequency = frequency
                },
                FrequencyHandling::SumIfMoreWeight => {
                    if params.weight > item.lexweight {
                        item.frequency += frequency;
                    }
                },
                FrequencyHandling::MaxIfMoreWeight => {
                    if params.weight > item.lexweight {
                        item.frequency = if frequency > item.frequency { frequency } else { item.frequency };
                    }
                },
                FrequencyHandling::MinIfMoreWeight => {
                    if params.weight > item.lexweight {
                        item.frequency = if frequency < item.frequency { frequency } else { item.frequency };
                    }
                },
                FrequencyHandling::ReplaceIfMoreWeight => {
                    if params.weight > item.lexweight {
                        item.frequency = frequency
                    }
                },
            }
            if params.weight > item.lexweight {
                item.lexweight = params.weight;
                item.lexindex = params.index;
            }
            if vocab_id == &BOS || vocab_id == &EOS || vocab_id == &UNK {
                item.vocabtype = VocabType::NoIndex; //by definition
            } else if item.vocabtype == VocabType::Intermediate { //we only override the intermediate type, meaning something can become 'Normal' after having been 'Intermediate', but not vice versa
                item.vocabtype = params.vocab_type;
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
                lexweight: params.weight,
                lexindex: params.index,
                variants: None,
                vocabtype: params.vocab_type
            });
            self.decoder.len() as VocabId - 1
        }
    }

    /// Find variants in the vocabulary for a given string (in its totality), returns a vector of vocabulaly ID and score pairs
    /// The resulting vocabulary Ids can be resolved through `get_vocab()`
    pub fn find_variants(&self, input: &str, params: &SearchParameters, cache: Option<&mut Cache>) -> Vec<(VocabId, f64)> {

        //Compute the anahash
        let normstring = input.normalize_to_alphabet(&self.alphabet);
        let anahash = input.anahash(&self.alphabet);

        //dynamically computed maximum distance, this will override max_edit_distance
        //when the number is smaller (for short input strings)
        let max_dynamic_distance: u8 = (normstring.len() as f64 / 2.0).floor() as u8;

        //Compute neighbouring anahashes and find the nearest anahashes in the model
        let anahashes = self.find_nearest_anahashes(&anahash, &normstring,
                                                    min(params.max_anagram_distance, max_dynamic_distance),
                                                    params.stop_criterion,
                                                    if let Some(cache) = cache {
                                                       Some(&mut cache.visited)
                                                    } else {
                                                       None
                                                    });

        //Get the instances pertaining to the collected hashes, within a certain maximum distance
        //and compute distances
        let variants = self.gather_instances(&anahashes, &normstring, input, min(params.max_edit_distance, max_dynamic_distance));

        self.score_and_rank(variants, input, params.max_matches, params.score_threshold)
    }


    /// Find the nearest anahashes that exists in the model (computing anahashes in the
    /// neigbhourhood if needed).
    pub(crate) fn find_nearest_anahashes<'a>(&'a self, focus: &AnaValue, normstring: &Vec<u8>, max_distance: u8,  stop_criterion: StopCriterion, cache: Option<&mut HashSet<AnaValue>>) -> HashSet<&'a AnaValue> {
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
    pub(crate) fn gather_instances(&self, nearest_anagrams: &HashSet<&AnaValue>, querystring: &[u8], query: &str, max_edit_distance: u8) -> Vec<(VocabId,Distance)> {
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
    pub(crate) fn score_and_rank(&self, instances: Vec<(VocabId,Distance)>, input: &str, max_matches: usize, score_threshold: f64 ) -> Vec<(VocabId,f64)> {
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
    pub fn find_all_matches<'a>(&self, text: &'a str, params: &SearchParameters) -> Vec<Match<'a>> {
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
        let mut begin_index: usize = 0;

        //Compose the text into batches, each batch ends where a hard boundary is found
        for (i, (strength, boundary)) in strengths.iter().zip(boundaries.iter()).enumerate() {
            if *strength == BoundaryStrength::Hard {
                let text_current = &text[begin..boundary.offset.begin];

                let boundaries = &boundaries[begin_index..i+1];
                if self.debug {
                    eprintln!("  (found hard boundary at {}:{}: {})", boundary.offset.begin, boundary.offset.end, text_current);
                    for boundary in boundaries.iter() {
                        eprintln!("    (inner boundary {}:{})", boundary.offset.begin, boundary.offset.end);
                    }
                }

                //Gather all segments for this batch
                let mut all_segments: Vec<Match<'a>> = Vec::new(); //second var in tuple corresponds to the ngram order
                for order in 1..=params.max_ngram {
                    all_segments.extend(find_match_ngrams(text, boundaries, order, begin, Some(boundary.offset.begin)).into_iter());
                }
                if self.debug {
                    eprintln!("  (processing {} ngrams: {:?})", all_segments.len(), all_segments);
                }

                //find variants for all segments in this batch (in parallel)
                if params.single_thread {
                    all_segments.iter_mut().for_each(|(segment)| {
                        if self.debug {
                            eprintln!("   (----------- finding variants for: {} -----------)", segment.text);
                        }
                        let variants = self.find_variants(&segment.text, params, None);
                        segment.variants = Some(variants);
                    });
                } else {
                    all_segments.par_iter_mut().for_each(|(segment)| {
                        if self.debug {
                            eprintln!("   (----------- finding variants for: {} -----------)", segment.text);
                        }
                        let variants = self.find_variants(&segment.text, params, None);
                        segment.variants = Some(variants);
                    });
                }


                let l = matches.len();
                //consolidate the matches, finding a single segmentation that has the best (highest
                //scoring) solution
                if params.max_ngram > 1 {
                    //(debug will be handled in the called method)
                    matches.extend(
                        self.most_likely_sequence(all_segments, boundaries, begin, boundary.offset.begin, params).into_iter()
                    );
                } else {
                    if self.debug {
                        eprintln!("  (returning matches directly, no need to find most likely sequence for unigrams)");
                    }
                    matches.extend(
                        all_segments.into_iter().map(|(mut m)| {
                            m.selected = Some(0); //select the first (highest ranking) option
                            m
                        })
                    );
                }
                if self.debug {
                    eprintln!("  (added sequence of {} matches)", matches.len() - l );
                }

                begin = boundary.offset.end; //(the hard boundary itself is not included in any variant/sequence matching)
                begin_index = i+1
            }

        }

        if self.debug {
            eprintln!("(returning {} matches: {:?})", matches.len(), matches);
        }
        matches
    }


    /// Find the solution that maximizes the variant scores, decodes using a Weighted Finite State Transducer
    fn most_likely_sequence<'a>(&self, matches: Vec<Match<'a>>, boundaries: &[Match<'a>], begin_offset: usize, end_offset: usize, params: &SearchParameters) -> Vec<Match<'a>> {
        if self.debug {
            eprintln!("(building FST for finding most likely sequence in range {}:{})", begin_offset, end_offset);
        }

        //Build a finite state transducer
        let mut fst = VectorFst::<TropicalWeight>::new();
        let mut symtab_in = SymbolTable::new(); //only used for drawing the FST in debug mode
        let mut symtab_out = SymbolTable::new(); //only used for drawing the FST in debug mode

        //add initial state
        let start = fst.add_state();
        fst.set_start(start).expect("set start state");


        //adds states for all boundaries
        let mut final_found = false;
        let states: Vec<usize> = boundaries.iter().map(|boundary| {
            let state = fst.add_state();
            if boundary.offset.begin == end_offset || boundary.offset.end == end_offset {
                final_found = true;
                fst.set_final(state, 0.0).expect("set end state");
            }
            state
        }).collect();

        if !final_found { //sanity check
            panic!("no final state found");
        }

        if self.debug {
            eprintln!(" (added {} states ({} boundaries), not including the start state)", states.len(), boundaries.len());
        }

        let mut output_symbols: Vec<OutputSymbol> = vec!(
            OutputSymbol { vocab_id: 0, symbol: 0, match_index: 0, variant_index: None, boundary_index: 0 }, //first entry is a dummy entry because the 0 symbol is reserved for epsilon
        );

        //add transitions between the boundary states
        for (match_index, m) in matches.iter().enumerate() {

            if self.debug {
                symtab_in.add_symbol(m.text); //symbol_index = match_index + 1
            }

            let mut prevboundary: Option<usize> = None;
            let mut nextboundary: Option<usize> = None;

            let input_symbol = match_index + 1;

            for (i, boundary) in boundaries.iter().enumerate() {
                if m.offset.begin == boundary.offset.end  {
                    prevboundary = Some(i)
                } else if m.offset.end == boundary.offset.begin  {
                    nextboundary = Some(i)
                }
            }

            let prevstate= if let Some(prevboundary) = prevboundary {
                *states.get(prevboundary).expect("prev state must exist")
            } else {
                start
            };
            let nextstate = *states.get(nextboundary.expect("next boundary must exist")).expect("next state must exist");

            if m.variants.is_some() && !m.variants.as_ref().unwrap().is_empty() {
                for (variant_index, (variant, score)) in m.variants.as_ref().unwrap().iter().enumerate() {
                    let output_symbol = output_symbols.len();
                    output_symbols.push( OutputSymbol {
                        vocab_id: *variant,
                        symbol: output_symbol,
                        match_index,
                        variant_index: Some(variant_index),
                        boundary_index: nextboundary.expect("next boundary must exist")
                    });

                    if self.debug {
                        let variant_text = self.decoder.get(*variant as usize).expect("variant_text").text.as_str();
                        eprintln!("   (transition {}->{} with symbol {}->{} and score {})", prevstate, nextstate, input_symbol, output_symbol, -1.0 * score.ln() as f32);
                        assert!(symtab_out.add_symbol(variant_text) == output_symbol);
                    }

                    fst.add_tr(prevstate, Tr::new(input_symbol, output_symbol, -1.0 * score.ln() as f32, nextstate)).expect("adding transition");
                }
            } else {
                let output_symbol = output_symbols.len();
                output_symbols.push( OutputSymbol {
                    vocab_id: 0, //0 vocab_id means we have an Out-of-Vocabulary word to copy from input
                    symbol: output_symbol,
                    match_index,
                    variant_index: None,
                    boundary_index: nextboundary.expect("next boundary must exist")
                });

                if self.debug {
                    eprintln!("   (transition {}->{} with OOV symbol {}->{} and score {})", prevstate, nextstate, input_symbol, output_symbol, -1.0 * OOV_EMISSION_PROB);
                    assert!(symtab_out.add_symbol(m.text) == output_symbol);
                }

                fst.add_tr(prevstate, Tr::new(input_symbol, output_symbol, -1.0 * OOV_EMISSION_PROB, nextstate)).expect("adding transition");
            }
        }

        //find the n most likely sequences, note that we only consider the variant scores here,
        //language modelling (considering context) is applied in a separate step later

        if self.debug {
            eprintln!(" (computed FST: {:?})", fst);
            eprintln!("   (symtab_in={:?})", symtab_in);
            eprintln!("   (symtab_out={:?})", symtab_out);
            eprintln!(" (finding shortest path)");
            fst.set_input_symbols(Arc::new(symtab_in));
            fst.set_output_symbols(Arc::new(symtab_out));
            if let Err(e) = fst.draw("/tmp/fst.dot", &DrawingConfig::default() ) {
                panic!("FST draw error: {}", e);
            }
        }
        let fst: VectorFst<TropicalWeight> = shortest_path_with_config(&fst, ShortestPathConfig::default().with_nshortest(params.max_seq) ).expect("computing shortest path fst");
        let mut sequences: Vec<Sequence> = Vec::new();
        let mut best_lm_logprob: f32 = -99999999.0;
        for (i, path)  in fst.paths_iter().enumerate() { //iterates over the n shortest path hypotheses (does not return them in weighted order)
            let w: f32 = *path.weight.value();
            let mut sequence = Sequence::new(w * -1.0f32);
            if self.debug {
                eprintln!("  (#{}, path: {:?})", i+1, path);
            }
            for (input_symbol, output_symbol) in path.ilabels.iter().zip(path.olabels.iter()) {
                let output_symbol = output_symbols.get(*output_symbol).expect("expected valid output symbol");
                sequence.output_symbols.push(output_symbol.clone());
            }

            let (lm_logprob, perplexity) = self.lm_score(&sequence, &matches, &boundaries);
            sequence.lm_logprob = lm_logprob;
            if sequence.lm_logprob > best_lm_logprob {
                best_lm_logprob = sequence.lm_logprob;
            }
            sequences.push(sequence);
        }

        let mut best_score: f32 = -99999999.0;
        let mut best_sequence: Option<Sequence> = None;
        for (i, sequence) in sequences.into_iter().enumerate() {
            //let norm_lm_logprob = sequence.lm_logprob - best_lm_logprob;
            //because we compute this in log-space this is essentially a weighted geometric mean
            //rather than an arithmetic mean. The geometric mean should be a good fit for normalised
            //pseudo-probability ratios like our scores.
            let score = (params.lm_weight * sequence.lm_logprob + params.variantmodel_weight * sequence.emission_logprob) / (params.lm_weight + params.variantmodel_weight); //note: the denominator isn't really relevant for finding the best score
            if self.debug {
                eprintln!("  (#{}, score={}, lm_logprob={}, variant_logprob={})", i+1, score, sequence.lm_logprob, sequence.emission_logprob);
                let mut text: String = String::new();
                for output_symbol in sequence.output_symbols.iter() {
                    if output_symbol.vocab_id > 0{
                        text += self.decoder.get(output_symbol.vocab_id as usize).expect("vocab").text.as_str();
                    } else {
                        let m = matches.get(output_symbol.match_index).expect("match index must exist");
                        text += m.text;
                    }
                    text += " | ";
                }
                eprintln!("    (text={})", text);
            }
            if score > best_score {
                best_score = score;
                best_sequence = Some(sequence);
            }
        }

        //return matches corresponding to best sequence
        best_sequence.expect("there must be a best sequence").output_symbols.into_iter().map(|osym| {
            let m = matches.get(osym.match_index).expect("match should be in bounds");
            let mut m = m.clone();
            m.selected = osym.variant_index;
            m
        }).collect()
    }


    /// Computes the logprob and perplexity for a given sequence
    pub fn lm_score<'a>(&self, sequence: &Sequence, matches: &[Match<'a>], boundaries: &[Match<'a>]) -> (f32,f64) {

        //step 1: collect all tokens in the sequence

        let mut tokens: Vec<Option<VocabId>> = Vec::with_capacity(sequence.output_symbols.len() + 5); //little bit of extra space to prevent needing to reallocate too quickly and to hold the BOS/EOS markers
        tokens.push(Some(BOS));


        for output_symbol in sequence.output_symbols.iter() {
            let m = matches.get(output_symbol.match_index).expect("match should be in bounds");
            let next_boundary = boundaries.get(output_symbol.boundary_index).expect("boundary should be in bounds");

            if output_symbol.vocab_id == 0  {
                //out of vocabulary (copied from input)
                tokens.push(None);
            } else {
                let mut ngram = self.into_ngram(output_symbol.vocab_id, &mut None);
                loop {
                    match ngram.pop_first() {
                        NGram::Empty => break,
                        unigram => tokens.push(unigram.first())
                    }
                }
            }

            //add boundary as a token too
            if !next_boundary.text.trim().is_empty() {
                if let Some(vocab_id) = self.encoder.get(next_boundary.text.trim()) {
                let mut ngram = self.into_ngram(*vocab_id, &mut None);
                    loop {
                        match ngram.pop_first() {
                            NGram::Empty => break,
                            unigram => tokens.push(unigram.first())
                        }
                    }
                } else {
                    //out of vocabulary boundary tokens (copied from input)
                    tokens.push(None);
                }
            }

        }

        tokens.push(Some(EOS));


        //move a sliding window over the tokens
        let mut logprob = 0.0;
        let mut n = 0;
        for i in 1..=tokens.len() - 1 {
            if let Ok(bigram) = NGram::from_option_list(&tokens[i-1..i+1]) {
                let prior = NGram::from_option_list(&tokens[i-1..i]).expect("extracting prior");

                let priorcount = if let Some(priorcount) = self.ngrams.get(&prior) {
                    *priorcount
                } else {
                    1
                };

                //Do we have a joint probability for the bigram that forms the transition?
                if let Some(jointcount) = self.ngrams.get(&bigram) {
                    if priorcount < *jointcount {
                        //sanity check, shouldn't be the case, correct:
                        logprob +=  (*jointcount as f32).ln()
                    } else {
                        logprob += (*jointcount as f32 / priorcount as f32).ln()
                    }
                } else {
                    logprob += TRANSITION_SMOOTHING_LOGPROB
                }

                n += 1;
            } else {
                //if we have an out of vocabulary bigram or prior we fall back to add-on smoothing
                //simply setting the count of that ngram/prior to 1
                //for the perplexity computation this means the score doesn't change, but n does
                //increase (so we end up with a lower perplexity)
                n += 1;
                logprob += TRANSITION_SMOOTHING_LOGPROB
            }


        }

        //PP(W) = (1/P(w1...wN))^(1/N)
        // in logspace: PP(W) = -1.0/N * Log(P(w1...Wn))

        let perplexity = -1.0/(n as f64) * logprob as f64;
        (logprob, perplexity)
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
        let parts = find_match_ngrams(m.text, internal, 1, 0, None);
        let mut ngram = NGram::Empty;
        for part in parts {
            if let Some(vocabid) = self.encoder.get(part.text) {
                ngram.push(*vocabid);
            } else {
                return Err(format!("unable to convert match to ngram, contains out-of-vocabulary token: {}", part.text));
            }
        }
        Ok(ngram)
    }


}
