//#[macro_use]
//extern crate matches;

extern crate sesdiff;

use analiticcl::*;
use analiticcl::test::*;

const LEXICON_AMPHIBIANS: &str = "bindings/python/tests/amphibians.tsv";
const LEXICON_REPTILES: &str = "bindings/python/tests/reptiles.tsv";

#[test]
fn test0001_alphabet() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    assert_eq!(alphabet.len(), 27);
}

#[test]
fn test0002_primes() {
    //tests whether the primes are really prime
    //(since they're hard coded and we don't want accidental typos)
    for prime in PRIMES {
        for i in 2..*prime {
            assert!(*prime % i != 0);
        }
    }
}

#[test]
fn test0102_hash_hash() {
    let (_alphabet, _alphabet_size) = get_test_alphabet();

    //this is a hash that would overflow any normal 64-bit int, but it should hash fine
    assert_eq!(AnaValue::empty(), AnaValue::from(1 as usize));
}

#[test]
fn test0103_hash_basic() {
    let (alphabet, _alphabet_size) = get_test_alphabet();

    assert_eq!("a".anahash(&alphabet), AnaValue::from(2 as usize));
    assert_eq!("b".anahash(&alphabet), AnaValue::from(3 as usize));
    assert_eq!("c".anahash(&alphabet), AnaValue::from(5 as usize));
    assert_eq!("ab".anahash(&alphabet), AnaValue::from((2*3) as usize));
    assert_eq!("ba".anahash(&alphabet), AnaValue::from((3*2) as usize));
    assert_eq!("ab".anahash(&alphabet), "ba".anahash(&alphabet));
    assert_eq!("abc".anahash(&alphabet), AnaValue::from((2*3*5) as usize));
    assert_eq!("abcabcabc".anahash(&alphabet), AnaValue::from((2*3*5*2*3*5*2*3*5) as usize));
}

#[test]
fn test0103_hash_alphabet_equivalence() {
    let (alphabet, _alphabet_size) = get_test_alphabet();

    //the alphabet may define multiple values that map to the same
    //the provided example alphabet does so for case-differences
    //and periods and commas:

    assert_eq!("abc".anahash(&alphabet), "ABC".anahash(&alphabet));
    assert_eq!("abc".anahash(&alphabet), "bAc".anahash(&alphabet));
    assert_eq!("a.b".anahash(&alphabet), "a,b".anahash(&alphabet));
}

#[test]
fn test0104_hash_big() {
    let (alphabet, _alphabet_size) = get_test_alphabet();

    //this is a hash that would overflow any normal 64-bit int, but it should hash fine
    assert!("xyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyz".anahash(&alphabet) > AnaValue::empty());
}


#[test]
fn test0105_hash_anagram() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    assert_eq!("stressed".anahash(&alphabet),"desserts".anahash(&alphabet) );
    assert_eq!("dormitory".anahash(&alphabet),"dirtyroom".anahash(&alphabet) );
    assert_eq!("presents".anahash(&alphabet),"serpents".anahash(&alphabet) );
}

#[test]
fn test0106_hash_insertion() {
    let (alphabet, _alphabet_size) = get_test_alphabet();

    let ab = "ab".anahash(&alphabet);
    let c = "c".anahash(&alphabet);
    let abc = "abc".anahash(&alphabet);

    assert_eq!(ab.insert(&c), abc);
    assert_eq!(c.insert(&ab), abc);
}

#[test]
fn test0107_hash_containment() {
    let (alphabet, _alphabet_size) = get_test_alphabet();

    let ab = "ab".anahash(&alphabet);
    let c = "c".anahash(&alphabet);
    let abc = "abc".anahash(&alphabet);

    assert_eq!(abc.contains(&c), true);
    assert_eq!(abc.contains(&ab), true);
    assert_eq!(abc.contains(&abc), true);

    //counter-examples that should evaluate to false:
    assert_eq!(c.contains(&abc), false);
    assert_eq!(ab.contains(&c), false);
    assert_eq!(ab.contains(&abc), false);
}

