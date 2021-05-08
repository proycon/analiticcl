//#[macro_use]
//extern crate matches;

extern crate sesdiff;

use std::str::FromStr;
use analiticcl::*;
use analiticcl::test::*;


#[test]
fn test0001_alphabet() {
    let (alphabet, alphabet_size) = get_test_alphabet();
    assert_eq!(alphabet.len(), 27);
}

#[test]
fn test0301_normalize_to_alphabet() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    assert_eq!(&"a".normalize_to_alphabet(&alphabet), &[0]);
    assert_eq!(&"b".normalize_to_alphabet(&alphabet), &[1]);
}

#[test]
fn test0302_levenshtein() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    assert_eq!(levenshtein(&"a".normalize_to_alphabet(&alphabet), &"a".normalize_to_alphabet(&alphabet),99), Some(0));
    assert_eq!(levenshtein(&"a".normalize_to_alphabet(&alphabet), &"b".normalize_to_alphabet(&alphabet),99), Some(1));
    //substitution
    assert_eq!(levenshtein(&"ab".normalize_to_alphabet(&alphabet), &"ac".normalize_to_alphabet(&alphabet),99), Some(1));
    //insertion
    assert_eq!(levenshtein(&"a".normalize_to_alphabet(&alphabet), &"ab".normalize_to_alphabet(&alphabet),99), Some(1));
    //deletion
    assert_eq!(levenshtein(&"ab".normalize_to_alphabet(&alphabet), &"a".normalize_to_alphabet(&alphabet),99), Some(1));
    //transposition
    assert_eq!(levenshtein(&"ab".normalize_to_alphabet(&alphabet), &"ba".normalize_to_alphabet(&alphabet),99), Some(2));

    assert_eq!(levenshtein(&"abc".normalize_to_alphabet(&alphabet), &"xyz".normalize_to_alphabet(&alphabet),99), Some(3));
}

#[test]
fn test0303_damereau_levenshtein() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    assert_eq!(damerau_levenshtein(&"a".normalize_to_alphabet(&alphabet), &"a".normalize_to_alphabet(&alphabet),99), Some(0));
    assert_eq!(damerau_levenshtein(&"a".normalize_to_alphabet(&alphabet), &"b".normalize_to_alphabet(&alphabet),99), Some(1));
    //substitution
    assert_eq!(damerau_levenshtein(&"ab".normalize_to_alphabet(&alphabet), &"ac".normalize_to_alphabet(&alphabet),99), Some(1));
    //insertion
    assert_eq!(damerau_levenshtein(&"a".normalize_to_alphabet(&alphabet), &"ab".normalize_to_alphabet(&alphabet),99), Some(1));
    //deletion
    assert_eq!(damerau_levenshtein(&"ab".normalize_to_alphabet(&alphabet), &"a".normalize_to_alphabet(&alphabet),99), Some(1));
    //transposition (this is where the difference with normal levenshtein is)
    assert_eq!(damerau_levenshtein(&"ab".normalize_to_alphabet(&alphabet), &"ba".normalize_to_alphabet(&alphabet),99), Some(1));

    assert_eq!(damerau_levenshtein(&"abc".normalize_to_alphabet(&alphabet), &"xyz".normalize_to_alphabet(&alphabet),99), Some(3));
}

#[test]
fn test0304_lcslen() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    assert_eq!(longest_common_substring_length(&"test".normalize_to_alphabet(&alphabet), &"testable".normalize_to_alphabet(&alphabet)), 4);
    assert_eq!(longest_common_substring_length(&"fasttest".normalize_to_alphabet(&alphabet), &"testable".normalize_to_alphabet(&alphabet)), 4);
    assert_eq!(longest_common_substring_length(&"abcdefhij".normalize_to_alphabet(&alphabet), &"def".normalize_to_alphabet(&alphabet)), 3);
    assert_eq!(longest_common_substring_length(&"def".normalize_to_alphabet(&alphabet), &"abcdefhij".normalize_to_alphabet(&alphabet)), 3);
}

