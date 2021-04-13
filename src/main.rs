use std::collections::HashMap;
use std::fs::File;
use std::io::{Write,Read,BufReader,BufRead,Error};

///Each type gets assigned an ID integer, carries no further meaning
type VocabId = u64;

///A normalized string encoded via the alphabet
type NormString = Vec<u8>;

#[derive(Clone)]
struct VocabValue {
    text: String,
    norm: NormString,
    frequency: Option<u32>,
    ///The number of words
    tokencount: u8,
}


///Map integers (indices correspond to VocabId) to string values (and optionally a frequency count)
type VocabDecoder = Vec<VocabValue>;

///Maps strings to integers
type VocabEncoder = HashMap<String, VocabId>;

///The anagram hash: uses a bag-of-characters representation where each bit flags the presence/absence of a certain character (the order of the bits are defined by Alphabet)
type Anahash = u64;

///Defines the alphabet, index corresponds how things are encoded, multiple strings may be encoded
///in the same way
type Alphabet = Vec<Vec<String>>;

/// Map from anahashes to vocabulary IDs
type AnahashTable = HashMap<Anahash,Vec<VocabId>>;

/// Map from anahashes to anahashes (one to many)
type AnahashMap = HashMap<Anahash,Vec<Anahash>>;


struct VariantModel {
    decoder: VocabDecoder,
    encoder: VocabEncoder,

    alphabet: Alphabet,

    ///Maps an anahash to all existing instances that instantiate it
    instances: AnahashTable,

    ///Maps an anahash to all anahashes that delete a character
    deletions: AnahashMap,

    ///Maps an anahash to all anahashes that add a character
    insertions: AnahashMap,
}


///Trait for objects that can be anahashed (string-like)
trait Anahashable {
    fn anahash(&self, alphabet: &Alphabet) -> Anahash;
    fn normalize_to_alphabet(&self, alphabet: &Alphabet) -> NormString;
}

