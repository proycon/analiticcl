extern crate ibig;
extern crate num_traits;
extern crate sesdiff;
extern crate rayon;
extern crate rustfst;
extern crate simple_error;

use std::fs::File;
use std::io::{BufReader,BufRead};
use std::collections::{HashMap,BTreeMap,BTreeSet};
use std::cmp::min;
use sesdiff::shortest_edit_script;
use std::time::SystemTime;
use std::sync::Arc;
use std::cmp::Ordering;
use std::str::FromStr;
use std::error::Error;
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

/// An absolute maximum on the anagram distance, even for long inputs
const MAX_ANAGRAM_DISTANCE: u8 = 12;

/// An absolute maximum on the edit distance, even for long inputs
const MAX_EDIT_DISTANCE: u8 = 12;


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

    /// Do we have frequency information for variant matching?
    pub have_freq: bool,

    /// Do we have an LM?
    pub have_lm: bool,

    /// Context rules
    pub context_rules: Vec<ContextRule>,
    /// Tags used by the context rules
    pub tags: Vec<String>,

    ///Weights used in distance scoring
    pub weights: Weights,

    /// Stores the names of the loaded lexicons, they will be referenced by index from individual
    /// items for provenance reasons
    pub lexicons: Vec<String>,

    /// Holds weighted confusable recipes that can be used in scoring and ranking
    pub confusables: Vec<Confusable>,

    ///Process confusables before pruning by max_matches
    pub confusables_before_pruning: bool,

    pub debug: u8
}

impl VariantModel {
    /// Instantiate a new variant model
    pub fn new(alphabet_file: &str, weights: Weights, debug: u8) -> VariantModel {
        let mut model = VariantModel {
            alphabet: Vec::new(),
            encoder: HashMap::new(),
            decoder: Vec::new(),
            index: HashMap::new(),
            sortedindex: BTreeMap::new(),
            ngrams: HashMap::new(),
            freq_sum: vec!(0),
            have_freq: false,
            have_lm: false,
            weights,
            lexicons: Vec::new(),
            confusables: Vec::new(),
            confusables_before_pruning: false,
            context_rules: Vec::new(),
            tags: Vec::new(),
            debug,
        };
        model.read_alphabet(alphabet_file).expect("Error loading alphabet file");
        init_vocab(&mut model.decoder, &mut model.encoder);
        model
    }

