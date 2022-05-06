use crate::types::*;

pub const ALPHABET: &[&[&str]] = &[
   &["a","A"],
   &["b","B"],
   &["c","C"],
   &["d","D"],
   &["e","E"],
   &["f","F"],
   &["g","G"],
   &["h","H"],
   &["i","I"],
   &["j","J"],
   &["k","K"],
   &["l","L"],
   &["m","M"],
   &["n","N"],
   &["o","O"],
   &["p","P"],
   &["q","Q"],
   &["r","R"],
   &["s","S"],
   &["t","T"],
   &["u","U"],
   &["v","V"],
   &["w","W"],
   &["x","X"],
   &["y","Y"],
   &["z","Z"],
   &[".",","],
];


pub fn get_test_alphabet() -> (Alphabet,CharIndexType) {
    //this is a bit silly to do so verbosely here just to get
    //everything in Vecss and Strings, but it works
    let mut alphabet: Alphabet = Vec::new();
    for chars in ALPHABET {
        let mut ownedchars: Vec<String> = Vec::new();
        for c in *chars {
            ownedchars.push(c.to_string());
        }
        alphabet.push(ownedchars);
    }
    let l = alphabet.len();
    (alphabet, l as CharIndexType)
}

pub fn get_test_searchparams() -> SearchParameters {
    SearchParameters {
        max_edit_distance: DistanceThreshold::Absolute(2),
        max_anagram_distance: DistanceThreshold::Absolute(2),
        max_matches: 10,
        stop_criterion: StopCriterion::Exhaustive,
        score_threshold: 0.0,
        cutoff_threshold: 0.0,
        max_ngram: 2,
        lm_order: 2,
        freq_weight: 0.0,
        single_thread: true,
        context_weight: 0.0,
        lm_weight: 1.0,
        variantmodel_weight: 3.0,
        contextrules_weight: 1.0,
        max_seq: 250,
        consolidate_matches: true,
        unicodeoffsets: false,
    }
}