#[test]
fn test0304_prefixlen() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    assert_eq!(common_prefix_length(&"test".normalize_to_alphabet(&alphabet), &"testable".normalize_to_alphabet(&alphabet)), 4);
    assert_eq!(common_prefix_length(&"testable".normalize_to_alphabet(&alphabet), &"test".normalize_to_alphabet(&alphabet)), 4);
    assert_eq!(common_prefix_length(&"fasttest".normalize_to_alphabet(&alphabet), &"testable".normalize_to_alphabet(&alphabet)), 0);
    assert_eq!(common_prefix_length(&"fasttest".normalize_to_alphabet(&alphabet), &"test".normalize_to_alphabet(&alphabet)), 0);
}

#[test]
fn test0304_suffixlen() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    assert_eq!(common_suffix_length(&"test".normalize_to_alphabet(&alphabet), &"testable".normalize_to_alphabet(&alphabet)), 0);
    assert_eq!(common_suffix_length(&"testable".normalize_to_alphabet(&alphabet), &"test".normalize_to_alphabet(&alphabet)), 0);
    assert_eq!(common_suffix_length(&"fasttest".normalize_to_alphabet(&alphabet), &"testable".normalize_to_alphabet(&alphabet)), 0);
    assert_eq!(common_suffix_length(&"fasttest".normalize_to_alphabet(&alphabet), &"test".normalize_to_alphabet(&alphabet)), 4);
}


#[test]
fn test0400_model_load() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let model = VariantModel::new_with_alphabet(alphabet, Weights::default(), true);
}

#[test]
fn test0401_model_build() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), true);
    let lexicon: &[&str] = &["rites","tiers", "tires","tries","tyres","rides","brides","dire"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,None, 0);
    }
    model.build();
    assert!(model.has(&"rites"));
    for text in lexicon.iter() {
        assert!(model.has(text));
        assert!(model.get(text).is_some());
    }
    assert!(!model.has(&"unknown"));
    assert!(model.get(&"unknown").is_none());
}

#[test]
fn test0404_score_test() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), true);
    let lexicon: &[&str] = &["huis","huls"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,None, 0);
    }
    model.build();
    let results = model.find_variants("huys", 2, 2, 10, 0.0, StopCriterion::Exhaustive, None);
    //results are a bit indeterministic due to sort_unstable
    //(order of equal-scoring elements is not fixed)
    //we just check if we get two results with the same score
    assert_eq!( results.len(), 2);
    assert_ne!( results.get(0).unwrap().0, results.get(1).unwrap().0 );
    assert_eq!( results.get(0).unwrap().1, results.get(1).unwrap().1 );
}


#[test]
fn test0501_confusable_found_in() {
    let confusable =  Confusable::new("-[y]+[i]",1.1).expect("valid script");
    eprintln!("confusable: {:?}", confusable);
    let huis_script = sesdiff::shortest_edit_script("huys","huis", false, false, false);
    eprintln!("huis_script: {:?}", huis_script);
    let huls_script = sesdiff::shortest_edit_script("huys","huls", false, false, false);
    eprintln!("huls_script: {:?}", huls_script);
    assert!(confusable.found_in(&huis_script), "confusable should be found in huys->huis");
    assert!(!confusable.found_in(&huls_script),"confusable should not be found in huys->huls");
}

#[test]
fn test0502_confusable_test() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), true);
    let lexicon: &[&str] = &["huis","huls"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,None, 0);
    }
    model.add_to_confusables("-[y]+[i]",1.1).expect("added to confusables");
    model.build();
    let results = model.find_variants("huys", 2, 2, 10, 0.0, StopCriterion::Exhaustive, None);
    assert_eq!( model.decoder.get(results.get(0).unwrap().0 as usize).unwrap().text, "huis");
    assert_eq!( model.decoder.get(results.get(1).unwrap().0 as usize).unwrap().text, "huls");
    assert!( results.get(0).unwrap().1 > results.get(1).unwrap().1, "score of huis should be greater than that of huls" );
}