#[test]
fn test0108_hash_deletion() {
    let (alphabet, _alphabet_size) = get_test_alphabet();

    let ab = "ab".anahash(&alphabet);
    let b = "b".anahash(&alphabet);
    let c = "c".anahash(&alphabet);
    let abc = "abc".anahash(&alphabet);
    let ac = "ac".anahash(&alphabet);
    let x = "x".anahash(&alphabet);

    assert_eq!(abc.delete(&c), Some(ab));
    assert_eq!(abc.delete(&b), Some(ac));

    //counter-examples that should return None
    assert_eq!(c.delete(&abc), None);
    assert_eq!(abc.delete(&x), None);
}


#[test]
fn test0108_hash_upper_bound() {
    let (alphabet, alphabet_size) = get_test_alphabet();

    let ab = "ab".anahash(&alphabet);
    let abc = "abc".anahash(&alphabet);
    let x = "x".anahash(&alphabet);

    assert_eq!(abc.alphabet_upper_bound(alphabet_size), (2,3)); //indices 0,1,2 -> a,b,c   3 -> 3 characters
    assert_eq!(ab.alphabet_upper_bound(alphabet_size), (1,2));
    assert_eq!(x.alphabet_upper_bound(alphabet_size), (23,1));
}



#[test]
fn test0201_iterator_parents() {
    let (alphabet, alphabet_size) = get_test_alphabet();
    let anavalue: AnaValue = "house".anahash(&alphabet);
    let mut chars: Vec<AnaValue> = Vec::new();
    let mut deletions: Vec<AnaValue> = Vec::new();
    for deletion in anavalue.iter_parents(alphabet_size) {
       chars.push(AnaValue::character(deletion.charindex));
       deletions.push(deletion.value.clone());
    }
    assert_eq!(chars.len(), 5, "Checking length of results",);
    assert_eq!(chars.get(0).unwrap(), &"u".anahash(&alphabet));
    assert_eq!(chars.get(1).unwrap(), &"s".anahash(&alphabet));
    assert_eq!(chars.get(2).unwrap(), &"o".anahash(&alphabet));
    assert_eq!(chars.get(3).unwrap(), &"h".anahash(&alphabet));
    assert_eq!(chars.get(4).unwrap(), &"e".anahash(&alphabet));
    assert_eq!(deletions.get(0).unwrap(), &"hose".anahash(&alphabet));
    assert_eq!(deletions.get(1).unwrap(), &"houe".anahash(&alphabet));
    assert_eq!(deletions.get(2).unwrap(), &"huse".anahash(&alphabet));
    assert_eq!(deletions.get(3).unwrap(), &"ouse".anahash(&alphabet));
    assert_eq!(deletions.get(4).unwrap(), &"hous".anahash(&alphabet));
}

#[test]
fn test0202_iterator_parents_dup() {
    //This one has duplicate letters, but no duplicate
    //anagram output will be generated, we do only
    //1 deletion
    let (alphabet, alphabet_size) = get_test_alphabet();
    let anavalue: AnaValue = "pass".anahash(&alphabet);
    let mut chars: Vec<AnaValue> = Vec::new();
    let mut deletions: Vec<AnaValue> = Vec::new();
    for deletion in anavalue.iter_parents(alphabet_size) {
       chars.push(AnaValue::character(deletion.charindex));
       deletions.push(deletion.value.clone());
    }
    assert_eq!(chars.len(),3, "Checking length of results",);
    assert_eq!(chars.get(0).unwrap(), &"s".anahash(&alphabet));
    assert_eq!(chars.get(1).unwrap(), &"p".anahash(&alphabet));
    assert_eq!(chars.get(2).unwrap(), &"a".anahash(&alphabet));
    assert_eq!(deletions.get(0).unwrap(), &"pas".anahash(&alphabet));
    assert_eq!(deletions.get(1).unwrap(), &"ass".anahash(&alphabet));
    assert_eq!(deletions.get(2).unwrap(), &"pss".anahash(&alphabet));
}