impl Anahashable for str {
    ///Compute the anahash for a given string, according to the alphabet
    fn anahash(&self, alphabet: &Alphabet) -> Anahash {
        let mut hash: Anahash = 0;
        for (pos, c) in self.char_indices() {
            let mask = 1 << pos;
            for chars in alphabet.iter() {
                for element in chars.iter() {
                    let l = element.chars().count();
                    if let Some(slice) = self.get(pos..pos+l) {
                        if slice == element {
                            hash = hash | mask;
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
        let result = Vec::with_capacity(self.chars().count());
        for (pos, c) in self.char_indices() {
            let mask = 1 << pos;
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

enum AnahashEdit {
    Insertion(String),
    Deletion(String),
}

enum AnahashExpandMode {
    ///Expand to all anahashes, whether they occur as instances or not
    All,
    ///Expand only to anahashes that exist in the instances
    MatchOnly,
    ///Expand only to anahashes that do not exist in the instances
    NoMatchOnly,
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
    fn default() -> VocabParams {
        VocabParams {
            text_column: 0,
            freq_column: None,
        }
    }
}


///Merges a sorted source vector into a sorted target vector, ignoring duplicates
fn merge_into<T: std::cmp::Ord>(target: &mut Vec<T>, source: &[T]) {
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
        target.insert(pos, *elem);
    }
}



///Computes the difference between two anahashes,
///i.e. the number of transitions needed to go from hash1
///to hash2
fn anahash_diff(hash1: Anahash, hash2: Anahash) -> u8 {
    //TODO: implement
}



///Compute levenshtein distance between two normalised strings
///Returns None if the maximum distance is exceeded
fn levenshtein(a: &[u8], b: &[u8], max_distance: u8) -> Option<u8>
    //TODO: implement
}

impl VariantModel {
    fn new(alphabet_file: &str, vocabulary_file: &str, vocabparams: Option<VocabParams>) -> VariantModel {
        let mut model = VariantModel {
            alphabet: read_alphabet(alphabet_file).expect("Error loading alphabet file"),
            encoder: HashMap::new(),
            decoder: Vec::new(),
            instances: HashMap::new(),
            deletions: HashMap::new(),
            insertions: HashMap::new(),
        };
        model.read_vocabulary(vocabulary_file, &vocabparams).expect("Error loading vocabulary file");
        model
    }

    fn train(&mut self) {
        eprintln!("Computing anahash instance table...");
        for (s, id)  in self.encoder.iter() {
            //get the anahash
            let anahash = s.anahash(&self.alphabet);

            //add it to the instances
            if let Some(idlist) = self.instances.get_mut(&anahash) {
                idlist.push(*id);
            } else {
                self.instances.insert(anahash, vec!(*id));
            }
        }

        eprintln!("Computing anahash search space...");

        //Compute deletions for all instances, expanding
        //recursively also to anahashes which do not have instances
        //so we have complete route for all anahashes
        for anahash in self.instances.keys() {
            self.expand_deletions(&[*anahash]);
        }

        eprintln!("Computing insertions...");

        //Insertions are simply the reverse of deletions
        for (anahash, parents) in self.deletions.iter() {
            for parent in parents.iter() {
                if let Some(newinsertions) = self.insertions.get_mut(&parent) {
                    if !newinsertions.contains(&anahash) {
                        newinsertions.push(*anahash); //we will sort later
                    }
                } else {
                    self.insertions.insert(*anahash, vec!(*parent));
                }
            }
        }

        eprintln!("Sorting insertions...");

        //Sort the insertions in a separate step
        for (_, children) in self.insertions.iter_mut() {
            children.sort();
        }
    }

    ///Compute all possible deletions for this anahash, where only one deletion is made at a time
    fn compute_deletions(&self, anahash: Anahash, expandmode: AnahashExpandMode) -> Vec<Anahash> {
        let mut deletions = Vec::new();
        for i in 0..self.alphabet.len() {
            let mask = 1 << i;
            if anahash | mask == anahash {
                let candidate = anahash ^ mask;
                match expandmode {
                    AnahashExpandMode::All => deletions.push(candidate),
                    AnahashExpandMode::MatchOnly => if self.has_instances(candidate) { deletions.push(candidate) },
                    AnahashExpandMode::NoMatchOnly => if !self.has_instances(candidate) { deletions.push(candidate) },
                };
            }
        }
        deletions.sort_unstable(); //unstable does not preserve the order of equal elements, but is faster
        deletions
    }


    ///Computes all deletions recursively
    fn expand_deletions(&mut self, hashes: &[Anahash]) {
        for anahash in hashes.iter() {
            if !self.deletions.contains_key(anahash) {
                let parents = self.compute_deletions(*anahash, AnahashExpandMode::All);
                self.deletions.insert(*anahash, parents);
                if let Some(parents) = self.deletions.get(&anahash) {
                    self.expand_deletions(&parents);
                }
            }
        }
    }

    ///Find all insertions within a certain distance
    fn expand_insertions(&self, target: &mut Vec<Anahash>, query: Anahash, hashes: &[Anahash], max_distance: u8) {
        merge_into::<Anahash>(&mut target, hashes);
        for anahash in hashes {
            if let Some(children) = self.insertions.get(anahash) {
                self.expand_insertions(&mut target,
                                       query,
                                       &children.iter().map(|x| *x).filter(|x| anahash_diff(query,*x) <= max_distance).collect::<Vec<Anahash>>(),
                                       max_distance);
            }
        }
    }


    fn contains_anahash(&self, anahash: Anahash) -> bool {
        self.has_instances(anahash) || self.deletions.contains_key(&anahash)
    }

    fn has_instances(&self, anahash: Anahash) -> bool {
        self.instances.contains_key(&anahash)
    }

    fn contains(&self, s: &str) -> bool {
        self.encoder.contains_key(s)
    }



    ///Read vocabulary from a TSV file
    ///The parameters define what value can be read from what column
    fn read_vocabulary(&mut self, filename: &str, params: &Option<VocabParams>) -> Result<(), std::io::Error> {
        let params = params.unwrap_or_default();
        let f = File::open(filename)?;
        let f_buffer = BufReader::new(f);
        for line in f_buffer.lines() {
            if let Ok(line) = line {
                if !line.is_empty() {
                    let fields: Vec<&str> = line.split("\t").collect();
                    let text = fields.get(params.text_column as usize).expect("Expected text column not found");
                    let frequency = if let Some(freq_column) = params.freq_column {
                        Some(fields.get(freq_column as usize).expect("Expected frequency column not found").parse::<u32>().expect("frequency should be a valid integer"))
                    } else {
                        None
                    };
                    self.encoder.insert(text.to_string(), self.decoder.len() as u64);
                    self.decoder.push(VocabValue {
                        text: text.to_string(),
                        norm: text.normalize_to_alphabet(&self.alphabet),
                        frequency: frequency,
                        tokencount: text.chars().filter(|c| c == ' ').count() + 1
                    });
                }
            }
        }
        Ok(())
    }

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
        let variants: Vec<VocabId> = self.gather_instances(&expanded_anahashes, &normstring, max_edit_distance);
    }

    /// Gather instances and their edit distances, given a search string (normalised) and anagram hashes
    fn gather_instances(&self, hashes: &[Anahash], query: &[u8], max_edit_distance: u8) -> Vec<(VocabId,u8)> {
        let mut found_instances = Vec::new();
        for anahash in hashes {
            if let Some(instances) = self.instances.get(anahash) {
                for vocab_id in instances {
                    if let Some(vocabitem) = self.decoder.get(*vocab_id as usize) {
                        if let Some(distance) = levenshtein(query, &vocabitem.norm, max_edit_distance) {
                            found_instances.push((*vocab_id,distance));
                        }
                    }
                }
            }
        }
        found_instances.sort_unstable_by_key(|k| k.1 ); //sort by distance
        found_instances
    }

    /// Find the nearest anahashes that exist in the model
    fn find_nearest_anahashes(&self, anahash: &Anahash, max_distance: u8) -> Vec<Anahash> {
        if self.contains_anahash(*anahash) {
            //the easiest case, this anahash exists in the model
            vec!(*anahash)
        } else if max_distance > 0 {
            let mut results = Vec::new();
            let parents: Vec<Anahash> = self.compute_deletions(*anahash, AnahashExpandMode::All);
            for anahash in parents {
                merge_into::<Anahash>(&mut results, &self.find_nearest_anahashes(&anahash, max_distance - 1) )
            }
            results
        } else {
            vec!()
        }
    }


}

fn main() {
    println!("Hello, world!");
}