#[test]
fn test0503_confusable_test2() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), true);
    let lexicon: &[&str] = &["huis","huls"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,None, 0);
    }
    model.add_to_confusables("-[y]+[i]",1.1).expect("added to confusables");
    model.build();
    let results = model.find_variants("Huys", 2, 2, 10, 0.0, StopCriterion::Exhaustive, None);
    assert_eq!( model.decoder.get(results.get(0).unwrap().0 as usize).unwrap().text, "huis");
    assert_eq!( model.decoder.get(results.get(1).unwrap().0 as usize).unwrap().text, "huls");
    assert!( results.get(0).unwrap().1 > results.get(1).unwrap().1, "score of huis should be greater than that of huls" );
}

#[test]
fn test0504_confusable_nomatch() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), false);
    let lexicon: &[&str] = &["huis","huls"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,None, 0);
    }
    model.add_to_confusables("-[y]+[p]",1.1).expect("added to confusables");
    model.build();
    let results = model.find_variants("Huys", 2, 2, 10, 0.0, StopCriterion::Exhaustive, None);
    assert_eq!( results.len() , 2 );
    assert_eq!( results.get(0).unwrap().1,results.get(1).unwrap().1, "score of huis should be equal to that of huls" );
}

#[test]
fn test0601_find_boundaries() {
    let text = "Hallo allemaal, ik zeg: \"Welkom in Aix-les-bains!\".";
    let boundaries = find_boundaries(&text);
    eprintln!("{:?}", boundaries);
    assert_eq!( boundaries.len() , 9 );
    assert_eq!( boundaries.get(0).unwrap().offset.begin , 5 );
    assert_eq!( boundaries.get(0).unwrap().offset.end , 6 );
    assert_eq!( boundaries.get(0).unwrap().text , " " );
    assert_eq!( boundaries.get(1).unwrap().text , ", " );
    assert_eq!( boundaries.get(2).unwrap().text , " " );
    assert_eq!( boundaries.get(3).unwrap().text , ": \"" );
    assert_eq!( boundaries.get(4).unwrap().text , " " );
    assert_eq!( boundaries.get(5).unwrap().text , " " );
    assert_eq!( boundaries.get(6).unwrap().text , "-" );
    assert_eq!( boundaries.get(7).unwrap().text , "-" );
    assert_eq!( boundaries.get(8).unwrap().text , "!\"." );
}

#[test]
fn test0602_find_ngrams() {
    let text = "dit is een mooie test";
    let boundaries = find_boundaries(&text);
    let ngrams = find_ngrams(text, &boundaries, 1, 0);
    assert_eq!( ngrams.len() , 5 );
    assert_eq!( ngrams.get(0).unwrap().0.text , "dit" );
    assert_eq!( ngrams.get(1).unwrap().0.text , "is" );
    assert_eq!( ngrams.get(2).unwrap().0.text , "een" );
    assert_eq!( ngrams.get(3).unwrap().0.text , "mooie" );
    assert_eq!( ngrams.get(4).unwrap().0.text , "test" );
}

#[test]
fn test0603_find_ngrams2() {
    let text = "dit is een mooie test.";
    let boundaries = find_boundaries(&text);
    let ngrams = find_ngrams(text, &boundaries, 1, 0);
    assert_eq!( ngrams.len() , 5 );
    assert_eq!( ngrams.get(0).unwrap().0.text , "dit" );
    assert_eq!( ngrams.get(1).unwrap().0.text , "is" );
    assert_eq!( ngrams.get(2).unwrap().0.text , "een" );
    assert_eq!( ngrams.get(3).unwrap().0.text , "mooie" );
    assert_eq!( ngrams.get(4).unwrap().0.text , "test" );
}