#[test]
fn test0203_iterator_recursive_singlebeam() {
    let (alphabet, alphabet_size) = get_test_alphabet();
    let anavalue: AnaValue = "house".anahash(&alphabet);
    let mut chars: Vec<AnaValue> = Vec::new();
    let mut depths: Vec<_> = Vec::new();
    let mut deletions: Vec<AnaValue> = Vec::new();
    for (deletion, depth) in anavalue.iter(alphabet_size) {
       chars.push(AnaValue::character(deletion.charindex));
       deletions.push(deletion.value.clone());
       depths.push(depth);
    }
    assert_eq!(chars.len(), 5, "Checking length of results",);
    assert_eq!(chars.get(0).unwrap(), &"u".anahash(&alphabet));
    assert_eq!(chars.get(1).unwrap(), &"s".anahash(&alphabet));
    assert_eq!(chars.get(2).unwrap(), &"o".anahash(&alphabet));
    assert_eq!(chars.get(3).unwrap(), &"h".anahash(&alphabet));
    assert_eq!(chars.get(4).unwrap(), &"e".anahash(&alphabet));
    assert_eq!(deletions.get(0).unwrap(), &"hose".anahash(&alphabet));
    assert_eq!(deletions.get(1).unwrap(), &"hoe".anahash(&alphabet));
    assert_eq!(deletions.get(2).unwrap(), &"he".anahash(&alphabet));
    assert_eq!(deletions.get(3).unwrap(), &"e".anahash(&alphabet));
    assert_eq!(deletions.get(4).unwrap(), &AnaValue::empty());
    assert_eq!(depths, &[1,2,3,4,5]);
}