    /// Instantiate a new variant model, explicitly passing an alphabet rather than loading one
    /// from file.
    pub fn new_with_alphabet(alphabet: Alphabet, weights: Weights, debug: u8) -> VariantModel {
        let mut model = VariantModel {
            alphabet: alphabet,
            decoder: Vec::new(),
            encoder: HashMap::new(),
            index: HashMap::new(),
            sortedindex: BTreeMap::new(),
            ngrams: HashMap::new(),
            freq_sum: vec!(0),
            have_freq: false,
            have_lm: false,
            weights,
            lexicons: Vec::new(),
            confusables: Vec::new(),
            confusables_before_pruning: false,
            context_rules: Vec::new(),
            tags: Vec::new(),
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
        eprintln!("Computing anagram values for all items in the lexicon...");

        // Hash all strings in the lexicon
        // and add them to the index
        let mut tmp_hashes: Vec<(AnaValue,VocabId)> = Vec::with_capacity(self.decoder.len());
        for (id, value)  in self.decoder.iter().enumerate() {
            if value.vocabtype.check(VocabType::INDEXED) {
                //get the anahash
                let anahash = value.text.anahash(&self.alphabet);
                if self.debug >= 2 {
                    eprintln!("   -- Anavalue={} VocabId={} Text={}", &anahash, id, value.text);
                }
                tmp_hashes.push((anahash, id as VocabId));
            }
        }
        eprintln!(" - Found {} instances",tmp_hashes.len());


        eprintln!("Adding all instances to the index...");
        self.index.clear();
        for (anahash, id) in tmp_hashes {
            //add it to the index
            let node = self.get_or_create_index(&anahash);
            node.instances.push(id);
        }
        eprintln!(" - Found {} anagrams", self.index.len() );

        eprintln!("Creating sorted secondary index...");
        self.sortedindex.clear();
        for (anahash, node) in self.index.iter() {
            if !self.sortedindex.contains_key(&node.charcount) {
                self.sortedindex.insert(node.charcount, Vec::new());
            }
            let keys = self.sortedindex.get_mut(&node.charcount).expect("getting sorted index (1)");
            keys.push(anahash.clone());  //TODO: see if we can make this a reference later
        }

        eprintln!("Sorting secondary index...");
        let mut sizes: Vec<u16> = self.sortedindex.keys().map(|x| *x).collect();
        sizes.sort();
        for size in sizes {
            let keys = self.sortedindex.get_mut(&size).expect("getting sorted index (2)");
            keys.sort();
            eprintln!(" - Found {} anagrams of length {}", keys.len(), size );
        }

        eprintln!("Constructing Language Model...");

        //extra unigrams extracted from n-grams that need to be added to the vocabulary decoder
        let mut unseen_parts: Option<VocabEncoder> = Some(VocabEncoder::new());

        for id in 0..self.decoder.len() {
            if self.decoder.get(id).expect("item").vocabtype.check(VocabType::LM) {
                //get the ngram and find any unseen parts
                if let Ok(ngram) = self.into_ngram(id as VocabId, &mut unseen_parts) {

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
            }
        }

        if let Some(unseen_parts) = unseen_parts {
            //add collected unseen n-gram parts to the decoder
            for (part, id) in unseen_parts {
                self.add_ngram(NGram::UniGram(id), 1);
                self.encoder.insert(part.clone(), id);
                self.decoder.push(VocabValue::new(part, VocabType::LM));
            }
        }
        if self.ngrams.is_empty() {
            eprintln!(" - No language model provided");
            self.have_lm = false;
        } else {
            eprintln!(" - Found {} n-grams for language modelling", self.ngrams.len() );
            self.have_lm = true;
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

    /// Decomposes and decodes and anagram value into the characters that make it up.
    /// Mostly intended for debugging purposes.
    pub fn decompose_anavalue(&self, av: &AnaValue) -> Vec<&str> {
        let mut result = Vec::new();
        for c in av.iter(self.alphabet_size()) {
            result.push(self.alphabet.get(c.0.charindex as usize).expect("alphabet item must exist").get(0).unwrap().as_str());
        }
        result
    }


    ///Read the alphabet from a TSV file
    ///The file contains one alphabet entry per line, but may
    ///consist of multiple tab-separated alphabet entries on that line, which
    ///will be treated as the identical.
    ///The alphabet is not limited to single characters but may consist
    ///of longer string, a greedy matching approach will be used so order
    ///matters (but only for this)
    pub fn read_alphabet(&mut self, filename: &str) -> Result<(), std::io::Error> {
        if self.debug >= 1 {
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
        if self.debug >= 1 {
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
        if self.debug >= 1 {
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
        if self.debug >= 1 {
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


    /// Add a (weighted) variant to the model, referring to a reference that already exists in
    /// the model.
    /// Variants will be added
    /// to the lexicon automatically when necessary. Set VocabType::TRANSPARENT
    /// if you want variants to only be used as an intermediate towards items that
    /// have already been added previously through a more authoritative lexicon.
    pub fn add_variant(&mut self, ref_id: VocabId, variant: &str, score: f64, freq: Option<u32>, params: &VocabParams) -> bool {
        let variantid = self.add_to_vocabulary(variant, freq, &params);
        self.add_variant_by_id(ref_id, variantid, score)
    }

    /// Add a (weighted) variant to the model, referring to a reference that already exists in
    /// the model.
    /// Variants will be added
    /// to the lexicon automatically when necessary. Set VocabType::TRANSPARENT
    /// if you want variants to only be used as an intermediate towards items that
    /// have already been added previously through a more authoritative lexicon.
    pub fn add_variant_by_id(&mut self, ref_id: VocabId, variantid: VocabId, score: f64) -> bool {
        if variantid != ref_id {
            //link reference to variant
            if let Some(vocabvalue) = self.decoder.get_mut(ref_id as usize) {
                let variantref = VariantReference::ReferenceFor((variantid,score) );
                if vocabvalue.variants.is_none() {
                    vocabvalue.variants = Some(vec!(variantref));
                } else if let Some(variantrefs) = vocabvalue.variants.as_mut() {
                    //only add if it doesn't already exists (only first mention counts, regardless of score)
                    if !variantrefs.iter().any(|x| match x {
                        VariantReference::ReferenceFor((y,_)) => variantid == *y,
                        _ => false,
                    }) {
                        variantrefs.push(variantref);
                    }
                }
            }
            //link variant to reference
            if let Some(vocabvalue) = self.decoder.get_mut(variantid as usize) {
                let variantref = VariantReference::VariantOf((ref_id,score) );
                if vocabvalue.variants.is_none() {
                    vocabvalue.variants = Some(vec!(variantref));
                } else if let Some(variantrefs) = vocabvalue.variants.as_mut() {
                    //only add if it doesn't already exists (only first mention counts, regardless of score)
                    if !variantrefs.iter().any(|x| match x {
                        VariantReference::VariantOf((y,_)) => variantid == *y,
                        _ => false,
                    }) {
                        variantrefs.push(variantref);
                    }
                }
            }
            true
        } else {
            false
        }
    }


    ///Read vocabulary (a lexicon or corpus-derived lexicon) from a TSV file
    ///May contain frequency information
    ///The parameters define what value can be read from what column
    pub fn read_vocabulary(&mut self, filename: &str, params: &VocabParams) -> Result<(), std::io::Error> {
        if self.debug >= 1 {
            eprintln!("Reading vocabulary #{} from {} ({:?})...", self.lexicons.len() + 1, filename, params.vocab_type);
        }
        let beginlen = self.decoder.len();
        let f = File::open(filename)?;
        let f_buffer = BufReader::new(f);
        let mut params = params.clone();
        params.index = self.lexicons.len() as u8;
        for line in f_buffer.lines() {
            if let Ok(line) = line {
                if !line.is_empty() {
                    let fields: Vec<&str> = line.split("\t").collect();
                    let text = fields.get(params.text_column as usize).expect("Expected text column not found");
                    let frequency = if let Some(freq_column) = params.freq_column {
                        if params.vocab_type.check(VocabType::INDEXED) {
                            self.have_freq = true;
                        }
                        fields.get(freq_column as usize).unwrap_or(&"1").parse::<u32>().expect("frequency should be a valid integer")
                    } else {
                        1
                    };
                    self.add_to_vocabulary(text, Some(frequency), &params);
                }
            }
        }
        if self.debug >= 1 {
            eprintln!(" - Read vocabulary of size {}", self.decoder.len() - beginlen);
        }
        self.lexicons.push(filename.to_string());
        Ok(())
    }

    pub fn read_contextrules(&mut self, filename: &str) -> Result<(), std::io::Error> {
        if self.debug >= 1 {
            eprintln!("Reading context rules {}...",  filename);
        }
        let f = File::open(filename)?;
        let f_buffer = BufReader::new(f);
        for line in f_buffer.lines() {
            if let Ok(line) = line {
                if !line.is_empty() && !line.starts_with('#') {
                    let fields: Vec<&str> = line.split("\t").collect();
                    if fields.len() < 2 {
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Expected at least two columns in context rules file"));
                    }

                    let pattern: &str = fields.get(0).unwrap();
                    let score = fields.get(1).unwrap().parse::<f32>().expect("context rule score should be a floating point value above or below 1.0");

                    let tag: Vec<&str> = match fields.get(2) {
                        Some(s) => s.split(";").map(|w| w.trim()).collect(),
                        None => Vec::new()
                    };

                    let mut tagoffset: Vec<&str> = match fields.get(3) {
                        Some(s) => s.split(";").map(|w| w.trim()).collect(),
                        None => Vec::new()
                    };

                    if tag.len() == 1  && tagoffset.len() == 0 {
                        tagoffset.push("0:");
                    } else if tag.len() != tagoffset.len() {
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Multiple tags are specified, expected the same number of tag offsets! (semicolon separated)"));
                    }

                    self.add_contextrule(pattern, score, tag, tagoffset);

                }
            }
        }
        if self.debug >= 1 {
            eprintln!(" -- Read {} context rules", self.context_rules.len());
        }

        Ok(())
    }


    pub fn add_contextrule(&mut self, pattern: &str, score: f32, tag: Vec<&str>, tagoffset: Vec<&str>) {
        let expressions: Vec<&str> = pattern.split(";").map(|s| s.trim()).collect();
        let mut pattern: Vec<PatternMatch> = Vec::new();
        for expr in expressions {
            match PatternMatch::parse(expr, &self.lexicons, &self.encoder) {
                Ok(pm) => pattern.push(pm),
                Err(err) => eprintln!("Error parsing context rule: {}", err)
            }
        }

        let tag: Vec<u16> = tag.iter().map(|tag| {
            let mut pos = None;
            for (i, t) in self.tags.iter().enumerate() {
                if t == tag {
                    pos = Some(i as u16);
                    break;
                }
            }
            if pos.is_none() {
                self.tags.push(tag.to_string());
                (self.tags.len() - 1) as u16
            } else {
                pos.unwrap()
            }
        }).collect();


        let mut tagoffset: Vec<(u8,u8)> = tagoffset.iter().map(|s| {
            let fields: Vec<&str> = s.split(":").collect();
            let tagbegin: u8 = if let Some(tagbegin) = fields.get(0) {
                if tagbegin.is_empty() {
                    0
                } else {
                    tagbegin.parse::<u8>().expect("tag offset should be an integer")
                }
            } else {
                0
            };
            let taglength: u8 = if let Some(taglength) = fields.get(1) {
                if taglength.is_empty() {
                    pattern.len() as u8 - tagbegin
                } else {
                    taglength.parse::<u8>().expect("tag length should be an integer")
                }
            } else {
                pattern.len() as u8 - tagbegin
            };

            (tagbegin,taglength)
        }).collect();

        while tagoffset.len() < tag.len() {
            tagoffset.push((0,pattern.len() as u8));
        }

        if !pattern.is_empty() {
            self.context_rules.push( ContextRule {
                pattern,
                score,
                tag,
                tagoffset
            });
        }
    }

    ///Read a weighted variant list from a TSV file. Contains a canonical/reference form in the
    ///first column, and variants with score (two columns) in the following columns. May also
    ///contain frequency information (auto detected), in which case the first column has the
    ///canonical/reference form, the second column the frequency, and all further columns hold
    ///variants, their score and their frequency (three columns).
    ///Consumes much more memory than equally weighted variants.
    pub fn read_variants(&mut self, filename: &str, params: Option<&VocabParams>, transparent: bool) -> Result<(), std::io::Error> {
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
        let transparent_params = if transparent {
            let mut p = params.clone();
            p.vocab_type |= VocabType::TRANSPARENT;
            p
        } else {
            params.clone()
        };

        if self.debug >= 1 {
            eprintln!("Reading variants from {}...", filename);
        }
        let mut count = 0;
        let mut has_freq = None;
        let f = File::open(filename)?;
        let f_buffer = BufReader::new(f);
        for (linenr, line) in f_buffer.lines().enumerate() {
            let linenr = linenr + 1;
            if let Ok(line) = line {
                if !line.is_empty() {
                    let fields: Vec<&str> = line.split("\t").collect();
                    let reference = fields.get(0).expect(format!("reference item (line {}, column 1, of {})", linenr, filename).as_str());
                    let freq = if has_freq.is_none() {
                        //autodetect whether we have frequency information or not
                        if (fields.len() - 2) % 3 == 0 {
                            let freq = fields.get(1).expect("second field");
                            match freq.parse::<u32>() {
                                Ok(freq) => {
                                    has_freq = Some(true);
                                    Some(freq)
                                },
                                _ => None
                            }
                        } else {
                            //number of columns not consistent with holding frequency information
                            has_freq = Some(false);
                            None
                        }
                    } else if has_freq == Some(true) {
                        let freq = fields.get(1).expect("score of reference item");
                        Some(freq.parse::<u32>().expect(format!("Frequency must be an integer (line {}, column 2, of {})", linenr, filename).as_str()))
                    } else {
                        None
                    };
                    let ref_id = self.add_to_vocabulary(reference, freq, &params);
                    let mut iter = fields.iter();

                    if has_freq == Some(true) {
                        iter.next(); iter.next();
                        while let (Some(variant), Some(score), Some(freq)) = (iter.next(), iter.next(), iter.next()) {
                            let score = score.parse::<f64>().expect(format!("Variant scores must be a floating point value (line {} of {}, got {} instead), also parsing frequency", linenr, filename, score).as_str());
                            let freq = freq.parse::<u32>().expect(format!("Variant frequency must be an integer (line {} of {}), got {} instead", linenr, filename, freq).as_str());
                            if self.add_variant(ref_id, variant, score, Some(freq), if transparent { &transparent_params } else { &params } ) {
                                count += 1;
                            }
                        }
                    } else {
                        iter.next();
                        while let (Some(variant), Some(score)) = (iter.next(), iter.next()) {
                            let score = score.parse::<f64>().expect(format!("Variant scores must be a floating point value (line {} of {}, got {}), no frequency information", linenr, filename, score).as_str());
                            if self.add_variant(ref_id, variant, score, None, if transparent { &transparent_params } else { &params } ) {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        if self.debug >= 1 {
            eprintln!(" - Read weighted variants list, added {} references", count);
        }
        self.lexicons.push(filename.to_string());
        Ok(())
    }



    /// Adds an entry in the vocabulary
    pub fn add_to_vocabulary(&mut self, text: &str, frequency: Option<u32>, params: &VocabParams) -> VocabId {
        let frequency = frequency.unwrap_or(1);
        if self.debug >= 2 {
            eprintln!(" -- Adding to vocabulary: {}  ({})", text, frequency);
        }
        if let Some(vocab_id) = self.encoder.get(text) {
            let item = self.decoder.get_mut(*vocab_id as usize).expect(&format!("Retrieving existing vocabulary entry {}",vocab_id));
            match params.freq_handling {
                FrequencyHandling::Sum => {
                    item.frequency += frequency;
                },
                FrequencyHandling::Max => {
                    if frequency > item.frequency {
                        item.frequency  = frequency;
                    };
                },
                FrequencyHandling::Min => {
                    if frequency < item.frequency {
                        item.frequency  = frequency;
                    };
                },
                FrequencyHandling::Replace => {
                    item.frequency = frequency;
                },
            }
            if vocab_id == &BOS || vocab_id == &EOS || vocab_id == &UNK {
                item.vocabtype = VocabType::LM; //by definition
            } else if item.vocabtype.check(VocabType::TRANSPARENT) && !params.vocab_type.check(VocabType::TRANSPARENT) {
                //we can lose the transparency flag if a later lexicon doesn't provide it
                item.vocabtype ^= VocabType::TRANSPARENT;
            }
            item.lexindex |= 1 << params.index;
            if self.debug >= 3 {
                eprintln!("    (updated) freq={}, lexindex+={}", item.frequency, params.index);
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
                lexindex:  1 << params.index,
                variants: None,
                vocabtype: params.vocab_type
            });
            if self.debug >= 3 {
                eprintln!("    (new) lexindex={}", params.index);
            }
            self.decoder.len() as VocabId - 1
        }
    }

    /// Find variants in the vocabulary for a given string (in its totality), returns a vector of vocabulary ID and score pairs
    /// Returns a vector of three-tuples (VocabId, distance_score, freq_score)
    /// The resulting vocabulary Ids can be resolved through `get_vocab()`
    pub fn find_variants(&self, input: &str, params: &SearchParameters) -> Vec<VariantResult> {

        if self.index.is_empty()  {
            eprintln!("ERROR: Model has not been built yet! Call build() before find_variants()");
            return vec!();
        }

        //Compute the anahash
        let normstring = input.normalize_to_alphabet(&self.alphabet);
        let anahash = input.anahash(&self.alphabet);

        let max_anagram_distance: u8 = match params.max_anagram_distance {
            DistanceThreshold::Ratio(x) => min(
                (normstring.len() as f32 * x).floor() as u8,
                MAX_ANAGRAM_DISTANCE, //absolute maximum as a safeguard
            ),
            DistanceThreshold::RatioWithLimit(x,limit) => min(
                (normstring.len() as f32 * x).floor() as u8,
                limit,
            ),
            DistanceThreshold::Absolute(x) => min(
                    x,
                    (normstring.len() as f64 / 2.0).floor() as u8 //we still override the absolute threshold when dealing with very small inputs
            )
        };

        //Compute neighbouring anahashes and find the nearest anahashes in the model
        let anahashes = self.find_nearest_anahashes(&anahash,
                                                    max_anagram_distance,
                                                    params.stop_criterion);


        let max_edit_distance: u8 = match params.max_edit_distance {
            DistanceThreshold::Ratio(x) => min(
                (normstring.len() as f32 * x).floor() as u8,
                MAX_EDIT_DISTANCE, //absolute maximum as a safeguard
            ),
            DistanceThreshold::RatioWithLimit(x,limit) => min(
                (normstring.len() as f32 * x).floor() as u8,
                limit,
            ),
            DistanceThreshold::Absolute(x) => min(
                    x,
                    (normstring.len() as f64 / 2.0).floor() as u8 //we still override the absolute threshold when dealing with very small inputs
            )
        };

        //Get the instances pertaining to the collected hashes, within a certain maximum distance
        //and compute distances
        let variants = self.gather_instances(&anahashes, &normstring, input, max_edit_distance);

        self.score_and_rank(variants, input, normstring.len(), params.max_matches, params.score_threshold, params.cutoff_threshold, params.freq_weight)
    }


    ///Auxiliary function used by [`learn_variants()`], abstracts over strict mode
    fn find_variants_for_learning<'a>(&self, inputstr: &'a str, params: &SearchParameters, strict: bool) -> Vec<(&'a str, VariantResult)> {
        if strict {
            self.find_variants(inputstr, params).into_iter().map(|result| (inputstr, result)).collect()
        } else {
            self.find_all_matches(inputstr, params).iter().filter_map(|result_match| {
                if let Some(variants) = &result_match.variants {
                    if let Some(selected) = result_match.selected {
                        if let Some(result) = variants.get(selected) {
                            return Some((result_match.text, result.clone()));
                        }
                    }
                }
                None
            }).collect()
        }
    }

    /// Processes input and finds variants (like [`find_variants()`]), but all variants that are found (which meet
    /// the set thresholds) will be stored in the model rather than returned. Unlike `find_variants()`, this is
    /// invoked with an iterator over multiple inputs and returns no output by itself. It
    /// will automatically apply parallellisation.
    pub fn learn_variants<'a, I>(&mut self, input: I, params: &SearchParameters, strict: bool, auto_build: bool) -> usize
    where
        I: IntoParallelIterator<Item = &'a String> + IntoIterator<Item = &'a String>,
    {
        if self.debug >= 1 {
            eprintln!("(Learning variants)");
        }

        let vocabparams = VocabParams::default().with_vocab_type(VocabType::TRANSPARENT).with_freq_handling(FrequencyHandling::Max);

        let mut all_variants: Vec<Vec<(&'a str, VariantResult)>> = Vec::new();
        if params.single_thread {
            all_variants.extend( input.into_iter().map(|inputstr| {
                self.find_variants_for_learning(inputstr.as_str(), params, strict)
            }));
        } else {
            all_variants.par_extend( input.into_par_iter().map(|inputstr| {
                self.find_variants_for_learning(inputstr.as_str(), params, strict)
            }));
        }

        if self.debug >= 1 {
            eprintln!("(adding variants over {} input items to the model)", all_variants.len());
        }


        let mut count = 0;
        let mut prev = None;
        for (inputstr, result) in all_variants.into_iter().flatten() {
            //get a vocabulary id for the input string;
            //adding it to the vocabulary if it does not exist yet
            let vocab_id = if let Some(vocab_id) = self.encoder.get(inputstr) {
                //item exists
                let vocabitem = self.decoder.get_mut(*vocab_id as usize).expect("item must exist");
                //is this the first occurrence in a consecutive sequence?
                if prev != Some(inputstr) {
                    //then increment the frequency
                    vocabitem.frequency += 1;
                }
                *vocab_id
            } else {
                //item is new
                self.add_to_vocabulary(inputstr, Some(1), &vocabparams)
            };
            if result.vocab_id != vocab_id { //ensure we don't add exact matches
                if self.add_variant_by_id(result.vocab_id, vocab_id, result.dist_score) {
                    count += 1;
                }
            }
            prev = Some(inputstr);
        }

        if self.debug >= 1 {
            eprintln!("(added {} variants)", count);
        }

        if auto_build {
            if self.debug >= 1 {
                eprintln!("((re)building the model)");
            }
            self.build();
        }
        count
    }


    /// Find the nearest anahashes that exists in the model (computing anahashes in the
    /// neigbhourhood if needed).
    pub(crate) fn find_nearest_anahashes<'a>(&'a self, focus: &AnaValue, max_distance: u8,  stop_criterion: StopCriterion) -> BTreeSet<&'a AnaValue> {
        let mut nearest: BTreeSet<&AnaValue> = BTreeSet::new();

        let begintime = if self.debug >= 2 {
            eprintln!("(finding nearest anagram matches for focus anavalue {}, max_distance={}, stop_criterion={:?})", focus, max_distance, stop_criterion);
            Some(SystemTime::now())
        } else {
            None
        };

        if let Some((matched_anahash, node)) = self.index.get_key_value(focus) {
            //the easiest case, this anahash exists in the model!
            if self.debug >= 2 {
                eprintln!(" (found exact match)");
            }
            nearest.insert(matched_anahash);
            if StopCriterion::StopAtExactMatch == stop_criterion {
                for vocab_id in node.instances.iter() {
                    if let Some(_) = self.decoder.get(*vocab_id as usize) {
                        if self.debug >= 2 {
                            eprintln!(" (stopping early)");
                        }
                        return nearest;
                    }
                }
            }
        }

        let (focus_upper_bound, focus_charcount) = focus.alphabet_upper_bound(self.alphabet_size());
        let focus_alphabet_size = focus_upper_bound + 1;

        // Gather lookups to match against the secondary index
        // keys correspond to the number of characters
        // We first gather all lookups rather than doing them immediately,
        // so we need to iterate over the secondary index only once, which
        // has a slight performance benefit
        let mut lookups: HashMap<u8,Vec<AnaValue>> = HashMap::new();

        //Find anagrams reachable through insertions within the the maximum distance
        for distance in 1..=max_distance {
            let search_charcount = focus_charcount + distance as u16;
            if let Some(lookups) = lookups.get_mut(&(search_charcount as u8)) {
                lookups.push(focus.clone());
            } else {
                lookups.insert(search_charcount as u8, vec!(focus.clone()));
            }
            if self.debug >= 3 {
                eprintln!(" (scheduling finding insertion at distance {}, charcount {})", distance, search_charcount);
            }
        }


        let searchparams = SearchParams {
            max_distance: Some(max_distance as u32),
            breadthfirst: true,
            allow_empty_leaves: false,
            allow_duplicates: false,
            ..Default::default()
        };


        /*let iterator = if let Some(cache) = cache {
            focus.iter_recursive_external_cache(focus_alphabet_size+1, &searchparams, cache)
        } else {*/
        let iterator = focus.iter_recursive(focus_alphabet_size+1, &searchparams);
        /*};*/


        // Do a breadth first search for deletions
        for (deletion,distance) in iterator {
            if self.debug >= 3 {
                eprintln!(" (testing deletion at distance {}, charcount {}: anavalue {})", distance, focus_charcount as u32 - distance, deletion.value);
                if self.debug >= 4 {
                    let decomposed: String = self.decompose_anavalue(&deletion.value).join("");
                    eprintln!("  (anavalue decomposition: {})", decomposed);
                }
            }

            if let Some((matched_anahash, _node)) = self.index.get_key_value(&deletion) {
                if self.debug >= 3 {
                    eprintln!("  (deletion matches; anagram exists in index)");
                }
                //This deletion exists in the model
                nearest.insert(matched_anahash);
            }

            let deletion_charcount = focus_charcount - distance as u16;
            if self.debug >= 3 {
                eprintln!("  (scheduling search for insertions from deletion result anavalue {})",  deletion.value);
            }
            //Find possible insertions starting from this deletion
            for search_distance in 1..=(max_distance as u16 - distance as u16) {
                let search_charcount = deletion_charcount + search_distance;
                if self.debug >= 3 {
                    eprintln!("   (search_distance={}, search_charcount={})", search_distance, search_charcount);
                }
                if let Some(lookups) = lookups.get_mut(&(search_charcount as u8)) {
                    lookups.push(deletion.value.clone());
                } else {
                    lookups.insert(search_charcount as u8, vec!(deletion.value.clone()));
                }
            }
        }


        if self.debug >= 2 {
            eprintln!("(finding all insertions)");
        }
        let mut count = 0;
        let beginlength = nearest.len();
        for (search_charcount, anavalues) in lookups.iter() {
            if let Some(sortedindex) = self.sortedindex.get(&(*search_charcount as u16)) {
                for candidate in sortedindex.iter() {
                    for av in anavalues {
                        if candidate.contains(&av) {//this is where the magic happens
                            count += 1;
                            nearest.insert(candidate);
                            break;
                        }
                    }
                }
            }
        }
        if self.debug >= 2 {
            eprintln!(" (added {} out of {} candidates, preventing duplicates)", nearest.len() - beginlength , count);
        }


        if self.debug >= 2 {
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


    /// Gather instances with their edit distances and frequency, given a search string (normalised to the alphabet) and anagram hashes
    pub(crate) fn gather_instances(&self, nearest_anagrams: &BTreeSet<&AnaValue>, querystring: &[u8], query: &str, max_edit_distance: u8) -> Vec<(VocabId,Distance)> {
        let mut found_instances = Vec::new();
        let mut pruned_instances = 0;

        let begintime = if self.debug >= 2 {
            Some(SystemTime::now())
        } else {
            None
        };

        for anahash in nearest_anagrams {
            let node = self.index.get(anahash).expect("all anahashes from nearest_anagrams must occur in the index");
            for vocab_id in node.instances.iter() {
                let vocabitem = self.decoder.get(*vocab_id as usize).expect("vocabulary id must exist in the decoder");
                if self.debug >= 4 {
                    eprintln!("  (comparing query {} with instance {})", query, vocabitem.text)
                }
                if let Some(ld) = damerau_levenshtein(querystring, &vocabitem.norm, max_edit_distance) {
                    if self.debug >= 4 {
                        eprintln!("   (ld={})", ld);
                    }
                    //we only get here if we make the max_edit_distance cut-off
                    let distance = Distance {
                        ld: ld,
                        lcs: if self.weights.lcs > 0.0 { longest_common_substring_length(querystring, &vocabitem.norm) } else { 0 },
                        prefixlen: if self.weights.prefix > 0.0 { common_prefix_length(querystring, &vocabitem.norm) } else { 0 },
                        suffixlen: if self.weights.suffix > 0.0 { common_suffix_length(querystring, &vocabitem.norm) } else { 0 },
                        samecase: if self.weights.case > 0.0 { vocabitem.text.chars().next().expect("first char").is_lowercase() == query.chars().next().expect("first char").is_lowercase() } else { true },
                    };
                    //match will be added to found_instances at the end of the block (we
                    //need to borrow the distance for a bit still)

                    //add the original match
                    found_instances.push((*vocab_id,distance));
                } else {
                    if self.debug >= 4 {
                        eprintln!("   (exceeds max_edit_distance {})", max_edit_distance);
                    }
                    pruned_instances += 1;
                }
            }
        }
        //found_instances.sort_unstable_by_key(|k| k.1 ); //sort by distance, ascending order
        if self.debug >= 2 {
            let endtime = SystemTime::now();
            let duration = endtime.duration_since(begintime.expect("begintime")).expect("clock can't go backwards").as_micros();
            eprintln!("(found {} instances (pruned {} above max_edit_distance {}) over {} anagrams in {} μs)", found_instances.len(), pruned_instances, max_edit_distance,  nearest_anagrams.len(), duration);
        }
        found_instances
    }



    /// Rank and score all variants, returns a vector of three-tuples: (VocabId, distance score, frequency score)
    pub(crate) fn score_and_rank(&self, instances: Vec<(VocabId,Distance)>, input: &str, input_length: usize, max_matches: usize, score_threshold: f64, cutoff_threshold: f64, freq_weight: f32) -> Vec<VariantResult> {
        let mut results: Vec<VariantResult> = Vec::new();
        let mut max_freq = 0.0;
        let mut has_expandable_variants = false;
        let weights_sum = self.weights.sum();

        assert!(input_length > 0);

        let begintime = if self.debug >= 2 {
            eprintln!("(scoring and ranking {} instances)", instances.len());
            Some(SystemTime::now())
        } else {
            None
        };


        //Compute scores
        for (vocab_id, distance) in instances.iter() {
            if let Some(vocabitem) = self.decoder.get(*vocab_id as usize) {
                //all scores are expressed in relation to the input length
                let distance_score: f64 = if distance.ld as usize > input_length {
                    0.0
                } else {
                    1.0 - (distance.ld as f64 / input_length as f64)
                };
                let lcs_score: f64 = distance.lcs as f64 / input_length as f64;
                let prefix_score: f64 = distance.prefixlen as f64 / input_length as f64;
                let suffix_score: f64 = distance.suffixlen as f64 / input_length as f64;
                //simple weighted linear combination (arithmetic mean to normalize it again) over all normalized distance factors
                //expresses a similarity score, sensitive to the length of the input string, and where an exact match by default is 1.0
                let score = (
                    self.weights.ld * distance_score +
                    self.weights.lcs * lcs_score +
                    self.weights.prefix * prefix_score +
                    self.weights.suffix * suffix_score +
                    if distance.samecase { self.weights.case } else { 0.0 }
                ) / weights_sum;

                let freq_score: f64 = if self.have_freq { //absolute frequency, normalisation in later pass
                    vocabitem.frequency as f64
                } else {
                    1.0
                };
                if freq_score > max_freq {
                    max_freq = freq_score;
                }

                if !has_expandable_variants && vocabitem.variants.is_some() {
                    has_expandable_variants = true;
                }

                if score.is_nan() {
                    //should never happen
                    panic!("Invalid score (NaN) computed for variant={}, distance={:?}, score={}", vocabitem.text, distance, score);
                }
                if score >= score_threshold  {
                    results.push( VariantResult {
                        vocab_id: *vocab_id,
                        dist_score: score,
                        freq_score,
                        via: None
                    });
                    if self.debug >= 3 {
                        eprintln!("   (variant={}, distance={:?}, score={}, transparent={})", vocabitem.text, distance, score, vocabitem.vocabtype.check(VocabType::TRANSPARENT));
                    }
                } else {
                    if self.debug >= 3 {
                        eprintln!("   (PRUNED variant={}, distance={:?}, score={}, transparent={})", vocabitem.text, distance, score, vocabitem.vocabtype.check(VocabType::TRANSPARENT));
                    }
                }
            }
        }

        //rescore with confusable weights (EARLY)
        if !self.confusables.is_empty() && self.confusables_before_pruning {
            self.rescore_confusables(&mut results, input);
        }

        if has_expandable_variants {
            results = self.expand_variants(results);
            //Collect maximum frequency after expansion
            for result in results.iter() {
                if result.freq_score > max_freq {
                    max_freq = result.freq_score;
                }
            }
        }

        //normalize frequency score
        if max_freq > 0.0 {
            for result in results.iter_mut() {
                result.freq_score = result.freq_score / max_freq;
            }
        }

        //Sort the results by distance score, descending order
        self.rank_results(&mut results, freq_weight);

        if has_expandable_variants {
            //remove duplicates (can only occur when variant expansion was performed)
            results.dedup_by_key(|x| x.vocab_id);
        }

        //Crop the results at max_matches or cut off at the cutoff threshold
        if max_matches > 0 && results.len() > max_matches {
            let last_score = results.get(max_matches - 1).expect("get last score").score(freq_weight);
            let cropped_score = results.get(max_matches).expect("get cropped score").score(freq_weight);
            if cropped_score < last_score {
                if self.debug >= 2 {
                    eprintln!("   (truncating {} matches to {})", results.len(), max_matches);
                }
                //simplest case, crop at the max_matches
                results.truncate(max_matches);
            } else {
                //cropping at max_matches comes at arbitrary point of equal scoring items,
                //we crop earlier instead:
                let mut early_cutoff = 0;
                let mut late_cutoff = 0;
                for (i, result) in results.iter().enumerate() {
                    if result.dist_score == cropped_score && early_cutoff == 0 {
                        early_cutoff = i;
                    }
                    if result.dist_score < cropped_score {
                        late_cutoff = i;
                        break;
                    }
                }
                if early_cutoff > 0 {
                    if self.debug >= 2 {
                        eprintln!("   (truncating {} matches (early) to {})", results.len(), early_cutoff+1);
                    }
                    results.truncate(early_cutoff+1);
                } else if late_cutoff > 0 {
                    if self.debug >= 2 {
                        eprintln!("   (truncating {} matches (late) to {})", results.len(), late_cutoff+1);
                    }
                    results.truncate(late_cutoff+1);
                }
            }
        }


        //rescore with confusable weights (LATE, default)
        if !self.confusables.is_empty() && !self.confusables_before_pruning {
            self.rescore_confusables(&mut results, input);
            self.rank_results(&mut results, freq_weight);
        }

        // apply the cutoff threshold
        let mut cutoff = 0;
        let mut bestscore = None;
        if cutoff_threshold >= 1.0 {
            for (i, result) in results.iter().enumerate() {
                if let Some(bestscore) = bestscore {
                    if result.score(freq_weight) <= bestscore / cutoff_threshold {
                        cutoff = i;
                        break;
                    }
                } else {
                    bestscore = Some(result.score(freq_weight));
                }
            }
        }
        if cutoff > 0 {
            let l = results.len();
            results.truncate(cutoff);
            if self.debug >= 2 {
                eprintln!("   (truncating {} matches to {} due to cutoff value)", l, results.len());
            }
        }

        if self.debug >= 2 {
            for (i, result) in results.iter().enumerate() {
                if let Some(vocabitem) = self.decoder.get(result.vocab_id as usize) {
                    eprintln!("   (ranked #{}, variant={}, score={}, distance_score={}, freq_score={})", i+1, vocabitem.text, result.score(freq_weight), result.dist_score, result.freq_score);
                }
            }
        }


        if self.debug >= 2 {
            let endtime = SystemTime::now();
            let duration = endtime.duration_since(begintime.expect("begintime")).expect("clock can't go backwards").as_micros();
            eprintln!(" (scored and ranked {} results in {} μs)", results.len(), duration);
        }

        results
    }

    /// Rescore results according to confusables
    pub fn rescore_confusables(&self, results: &mut Vec<VariantResult>, input: &str) {
        if self.debug >= 2 {
            eprintln!("   (rescoring with confusable weights)");
        }
        for result in results.iter_mut() {
            result.dist_score *= self.compute_confusable_weight(input, result.vocab_id);
        }
    }

    /// Sorts a result vector of (VocabId, distance_score, freq_score)
    /// in decreasing order (best result first)
    pub fn rank_results(&self, results: &mut Vec<VariantResult>, freq_weight: f32) {
        results.sort_by(|a, b| {
            a.rank_cmp(&b, freq_weight).expect("ordering")
        });
    }

    /// Expand variants, adding all references for variants
    /// In case variants are 'transparent', only the references will be retained
    /// as results.
    /// The results list does not need to be sorted yet. This function may yield
    /// duplicates. For performance, call this only when you know there are variants that
    /// may be expanded.
    pub fn expand_variants(&self, mut results: Vec<VariantResult>) -> Vec<VariantResult> {
        if self.debug >= 3 {
            eprintln!("   (expanding variants, resolving transparency)");
        }
        let mut new_results = Vec::with_capacity(results.len());
        let mut count = 0;
        for result in results.drain(..) {
            count += 1;
            let vocabitem = self.decoder.get(result.vocab_id as usize).expect("vocabitem must exist");
            if let Some(variantrefs) = &vocabitem.variants {
                for variantref in variantrefs.iter() {
                    if let VariantReference::VariantOf((target_id,variant_dist_score)) = variantref {
                        new_results.push(VariantResult {
                            vocab_id: *target_id,
                            dist_score: result.dist_score * variant_dist_score,
                            freq_score: {
                                //take the minimum frequency of the item we refer to and the one of this variant
                                //note: frequency score is still absolute (not-normalised) at this point
                                let targetitem = self.decoder.get(*target_id as usize).expect("vocabitem must exist");
                                if (targetitem.frequency as f64) < result.freq_score {
                                    targetitem.frequency as f64
                                } else {
                                    result.freq_score
                                }
                            },
                            via: Some(result.vocab_id)
                        });
                    }
                }
            }
            if !vocabitem.vocabtype.check(VocabType::TRANSPARENT) {
                //add the original item
                new_results.push(result);
            }
        }
        if self.debug >= 3 {
            eprintln!("    (expanded {} instances to {})", count, new_results.len());
        }
        new_results
    }

    /// compute weight over known confusables
    /// Should return 1.0 when there are no known confusables
    /// < 1.0 when there are unfavourable confusables
    /// > 1.0 when there are favourable confusables
    pub fn compute_confusable_weight(&self, input: &str, candidate: VocabId) -> f64 {
        let mut weight = 1.0;
        if let Some(candidate) = self.decoder.get(candidate as usize) {
            let editscript = shortest_edit_script(input, &candidate.text, false, false, false);
            if self.debug >= 3 {
                eprintln!("   (editscript {} -> {}: {:?})", input, candidate.text, editscript);
            }
            for confusable in self.confusables.iter() {
                if confusable.found_in(&editscript) {
                    if self.debug >= 3 {
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
        if self.debug >= 2 {
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

        if text.is_empty() {
            return matches;
        }

        if self.debug >= 1 {
            eprintln!("(finding all matches in text: {})", text);
        }

        if self.index.is_empty()  {
            eprintln!("ERROR: Model has not been built yet! Call build() before find_all_matches()");
            return matches;
        }

        //Find the boundaries and classify their strength
        let boundaries = find_boundaries(text);
        let strengths = classify_boundaries(&boundaries);

        if self.debug >= 2 {
            eprintln!("  (boundaries: {:?})", boundaries);
            eprintln!("  ( strenghts: {:?})", strengths);
        }

        let mut begin: usize = 0;
        let mut begin_index: usize = 0;

        //Compose the text into batches, each batch ends where a hard boundary is found
        for (i, (strength, boundary)) in strengths.iter().zip(boundaries.iter()).enumerate() {
            if *strength == BoundaryStrength::Hard && boundary.offset.begin != begin {
                let text_current = &text[begin..boundary.offset.begin];

                let boundaries = &boundaries[begin_index..i+1];
                if self.debug >= 2 {
                    eprintln!("  (found hard boundary at {}:{}: {})", boundary.offset.begin, boundary.offset.end, text_current);
                    for boundary in boundaries.iter() {
                        eprintln!("    (inner boundary {}:{})", boundary.offset.begin, boundary.offset.end);
                    }
                }

                //Gather all segments for this batch
                let mut batch_matches: Vec<Match<'a>> = Vec::new();
                for order in 1..=params.max_ngram {
                    //Find all n-grams of this order
                    let mut currentorder_matches: Vec<Match<'a>> = find_match_ngrams(text, boundaries, order, begin, Some(boundary.offset.begin));
                    if self.debug >= 2 {
                        eprintln!("  (processing {} {}-grams: {:?})", currentorder_matches.len(), order, currentorder_matches);
                    }

                    //find variants for all segments of the current order in this batch
                    //for higher order matches, we first check if the match is not redundant
                    //(if the score of the unigrams isn't perfect already)
                    //so we don't needlessly look up variants we won't use anyway
                    if params.single_thread {
                        currentorder_matches.iter_mut().for_each(|segment| {
                            if order == 1 || !redundant_match(segment, &batch_matches) {
                                if self.debug >= 1 {
                                    eprintln!("   (----------- finding variants for: {} -----------)", segment.text);
                                }
                                let variants = self.find_variants(&segment.text, params);
                                if self.debug >= 1 {
                                    eprintln!("   (found {} variants)", variants.len());
                                }
                                segment.variants = Some(variants);
                            } else if self.debug >= 2 {
                                    eprintln!("   (skipping redundant match: {})", segment.text);
                            }
                        });
                    } else { //(in parallel)
                        currentorder_matches.par_iter_mut().for_each(|segment| {
                            if order == 1 || !redundant_match(segment, &batch_matches) {
                                if self.debug >= 1 {
                                    eprintln!("   (----------- finding variants for: {} -----------)", segment.text);
                                }
                                let variants = self.find_variants(&segment.text, params);
                                if self.debug >= 1 {
                                    eprintln!("    (found {} variants)", variants.len());
                                }
                                segment.variants = Some(variants);
                            } else if self.debug >= 2 {
                                    eprintln!("   (skipping redundant match: {})", segment.text);
                            }
                        });
                    }

                    batch_matches.extend(currentorder_matches.into_iter());
                }


                /*if params.context_weight > 0.0 {
                    self.rescore_input_context(&mut batch_matches, &boundaries, params);
                }*/

                let l = matches.len();
                //consolidate the matches, finding a single segmentation that has the best (highest
                //scoring) solution
                if params.max_ngram > 1 || self.have_lm || !self.context_rules.is_empty() {
                    //(debug will be handled in the called method)
                    matches.extend(
                        self.most_likely_sequence(batch_matches, boundaries, begin, boundary.offset.begin, params, text_current).into_iter()
                    );
                } else {
                    if self.debug >= 1 {
                        eprintln!("  (returning matches directly, no need to find most likely sequence for unigrams)");
                    }
                    matches.extend(
                        batch_matches.into_iter().map(|mut m| {
                            m.selected = Some(0); //select the first (highest ranking) option
                            m
                        })
                    );
                }
                if self.debug >= 1 {
                    eprintln!("  (added sequence of {} matches)", matches.len() - l );
                }

                begin = boundary.offset.end; //(the hard boundary itself is not included in any variant/sequence matching)
                begin_index = i+1
            }

        }

        if self.debug >= 1 {
            eprintln!("(returning {} matches)", matches.len());
            if self.debug >= 2 {
                eprintln!(" (MATCHES={:?})", matches);
            }
        }
        if params.unicodeoffsets {
            if self.debug >= 1 {
                eprintln!("(remapping UTF-8 offsets to unicodepoints)");
            }
            remap_offsets_to_unicodepoints(text, matches)
        } else {
            matches
        }
    }




    /*
    fn set_match_boundaries<'a>(&self, matches: &mut Vec<Match<'a>>, boundaries: &[Match<'a>]) {
        for m in matches.iter_mut() {

            for (i, boundary) in boundaries.iter().enumerate() {
                if m.offset.begin == boundary.offset.end  {
                    m.prevboundary = Some(i)
                } else if m.offset.end == boundary.offset.begin  {
                    m.nextboundary = Some(i)
                }
            }

            m.n = if let Some(prevboundary) = m.prevboundary {
                m.nextboundary.expect("next boundary must exist") - prevboundary
            } else {
                m.nextboundary.expect("next boundary must exist") + 1
            };
        }
    }

    /// Find the unigram context from the input for all matches
    fn find_input_context<'a>(&self, matches: &Vec<Match<'a>>) -> Vec<(usize,Context<'a>)> {
        let mut results = Vec::with_capacity(matches.len());
        for (i, m) in matches.iter().enumerate() {
            let mut left = None;
            let mut right = None;
            for mcontext in matches.iter() {
                if let Some(prevboundary) = m.prevboundary {
                    if mcontext.nextboundary == Some(prevboundary) && mcontext.n == 1{
                        left = Some(mcontext.text);
                    }
                }
                if let Some(nextboundary) = m.nextboundary {
                    if mcontext.prevboundary == Some(nextboundary) && mcontext.n == 1{
                        right = Some(mcontext.text);
                    }
                }
            }
            results.push(
                (i, Context {
                    left,
                    right,
                })
            );
        }
        results
    }
    */


    /*
    /// Rescores variants by incorporating a language model component in the variant score.
    /// For simplicity, however, this component is based on the original
    /// input text rather than corrected output from other parts.
    fn rescore_input_context<'a>(&self, matches: &mut Vec<Match<'a>>, boundaries: &[Match<'a>], params: &SearchParameters) {
        if self.debug >= 2 {
            eprintln!("   (rescoring variants according to input context)");
        }
        self.set_match_boundaries(matches, boundaries);
        let matches_with_context = self.find_input_context(matches);
        assert_eq!(matches_with_context.len(), matches.len());
        let mut tokens: Vec<Option<VocabId>> = Vec::new();
        let mut perplexities: Vec<f64> = Vec::new();
        for (i, context) in matches_with_context.iter() {
            let m = matches.get(*i).expect("match must exist");

            let left = match context.left {
                Some(text) => self.encoder.get(text).map(|x| *x),
                None => Some(BOS)
            };
            let right = match context.right {
                Some(text) => self.encoder.get(text).map(|x| *x),
                None => Some(BOS)
            };

            perplexities.clear();
            let mut best_perplexity = 99999.0; //to be minimised
            if let Some(variants) = &m.variants {
                for (variant, _dist_score, _freq_score) in variants.iter() {
                    if let Ok(mut ngram) = self.into_ngram(*variant, &mut None) {
                        tokens.clear();
                        tokens.push(left);
                        loop {
                            match ngram.pop_first() {
                                NGram::Empty => break,
                                unigram => tokens.push(unigram.first())
                            }
                        }
                        tokens.push(right);

                        let (_lm_logprob, perplexity) = self.lm_score_tokens(&tokens);

                        if perplexity < best_perplexity {
                            best_perplexity = perplexity;
                        }
                        perplexities.push(perplexity);
                    }
                }
            }
            if self.debug >= 2 {
                eprintln!("    (processing {} variants for match {}, best_perplexity={})", perplexities.len(), i+1, best_perplexity);
            }

            let m = matches.get_mut(*i).expect("match must exist");
            for (j, perplexity) in perplexities.iter().enumerate() {
                let variants = &mut m.variants.as_mut().expect("variants must exist");
                let (vocab_id, score, freq_score) = variants.get_mut(j).expect("variant must exist");
                //compute a weighted geometric mean between language model score
                //and variant model score

                //first normalize the perplexity where the best one corresponds to 1.0, and values decrease towards 0 as perplexity increases, the normalisation is technically not needed for geometric mean but we do need to invert the scale (minimisation of perplexity -> maximisation of score)
                let lmscore = best_perplexity / perplexity;

                //then the actual computation is done in log-space for more numerical stability,
                //and cast back afterwards
                let oldscore = *score;
                *score = ((score.ln() + params.context_weight as f64 * lmscore.ln()) / (1.0 + params.context_weight) as f64).exp();
                //                      fixed weight for variant model ------------------^
                if self.debug >= 3 {
                    if let Some(vocabitem) = self.decoder.get(*vocab_id as usize) {
                        eprintln!("      (leftcontext={:?}, variant={}, rightcontext={:?}, oldscore={}, score={}, norm_lm_score={}, perplexity={})", context.left, vocabitem.text, context.right, oldscore, score, lmscore, perplexity);
                    }
                }
            }

        }
    }
    */


    /// Find the solution that maximizes the variant scores, decodes using a Weighted Finite State Transducer
    fn most_likely_sequence<'a>(&self, matches: Vec<Match<'a>>, boundaries: &[Match<'a>], begin_offset: usize, end_offset: usize, params: &SearchParameters, input_text: &str) -> Vec<Match<'a>> {
        if self.debug >= 2 {
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
        let states: Vec<u32> = boundaries.iter().map(|boundary| {
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

        if self.debug >= 2 {
            eprintln!(" (added {} states ({} boundaries), not including the start state)", states.len(), boundaries.len());
        }

        let mut output_symbols: Vec<OutputSymbol> = vec!(
            OutputSymbol { vocab_id: 0, symbol: 0, match_index: 0, variant_index: None, boundary_index: 0 }, //first entry is a dummy entry because the 0 symbol is reserved for epsilon
        );

        //add transitions between the boundary states
        for (match_index, m) in matches.iter().enumerate() {

            if self.debug >= 2 {
                symtab_in.add_symbol(m.text); //symbol_index = match_index + 1
            }

            let mut prevboundary: Option<usize> = None;
            let mut nextboundary: Option<usize> = None;

            let input_symbol = (match_index + 1) as u32;

            for (i, boundary) in boundaries.iter().enumerate() {
                if m.offset.begin == boundary.offset.end  {
                    prevboundary = Some(i)
                } else if m.offset.end == boundary.offset.begin  {
                    nextboundary = Some(i)
                }
            }

            let n;
            let prevstate= if let Some(prevboundary) = prevboundary {
                n = nextboundary.expect("next boundary must exist") - prevboundary;
                *states.get(prevboundary).expect("prev state must exist")
            } else {
                n = nextboundary.expect("next boundary must exist") + 1;
                start
            };
            let nextstate = *states.get(nextboundary.expect("next boundary must exist")).expect("next state must exist");

            if m.variants.is_some() && !m.variants.as_ref().unwrap().is_empty() {
                for (variant_index, variantresult) in m.variants.as_ref().unwrap().iter().enumerate() {
                    let output_symbol = output_symbols.len() as u32;
                    output_symbols.push( OutputSymbol {
                        vocab_id: variantresult.vocab_id,
                        symbol: output_symbol,
                        match_index,
                        variant_index: Some(variant_index),
                        boundary_index: nextboundary.expect("next boundary must exist")
                    });

                    if self.debug >= 3 {
                        let mut variant_text = String::new();
                        variant_text += self.decoder.get(variantresult.vocab_id as usize).expect("variant_text").text.as_str();
                        variant_text += format!(" ({})", output_symbol).as_str(); //we encode the output symbol in the text otherwise the symbol table returns the old match
                        eprintln!("   (transition state {}->{}: {} ({}) -> {} and variant score {})", prevstate, nextstate, m.text, input_symbol, variant_text, -1.0 * variantresult.score(params.freq_weight).ln() as f32);
                        let osym = symtab_out.add_symbol(variant_text);
                        assert!(osym == output_symbol);
                    }

                    //each transition gets a base cost of n (the number of input tokens it covers)
                    //on top of that cost in the range 0.0 (best) - 1.0 (worst)  expresses the
                    //distance score (inversely)
                    let cost: f32 = n as f32 + (1.0 - variantresult.score(params.freq_weight) as f32);
                    fst.add_tr(prevstate, Tr::new(input_symbol, output_symbol, cost, nextstate)).expect("adding transition");
                }
            } else if n == 1 { //only for unigrams
                let output_symbol = output_symbols.len() as u32;
                output_symbols.push( OutputSymbol {
                    vocab_id: 0, //0 vocab_id means we have an Out-of-Vocabulary word to copy from input
                    symbol: output_symbol,
                    match_index,
                    variant_index: None,
                    boundary_index: nextboundary.expect("next boundary must exist")
                });

                //OOV emission cost
                let cost: f32 = n as f32 + 1.0;

                if self.debug >= 3 {
                    eprintln!("   (transition state {}->{}: {} ({}) -> OOV ({}) and score {})", prevstate, nextstate, m.text, input_symbol, output_symbol, cost);
                    let mut variant_text = String::from_str(m.text).expect("from str");
                    variant_text += format!(" ({})", output_symbol).as_str(); //we encode the output symbol in the text otherwise the symbol table returns the old match
                    let osym = symtab_out.add_symbol(&variant_text);
                    if osym != output_symbol {
                        panic!("Output symbol out of sync: {} vs {}, variant_text={}", osym, output_symbol, variant_text);
                    }
                }

                fst.add_tr(prevstate, Tr::new(input_symbol, output_symbol, cost, nextstate)).expect("adding transition");
            }
        }

        //failsafe: add high-cost epsilon transitions between boundaries to ensure the graph always has a complete path
        for i in 0..boundaries.len() {
            let nextboundary = i;
            let prevstate = if i == 0 {
                start
            } else {
                *states.get(i-1).expect("prev state must exist")
            };
            let nextstate = *states.get(nextboundary).expect("next state must exist");
            fst.add_tr(prevstate, Tr::new(0, 0, 100.0, nextstate)).expect("adding transition");
        }


        if output_symbols.len() == 1 {
            if self.debug >= 2 {
                eprintln!("   (no output symbols found, FST not needed, aborting)");
            }
            //we have no output symbols, building an FST is not needed, just return the input
            return matches;
        }

        //find the n most likely sequences, note that we only consider the distance scores here,
        //language modelling (considering context) is applied in a separate step later

        if self.debug >= 3 {
            eprintln!(" (computed FST: {:?})", fst);
            eprintln!("   (symtab_in={:?})", symtab_in);
            eprintln!("   (symtab_out={:?})", symtab_out);
            eprintln!(" (finding shortest path)");
            fst.set_input_symbols(Arc::new(symtab_in));
            fst.set_output_symbols(Arc::new(symtab_out));
            let input_text_filename = input_text.replace(" ","_").replace("\"","").replace("'","").replace(".","").replace("/","").replace("?",""); //strip filename unfriendly chars
            let mut config = DrawingConfig::default();
            config.portrait = true;
            config.title = input_text.to_owned();
            if let Err(e) = fst.draw(format!("/tmp/analiticcl.{}.fst.dot", input_text_filename.as_str()), &config ) {
                panic!("FST draw error: {}", e);
            }
        }
        let fst: VectorFst<TropicalWeight> = shortest_path_with_config(&fst, ShortestPathConfig::default().with_nshortest(params.max_seq) ).expect("computing shortest path fst");
        let mut sequences: Vec<Sequence> = Vec::new();
        let mut best_lm_perplexity: f64 = 999999.0; //to be minimised
        let mut best_variant_cost: f32 = (boundaries.len() - 1) as f32 * 2.0; //worst score, to be improved (to be minimised)
        let mut best_context_score: f64 = 0.0; //to be maximised

        for (i, path)  in fst.paths_iter().enumerate() { //iterates over the n shortest path hypotheses (does not return them in weighted order)
            let variant_cost: f32 = *path.weight.value();
            let mut sequence = Sequence::new(variant_cost);
            if self.debug >= 3 {
                eprintln!("  (#{}, path: {:?})", i+1, path);
            }
            for output_symbol in path.olabels.iter() {
                let output_symbol = output_symbols.get(*output_symbol as usize).expect("expected valid output symbol");
                sequence.output_symbols.push(output_symbol.clone());
            }
            if self.have_lm && params.lm_weight > 0.0 {
                //Apply the language model, considers context
                let (lm_logprob, perplexity) = self.lm_score(&sequence, &boundaries);
                sequence.lm_logprob = lm_logprob;
                sequence.perplexity = perplexity;
                if sequence.perplexity < best_lm_perplexity {
                    best_lm_perplexity = sequence.perplexity;
                }
            }
            if !self.context_rules.is_empty() {
                //Apply context rules and apply tags (if any), considers context
                let (context_score, sequence_results) = self.test_context_rules(&sequence);
                sequence.context_score = context_score;
                sequence.tags = sequence_results.into_iter().map(|vecpm| {
                    vecpm.into_iter().filter_map(|pm| 
                        if pm.tag.is_some() {
                            Some((pm.tag.unwrap(), pm.seqnr))
                        } else {
                            None
                        }
                    ).collect()
                }).collect();
                if self.debug >= 3 && sequence.context_score != 1.0 {
                    eprintln!("   (context_score: {})", sequence.context_score);
                }
            }
            if variant_cost < best_variant_cost {
                best_variant_cost = variant_cost;
            }
            if sequence.context_score > best_context_score {
                best_context_score = sequence.context_score;
            }
            sequences.push(sequence);
        }

        let mut debug_ranked: Option<Vec<(Sequence, f64, f64, f64, f64)>> = if self.debug >= 1 {
            Some(Vec::new())
        } else {
            None
        };

        //Compute the normalized scores
        let mut best_score: f64 = -99999999.0; //to be maximised
        let mut best_sequence: Option<Sequence> = None;
        for sequence in sequences.into_iter() {
            //we normalize both LM and variant model scores so the best score corresponds with 1.0 (in non-logarithmic terms, 0.0 in logarithmic space). We take the natural logarithm for more numerical stability and easier computation.
            let norm_lm_score: f64 = if self.have_lm && params.lm_weight > 0.0 {
                (best_lm_perplexity / sequence.perplexity).ln()
            } else {
                0.0
            };
            let norm_variant_score: f64 = (best_variant_cost as f64 / sequence.variant_cost as f64).ln();
            let norm_context_score: f64 = (sequence.context_score / best_context_score).ln();

            //then we interpret the score as a kind of pseudo-probability and minimize the joint
            //probability (the product; addition in log-space)
            let score = if (!self.have_lm || params.lm_weight == 0.0) && (self.context_rules.is_empty() || params.contextrules_weight == 0.0) {
                //no need for full computation, take a shortcut:
                norm_variant_score
            } else {
                (params.lm_weight as f64 * norm_lm_score + params.variantmodel_weight as f64 * norm_variant_score + params.contextrules_weight as f64 * norm_context_score) / (params.lm_weight as f64 + params.variantmodel_weight as f64 + params.contextrules_weight as f64) //note: the denominator isn't really relevant for finding the best score but normalizes the output for easier interpretability (=geometric mean)
            };
            if self.debug >= 1 {
                debug_ranked.as_mut().unwrap().push( (sequence.clone(), norm_lm_score, norm_variant_score, norm_context_score, score) );
            }
            if score > best_score || best_sequence.is_none() {
                best_score = score;
                best_sequence = Some(sequence);
            }
        }

        if self.debug >= 1 {
            //debug mode: output all candidate sequences and their scores in order
            debug_ranked.as_mut().unwrap().sort_by(|a,b| b.4.partial_cmp(&a.4).unwrap_or(Ordering::Equal) ); //sort by score
            for (i, (sequence, norm_lm_score, norm_variant_score, norm_context_score, score)) in debug_ranked.unwrap().into_iter().enumerate() {
                eprintln!("  (#{}, final_score={}, norm_lm_score={} (perplexity={}, logprob={}, weight={}), norm_variant_score={} (variant_cost={}, weight={}), norm_context_score={} (context_score={}, weight={})", i+1, score.exp(), norm_lm_score.exp(), sequence.perplexity, sequence.lm_logprob, params.lm_weight,  norm_variant_score.exp(), sequence.variant_cost, params.variantmodel_weight, norm_context_score.exp(), sequence.context_score, params.contextrules_weight);
                let mut inputtext: String = String::new();
                let mut text: String = String::new();
                for (j, output_symbol) in sequence.output_symbols.iter().enumerate() {
                    let m = matches.get(output_symbol.match_index).expect("match index must exist");
                    inputtext += m.text;
                    if output_symbol.vocab_id > 0{
                        text += self.decoder.get(output_symbol.vocab_id as usize).expect("vocab").text.as_str();
                    } else {
                        text += m.text;
                    }
                    if !sequence.tags.is_empty() {
                        if let Some(tags) = sequence.tags.get(j) {
                            for (tagindex, seqnr) in tags.iter() {
                                text += format!("[#{}:{}]", self.tags.get(*tagindex as usize).expect("Tag must exist"), seqnr).as_str();
                            }
                        }
                    }
                    inputtext += " | ";
                    text += " | ";
                }
                eprintln!("    (text_out={})", text);
                eprintln!("    (text_in={})", inputtext);
            }
        }

        //return matches corresponding to best sequence
        let best_sequence = best_sequence.expect("there must be a best sequence");
        best_sequence.output_symbols.iter().enumerate().map(|(i,osym)| {
            let m = matches.get(osym.match_index).expect("match should be in bounds");
            let mut m = m.clone();
            m.selected = osym.variant_index;
            if !best_sequence.tags.is_empty() {
                if let Some(tags) = best_sequence.tags.get(i) {
                    m.tag = tags.iter().map(|x| x.0).collect();
                    m.seqnr = tags.iter().map(|x| x.1).collect();
                }
            }
            m
        }).collect()
    }

    /// Favours or penalizes certain combinations of lexicon matches. matching words X and Y
    /// respectively with lexicons A and B might be favoured over other combinations.
    /// This returns either a bonus or penalty (number slightly above/below 1.0) score/
    /// for the sequence as a whole.
    pub fn test_context_rules<'a>(&self, sequence: &Sequence) -> (f64, Vec<Vec<PatternMatchResult>>) {
        let sequence: Vec<(VocabId, u32)> = sequence.output_symbols.iter().map(|output_symbol|
            if output_symbol.vocab_id == 0 {
                (output_symbol.vocab_id, 0)
            } else {
                if let Some(vocabvalue) = self.decoder.get(output_symbol.vocab_id as usize) {
                    (output_symbol.vocab_id, vocabvalue.lexindex)
                } else {
                    (output_symbol.vocab_id, 0)
                }
            }).collect();

        //The sequence will flag which items in the sequence have been covered by matches (non-empty vec)
        //and if so, what context rule scores and tags (inner vec) apply to that match. It's later used to compute
        //the final score
        let mut sequence_results: Vec<Vec<PatternMatchResult>> = vec![vec!(); sequence.len()];

        let mut found = false;
        for begin in 0..sequence.len() {
            for context_rule in self.context_rules.iter() {
                if context_rule.matches(&sequence, begin, &mut sequence_results) {
                    found = true;
                    if self.debug >= 2 {
                        let text: Vec<&str> = sequence.iter().map(|(vocab_id,_)| {
                            if *vocab_id == 0 {
                                "<UNK>"
                            } else {
                                if let Some(vocabvalue) = self.decoder.get(*vocab_id as usize) {
                                    vocabvalue.text.as_ref()
                                } else {
                                    "<UNK>"
                                }
                            }}).collect();
                        eprintln!("    Context rule matches: {:?} <-- \"{}\" --> {:?}", context_rule, text.join(" | "), sequence_results);
                    }
                }
            }
        }

        if !found {
            (1.0, sequence_results) //just a shortcut to prevent unnecessary computation
        } else {
            (
                //compute sum score
                sequence_results.iter().map(|x| if !x.is_empty() {
                    x[0].score //score is equal for all subelements (only tags differ), just grab first one
                } else {
                    1.0
                }).sum::<f32>() as f64 / sequence.len() as f64
                ,
                sequence_results
            )
        }
    }

    /// Computes the logprob and perplexity for a given sequence as produced in
    /// most_likely_sequence()
    pub fn lm_score<'a>(&self, sequence: &Sequence, boundaries: &[Match<'a>]) -> (f32,f64) {

        //step 1: collect all tokens in the sequence

        let mut tokens: Vec<Option<VocabId>> = Vec::with_capacity(sequence.output_symbols.len() + 5); //little bit of extra space to prevent needing to reallocate too quickly and to hold the BOS/EOS markers
        tokens.push(Some(BOS));


        for output_symbol in sequence.output_symbols.iter() {
            let next_boundary = boundaries.get(output_symbol.boundary_index).expect("boundary should be in bounds");

            if output_symbol.vocab_id == 0  {
                //out of vocabulary (copied from input)
                tokens.push(None);
            } else {
                if let Ok(mut ngram) = self.into_ngram(output_symbol.vocab_id, &mut None) {
                    loop {
                        match ngram.pop_first() {
                            NGram::Empty => break,
                            unigram => tokens.push(unigram.first())
                        }
                    }
                }
            }

            //add boundary as a token too
            if !next_boundary.text.trim().is_empty() {
                if let Some(vocab_id) = self.encoder.get(next_boundary.text.trim()) {
                    if let Ok(mut ngram) = self.into_ngram(*vocab_id, &mut None) {
                        loop {
                            match ngram.pop_first() {
                                NGram::Empty => break,
                                unigram => tokens.push(unigram.first())
                            }
                        }
                    }
                } else {
                    //out of vocabulary boundary tokens (copied from input)
                    tokens.push(None);
                }
            }

        }

        tokens.push(Some(EOS));

        //Compute the score over the tokens
        self.lm_score_tokens(&tokens)
    }


    /// Computes the logprob and perplexity for a given sequence of tokens.
    /// The tokens are either in the vocabulary or are None if out-of-vocabulary.
    pub fn lm_score_tokens<'a>(&self, tokens: &Vec<Option<VocabId>>) -> (f32,f64) {
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
                //if we have an out of vocabulary bigram or prior we fall back to add-one smoothing
                //simply setting the count of that ngram/prior to 1
                //for the perplexity computation this means the score doesn't change, but n does
                //increase, so we end up with a lower perplexity
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
    fn into_ngram(&self, word: VocabId, unseen_parts: &mut Option<VocabEncoder>) -> Result<NGram,Box<dyn Error>> {
        let word_dec = self.decoder.get(word as usize).expect("word does not exist in decoder");
        let mut iter = word_dec.text.split(" ");
        match word_dec.tokencount {
            0 => Ok(NGram::Empty),
            1 => Ok(NGram::UniGram(
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts)
            )),
            2 => Ok(NGram::BiGram(
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts)
            )),
            3 => Ok(NGram::TriGram(
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts)
            )),
            4 => Ok(NGram::QuadGram(
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts)
            )),
            5 => Ok(NGram::QuintGram(
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts),
                self.encode_token(iter.next().expect("ngram part"), true, unseen_parts)
            )),
            _ => simple_error::bail!("Can only deal with n-grams up to order 5")
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
        if let Some(vocabvalue) = self.match_to_vocabvalue(m) {
            vocabvalue.text.as_str()
        } else {
            m.text
        }
    }

    /// Gives the vocabitem for this match, always uses the solution (if any) and falls
    /// back to the input text only when no solution was found.
    pub fn match_to_vocabvalue<'a>(&'a self, m: &Match<'a>) -> Option<&'a VocabValue> {
        if let Some(result) = m.solution() {
            self.decoder.get(result.vocab_id as usize)
        } else {
            None
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