#[test]
fn test0203_iterator_recursive() {
    //depth first by default
    let (alphabet, alphabet_size) = get_test_alphabet();
    let anavalue: AnaValue = "abcd".anahash(&alphabet);
    let mut depths: Vec<_> = Vec::new();
    let mut deletions: Vec<AnaValue> = Vec::new();
    for (deletion, depth) in anavalue.iter_recursive(alphabet_size, &SearchParams::default()) {
       deletions.push(deletion.value.clone());
       depths.push(depth);
    }
    let mut iter = deletions.iter();
    assert_eq!(iter.next().unwrap(), &"abc".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &AnaValue::empty());
    assert_eq!(iter.next().unwrap(), &"b".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &AnaValue::empty());
    assert_eq!(iter.next().unwrap(), &"ac".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &AnaValue::empty());
    assert_eq!(iter.next().unwrap(), &"c".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &AnaValue::empty());
    assert_eq!(iter.next().unwrap(), &"bc".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"b".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &AnaValue::empty());
    assert_eq!(iter.next().unwrap(), &"c".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &AnaValue::empty());
    assert_eq!(iter.next().unwrap(), &"abd".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    //.. and more
}

#[test]
fn test0203_iterator_recursive_no_empty_leaves() {
    //depth first by default
    let (alphabet, alphabet_size) = get_test_alphabet();
    let anavalue: AnaValue = "abcd".anahash(&alphabet);
    let mut depths: Vec<_> = Vec::new();
    let mut deletions: Vec<AnaValue> = Vec::new();
    for (deletion, depth) in anavalue.iter_recursive(alphabet_size, &SearchParams {
        allow_empty_leaves: false,
        ..Default::default()}) {
       deletions.push(deletion.value.clone());
       depths.push(depth);
    }
    let mut iter = deletions.iter();
    assert_eq!(iter.next().unwrap(), &"abc".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"b".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"ac".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"c".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"bc".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"b".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"c".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"abd".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    //.. and more
}

#[test]
fn test0203_iterator_recursive_no_duplicates() {
    //depth first by default
    let (alphabet, alphabet_size) = get_test_alphabet();
    let anavalue: AnaValue = "abcd".anahash(&alphabet);
    let mut depths: Vec<_> = Vec::new();
    let mut deletions: Vec<AnaValue> = Vec::new();
    for (deletion, depth) in anavalue.iter_recursive(alphabet_size, &SearchParams {
        allow_empty_leaves: false,
        allow_duplicates: false,
        ..Default::default()}) {
       deletions.push(deletion.value.clone());
       depths.push(depth);
    }
    let mut iter = deletions.iter();
    assert_eq!(iter.next().unwrap(), &"abc".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"b".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"ac".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"c".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"bc".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"abd".anahash(&alphabet));
    //.. and more
}

#[test]
fn test0203_iterator_recursive_bfs() {
    //depth first by default
    let (alphabet, alphabet_size) = get_test_alphabet();
    let anavalue: AnaValue = "abcd".anahash(&alphabet);
    let mut depths: Vec<_> = Vec::new();
    let mut deletions: Vec<AnaValue> = Vec::new();
    for (deletion, depth) in anavalue.iter_recursive(alphabet_size, &SearchParams {
        breadthfirst: true,
        ..Default::default()}) {
       deletions.push(deletion.value.clone());
       depths.push(depth);
    }
    let mut iter = deletions.iter();
    let mut dpit = depths.iter();
    assert_eq!(iter.next().unwrap(), &"abc".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"abd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"acd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"bcd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);

    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"ac".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"bc".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"ad".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"bd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"ac".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"ad".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"cd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"bc".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"bd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"cd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);
    assert_eq!(iter.next().unwrap(), &"b".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);

    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);
    assert_eq!(iter.next().unwrap(), &"c".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);

    //
    //.. and way more duplicates!
}

#[test]
fn test0203_iterator_recursive_bfs_no_duplicates() {
    //depth first by default
    let (alphabet, alphabet_size) = get_test_alphabet();
    let anavalue: AnaValue = "abcd".anahash(&alphabet);
    let mut depths: Vec<_> = Vec::new();
    let mut deletions: Vec<AnaValue> = Vec::new();
    for (deletion, depth) in anavalue.iter_recursive(alphabet_size, &SearchParams {
        breadthfirst: true,
        allow_duplicates: false,
        allow_empty_leaves: false,
        ..Default::default()}) {
       deletions.push(deletion.value.clone());
       depths.push(depth);
    }
    let mut iter = deletions.iter();
    let mut dpit = depths.iter();
    assert_eq!(iter.next().unwrap(), &"abc".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"abd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"acd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"bcd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);

    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"ac".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"bc".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"ad".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"bd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"cd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);
    assert_eq!(iter.next().unwrap(), &"b".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);
    assert_eq!(iter.next().unwrap(), &"c".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);
    assert_eq!(iter.next().unwrap(), &"d".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);
    assert_eq!(iter.next(), None); //all done!
    assert_eq!(dpit.next(), None);
}

#[test]
fn test0203_iterator_recursive_bfs_max_dist() {
    //depth first by default
    let (alphabet, alphabet_size) = get_test_alphabet();
    let anavalue: AnaValue = "abcd".anahash(&alphabet);
    let mut depths: Vec<_> = Vec::new();
    let mut deletions: Vec<AnaValue> = Vec::new();
    for (deletion, depth) in anavalue.iter_recursive(alphabet_size, &SearchParams {
        breadthfirst: true,
        allow_duplicates: false,
        allow_empty_leaves: false,
        max_distance: Some(3),
        ..Default::default()}) {
       deletions.push(deletion.value.clone());
       depths.push(depth);
    }
    let mut iter = deletions.iter();
    let mut dpit = depths.iter();
    assert_eq!(iter.next().unwrap(), &"abc".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"abd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"acd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"bcd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);

    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"ac".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"bc".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"ad".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"bd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"cd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);
    assert_eq!(iter.next().unwrap(), &"b".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);
    assert_eq!(iter.next().unwrap(), &"c".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);
    assert_eq!(iter.next().unwrap(), &"d".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &3);
    assert_eq!(iter.next(), None); //all done!
    assert_eq!(dpit.next(), None);
}

#[test]
fn test0203_iterator_recursive_bfs_max_dist2() {
    //depth first by default
    let (alphabet, alphabet_size) = get_test_alphabet();
    let anavalue: AnaValue = "abcd".anahash(&alphabet);
    let mut depths: Vec<_> = Vec::new();
    let mut deletions: Vec<AnaValue> = Vec::new();
    for (deletion, depth) in anavalue.iter_recursive(alphabet_size, &SearchParams {
        breadthfirst: true,
        allow_duplicates: false,
        allow_empty_leaves: false,
        max_distance: Some(2),
        ..Default::default()}) {
       deletions.push(deletion.value.clone());
       depths.push(depth);
    }
    let mut iter = deletions.iter();
    let mut dpit = depths.iter();
    assert_eq!(iter.next().unwrap(), &"abc".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"abd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"acd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);
    assert_eq!(iter.next().unwrap(), &"bcd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &1);

    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"ac".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"bc".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"ad".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next().unwrap(), &"bd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);

    assert_eq!(iter.next().unwrap(), &"cd".anahash(&alphabet));
    assert_eq!(dpit.next().unwrap(), &2);
    assert_eq!(iter.next(), None); //all done!
    assert_eq!(dpit.next(), None);
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
fn test0303_damereau_levenshtein2() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    assert_eq!(damerau_levenshtein(&"hipotesis".normalize_to_alphabet(&alphabet), &"hypothesis".normalize_to_alphabet(&alphabet),99), Some(2));
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
    let _model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
}

#[test]
fn test0401_model_build() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
    let lexicon: &[&str] = &["rites","tiers", "tires","tries","tyres","rides","brides","dire"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,&VocabParams::default());
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
fn test0402_model_anagrams() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
    let lexicon: &[&str] = &["rites","tiers", "tires","tries","tyres","rides","brides","dire"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,&VocabParams::default());
    }
    model.build();
    assert!(model.has(&"rites"));
    assert_eq!(model.get_anagram_instances(&"rites").iter().map(|item| item.text.clone()).collect::<Vec<String>>(),
             &["rites","tiers","tires","tries"]
    );
}

#[test]
fn test0403_model_anagrams() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
    let lexicon: &[&str] = &["rites","tiers", "tires","tries","tyres","rides","brides","dire"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,&VocabParams::default());
    }
    model.build();
    model.find_variants("rite", &get_test_searchparams(), None);
}

#[test]
fn test0404_score_test() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
    let lexicon: &[&str] = &["huis","huls"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,&VocabParams::default());
    }
    model.build();
    let results = model.find_variants("huys", &get_test_searchparams(), None);
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
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
    let lexicon: &[&str] = &["huis","huls"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,&VocabParams::default());
    }
    model.add_to_confusables("-[y]+[i]",1.1).expect("added to confusables");
    model.build();
    let results = model.find_variants("huys", &get_test_searchparams(), None);
    assert_eq!( model.decoder.get(results.get(0).unwrap().0 as usize).unwrap().text, "huis");
    assert_eq!( model.decoder.get(results.get(1).unwrap().0 as usize).unwrap().text, "huls");
    assert!( results.get(0).unwrap().1 > results.get(1).unwrap().1, "score of huis should be greater than that of huls" );
}

#[test]
fn test0503_confusable_test2() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
    let lexicon: &[&str] = &["huis","huls"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,&VocabParams::default());
    }
    model.add_to_confusables("-[y]+[i]",1.1).expect("added to confusables");
    model.build();
    let results = model.find_variants("Huys", &get_test_searchparams(), None);
    assert_eq!( model.decoder.get(results.get(0).unwrap().0 as usize).unwrap().text, "huis");
    assert_eq!( model.decoder.get(results.get(1).unwrap().0 as usize).unwrap().text, "huls");
    assert!( results.get(0).unwrap().1 > results.get(1).unwrap().1, "score of huis should be greater than that of huls" );
}

#[test]
fn test0504_confusable_nomatch() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
    let lexicon: &[&str] = &["huis","huls"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,&VocabParams::default());
    }
    model.add_to_confusables("-[y]+[p]",1.1).expect("added to confusables");
    model.build();
    let results = model.find_variants("Huys", &get_test_searchparams(), None);
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
fn test0602_find_ngrams_unigram1() {
    let text = "dit is een mooie test";
    let boundaries = find_boundaries(&text);
    let ngrams = find_match_ngrams(text, &boundaries, 1, 0, None);
    assert_eq!( ngrams.len() , 5 );
    assert_eq!( ngrams.get(0).unwrap().text , "dit" );
    assert_eq!( ngrams.get(1).unwrap().text , "is" );
    assert_eq!( ngrams.get(2).unwrap().text , "een" );
    assert_eq!( ngrams.get(3).unwrap().text , "mooie" );
    assert_eq!( ngrams.get(4).unwrap().text , "test" );
}

#[test]
fn test0603_find_ngrams_unigram2() {
    let text = "dit is een mooie test.";
    let boundaries = find_boundaries(&text);
    let ngrams = find_match_ngrams(text, &boundaries, 1, 0, None);
    assert_eq!( ngrams.len() , 5 );
    assert_eq!( ngrams.get(0).unwrap().text , "dit" );
    assert_eq!( ngrams.get(1).unwrap().text , "is" );
    assert_eq!( ngrams.get(2).unwrap().text , "een" );
    assert_eq!( ngrams.get(3).unwrap().text , "mooie" );
    assert_eq!( ngrams.get(4).unwrap().text , "test" );
}

#[test]
fn test0604_find_ngrams_unigram3() {
    let text =  "hello, world!";
    let boundaries = find_boundaries(&text);
    let ngrams = find_match_ngrams(text, &boundaries, 1, 0, None);
    assert_eq!( ngrams.len() , 2 );
    assert_eq!( ngrams.get(0).unwrap().text , "hello" );
    assert_eq!( ngrams.get(1).unwrap().text , "world" );
}

#[test]
fn test0605_find_ngrams_bigrams() {
    let text = "dit is een mooie test.";
    let boundaries = find_boundaries(&text);
    eprintln!("{:?}", boundaries);
    assert_eq!( boundaries.len() , 5 );
    let ngrams = find_match_ngrams(text, &boundaries, 2, 0, None);
    eprintln!("{:?}", ngrams);
    assert_eq!( ngrams.len() , 4 );
    assert_eq!( ngrams.get(0).unwrap().text , "dit is" );
    assert_eq!( ngrams.get(1).unwrap().text , "is een" );
    assert_eq!( ngrams.get(2).unwrap().text , "een mooie" );
    assert_eq!( ngrams.get(3).unwrap().text , "mooie test" );
    //note: final punctuation is a hard boundary and not returned
}

#[test]
fn test0606_find_ngrams_bigrams2() {
    let text =  "hello,world!";
    let boundaries = find_boundaries(&text);
    let ngrams = find_match_ngrams(text, &boundaries, 2, 0, None);
    assert_eq!( ngrams.len() , 1 );
    assert_eq!( ngrams.get(0).unwrap().text , "hello,world" ); //this counts as a bigram ("," is a boundary)
}

#[test]
fn test0607_find_ngrams_bigrams3() {
    let text =  "hello, world!";
    let boundaries = find_boundaries(&text);
    let ngrams = find_match_ngrams(text, &boundaries, 2, 0, None);
    assert_eq!( ngrams.len() , 1 );
    assert_eq!( ngrams.get(0).unwrap().text , "hello, world" ); //this counts as a bigram (", " is a boundary)
}

#[test]
fn test0608_find_ngrams_bigrams4() {
    let text =  "hello!";
    let boundaries = find_boundaries(&text);
    let ngrams = find_match_ngrams(text, &boundaries, 2, 0, None);
    assert_eq!( ngrams.len() , 0 ); //no bigrams in this text
}


#[test]
fn test0701_find_all_matches_unigram_only() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
    let lexicon: &[&str] = &["I","think","sink","you","are","right"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,&VocabParams::default());
    }
    model.build();
    let matches = model.find_all_matches("I tink you are rihgt", &get_test_searchparams().with_max_ngram(1));
    assert!( !matches.is_empty() );
    assert_eq!( matches.get(0).unwrap().text , "I" );
    assert_eq!( matches.get(1).unwrap().text , "tink" );
    assert_eq!( model.match_to_str(matches.get(1).unwrap()) , "think" );
    assert_eq!( matches.get(2).unwrap().text , "you" );
    assert_eq!( matches.get(3).unwrap().text , "are" );
    assert_eq!( matches.get(4).unwrap().text , "rihgt" );
    assert_eq!( model.match_to_str(matches.get(4).unwrap()) , "right" );
}

#[test]
fn test0702_find_all_matches() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
    model.add_to_vocabulary("I",Some(2),&VocabParams::default());
    model.add_to_vocabulary("think",Some(2), &VocabParams::default());
    model.add_to_vocabulary("sink",Some(1), &VocabParams::default());
    model.add_to_vocabulary("you",Some(2), &VocabParams::default());

    model.add_to_vocabulary("are",Some(2),&VocabParams::default());
    model.add_to_vocabulary("right",Some(2),&VocabParams::default());
    model.add_to_vocabulary("are right",Some(2),&VocabParams::default());
    model.add_to_vocabulary("<bos> I",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("I think",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("I sink",Some(1),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("you are",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("right <eos>",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.build();
    let matches = model.find_all_matches("I tink you are rihgt", &get_test_searchparams());
    assert!( !matches.is_empty() );
    assert_eq!( matches.get(0).unwrap().text , "I" );
    assert_eq!( model.match_to_str(matches.get(0).unwrap()) , "I" );
    assert_eq!( matches.get(1).unwrap().text , "tink" );
    assert_eq!( model.match_to_str(matches.get(1).unwrap()) , "think" );
    assert_eq!( matches.get(2).unwrap().text , "you" );
    assert_eq!( model.match_to_str(matches.get(2).unwrap()) , "you" );
    assert_eq!( matches.get(3).unwrap().text , "are rihgt" ); //system opts for the bigram here
    assert_eq!( model.match_to_str(matches.get(3).unwrap()) , "are right" );
}

#[test]
fn test0703_find_all_matches_linebreak() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
    model.add_to_vocabulary("I",Some(2),&VocabParams::default());
    model.add_to_vocabulary("think",Some(2),&VocabParams::default());
    model.add_to_vocabulary("sink",Some(1),&VocabParams::default());
    model.add_to_vocabulary("you",Some(2),&VocabParams::default());
    model.add_to_vocabulary("are",Some(2),&VocabParams::default());
    model.add_to_vocabulary("right",Some(2),&VocabParams::default());
    model.add_to_vocabulary("are right",Some(2),&VocabParams::default());
    model.add_to_vocabulary("<bos> I",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("I think",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("I sink",Some(1),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("you are",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("right <eos>",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.build();
    let matches = model.find_all_matches("I tink you are\nrihgt",&get_test_searchparams());
    assert!( !matches.is_empty() );
    assert_eq!( matches.get(0).unwrap().text , "I" );
    assert_eq!( model.match_to_str(matches.get(0).unwrap()) , "I" );
    assert_eq!( matches.get(1).unwrap().text , "tink" );
    assert_eq!( model.match_to_str(matches.get(1).unwrap()) , "think" );
    assert_eq!( matches.get(2).unwrap().text , "you" );
    assert_eq!( model.match_to_str(matches.get(2).unwrap()) , "you" );
    assert_eq!( matches.get(3).unwrap().text , "are\nrihgt" ); //system opts for the bigram here
    assert_eq!( model.match_to_str(matches.get(3).unwrap()) , "are right" );
}

#[test]
fn test0704_find_all_matches_two_batches() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 1);
    model.add_to_vocabulary("I",Some(2),&VocabParams::default());
    model.add_to_vocabulary("think",Some(2),&VocabParams::default());
    model.add_to_vocabulary("sink",Some(1),&VocabParams::default());
    model.add_to_vocabulary("you",Some(2),&VocabParams::default());
    model.add_to_vocabulary("are",Some(2),&VocabParams::default());
    model.add_to_vocabulary("right",Some(2),&VocabParams::default());
    model.add_to_vocabulary("am",Some(2),&VocabParams::default());
    model.add_to_vocabulary("sure",Some(2),&VocabParams::default());
    model.add_to_vocabulary("are right",Some(2),&VocabParams::default());
    model.add_to_vocabulary("<bos> I",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("I think",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("I sink",Some(1),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("you are",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("right <eos>",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.add_to_vocabulary("I am",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    // "am sure" -> model has to figure this one out itself using an unknown transition
    model.add_to_vocabulary("sure <eos>",Some(2),&VocabParams { vocab_type: VocabType::NoIndex, ..VocabParams::default() });
    model.build();
    let matches = model.find_all_matches("I tink you are rihgt\n\nI am sur", &get_test_searchparams());
    assert!( !matches.is_empty() );
    assert_eq!( matches.get(0).unwrap().text , "I" );
    assert_eq!( model.match_to_str(matches.get(0).unwrap()) , "I" );
    assert_eq!( matches.get(1).unwrap().text , "tink" );
    assert_eq!( model.match_to_str(matches.get(1).unwrap()) , "think" );
    assert_eq!( matches.get(2).unwrap().text , "you" );
    assert_eq!( model.match_to_str(matches.get(2).unwrap()) , "you" );
    assert_eq!( matches.get(3).unwrap().text , "are rihgt" ); //system opts for the bigram here
    assert_eq!( model.match_to_str(matches.get(3).unwrap()) , "are right" );
    assert_eq!( matches.get(4).unwrap().text , "I" );
    assert_eq!( model.match_to_str(matches.get(4).unwrap()) , "I" );
    assert_eq!( matches.get(5).unwrap().text , "am" );
    assert_eq!( model.match_to_str(matches.get(5).unwrap()) , "am" );
    assert_eq!( matches.get(6).unwrap().text , "sur" );
    assert_eq!( model.match_to_str(matches.get(6).unwrap()) , "sure" );
}

#[test]
fn test0801_model_variants() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 2);
    let lexicon: &[&str] = &["rites","tiers", "tires","tries","tyres","rides","brides","dire"];
    for text in lexicon.iter() {
        model.add_to_vocabulary(text,None,&VocabParams::default());
    }
    model.add_variants(&vec!("tries", "attempts"), &VocabParams::default().with_vocab_type(VocabType::Intermediate));
    model.build();
    assert!(model.has(&"tries"));
    assert!(model.has(&"attempts"));
    //we look for "attemts", which matches "attempts", but this is just an intermediate towards
    //"tries", which is what is eventually returned.
    let results = model.find_variants("attemts", &get_test_searchparams(), None);
    assert_eq!( model.decoder.get(results.get(0).unwrap().0 as usize).unwrap().text, "tries");
}

#[test]
fn test0901_find_all_matches_with_multiple_lexicons() {
    let (alphabet, _alphabet_size) = get_test_alphabet();
    let mut model = VariantModel::new_with_alphabet(alphabet, Weights::default(), 2);
    assert!(model.read_vocabulary(LEXICON_AMPHIBIANS, &VocabParams::default()).is_ok());
    assert!(model.read_vocabulary(LEXICON_REPTILES, &VocabParams::default()).is_ok());
    model.build();
    let inputwords = vec!("Salamander", "lizard","frog","snake","toad");
    let outputrefwords = vec!("salamander", "lizard","frog","snake","toad");
    let inputstring = inputwords.join(" ");
    let matches = model.find_all_matches(inputstring.as_str(), &get_test_searchparams().with_max_ngram(1).with_single_thread());
    assert_eq!( matches.len(), inputwords.len());

    //Checking input
    for (i, inputword) in inputwords.iter().enumerate() {
        assert_eq!( &matches[i].text, inputword);
    }

    //Checking best variant output
    for (i, (inputword, outputrefword)) in inputwords.iter().zip(outputrefwords.iter()).enumerate() {
        assert_eq!( &model.match_to_str(&matches[i]), outputrefword);
    }

    //salamander
    assert_eq!( model.lexicons[model.match_to_vocabvalue(&matches[0]).expect("must exist").lexindex as usize],
                LEXICON_AMPHIBIANS  );
    //lizard
    assert_eq!( model.lexicons[model.match_to_vocabvalue(&matches[1]).expect("must exist").lexindex as usize],
                LEXICON_REPTILES  );
    //frog
    assert_eq!( model.lexicons[model.match_to_vocabvalue(&matches[1]).expect("must exist").lexindex as usize],
                LEXICON_AMPHIBIANS  );
    //snake
    assert_eq!( model.lexicons[model.match_to_vocabvalue(&matches[1]).expect("must exist").lexindex as usize],
                LEXICON_REPTILES  );
    //toad
    assert_eq!( model.lexicons[model.match_to_vocabvalue(&matches[1]).expect("must exist").lexindex as usize],
                LEXICON_AMPHIBIANS  );

}

