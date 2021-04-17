//#[macro_use]
//extern crate matches;

use std::str;
use std::ops::Deref;
use analiticcl::*;
use analiticcl::test::*;


#[test]
fn test0001_alphabet() {
    let (alphabet, alphabet_size) = get_test_alphabet();
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
    let (alphabet, alphabet_size) = get_test_alphabet();

    //this is a hash that would overflow any normal 64-bit int, but it should hash fine
    assert_eq!(AnaValue::empty(), AnaValue::from(1 as usize));
}

#[test]
fn test0103_hash_basic() {
    let (alphabet, alphabet_size) = get_test_alphabet();

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
    let (alphabet, alphabet_size) = get_test_alphabet();

    //the alphabet may define multiple values that map to the same
    //the provided example alphabet does so for case-differences
    //and periods and commas:

    assert_eq!("abc".anahash(&alphabet), "ABC".anahash(&alphabet));
    assert_eq!("abc".anahash(&alphabet), "bAc".anahash(&alphabet));
    assert_eq!("a.b".anahash(&alphabet), "a,b".anahash(&alphabet));
}

#[test]
fn test0104_hash_big() {
    let (alphabet, alphabet_size) = get_test_alphabet();

    //this is a hash that would overflow any normal 64-bit int, but it should hash fine
    assert!("xyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyz".anahash(&alphabet) > AnaValue::empty());
}


#[test]
fn test0105_hash_anagram() {
    let (alphabet, alphabet_size) = get_test_alphabet();
    assert_eq!("stressed".anahash(&alphabet),"desserts".anahash(&alphabet) );
    assert_eq!("dormitory".anahash(&alphabet),"dirtyroom".anahash(&alphabet) );
    assert_eq!("presents".anahash(&alphabet),"serpents".anahash(&alphabet) );
}

#[test]
fn test0106_hash_insertion() {
    let (alphabet, alphabet_size) = get_test_alphabet();

    let ab = "ab".anahash(&alphabet);
    let c = "c".anahash(&alphabet);
    let abc = "abc".anahash(&alphabet);

    assert_eq!(ab.insert(&c), abc);
    assert_eq!(c.insert(&ab), abc);
}

#[test]
fn test0107_hash_containment() {
    let (alphabet, alphabet_size) = get_test_alphabet();

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
    let (alphabet, alphabet_size) = get_test_alphabet();

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
    assert_eq!(iter.next().unwrap(), &"abc".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"abd".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"acd".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"bcd".anahash(&alphabet));

    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"ac".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"bc".anahash(&alphabet));

    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"ad".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"bd".anahash(&alphabet));

    assert_eq!(iter.next().unwrap(), &"ac".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"ad".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"cd".anahash(&alphabet));

    assert_eq!(iter.next().unwrap(), &"bc".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"bd".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"cd".anahash(&alphabet));

    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"b".anahash(&alphabet));
    //.. and more
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
        ..Default::default()}) {
       deletions.push(deletion.value.clone());
       depths.push(depth);
    }
    let mut iter = deletions.iter();
    assert_eq!(iter.next().unwrap(), &"abc".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"abd".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"acd".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"bcd".anahash(&alphabet));

    assert_eq!(iter.next().unwrap(), &"ab".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"ac".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"bc".anahash(&alphabet));

    assert_eq!(iter.next().unwrap(), &"ad".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"bd".anahash(&alphabet));

    assert_eq!(iter.next().unwrap(), &"cd".anahash(&alphabet));

    assert_eq!(iter.next().unwrap(), &"a".anahash(&alphabet));
    assert_eq!(iter.next().unwrap(), &"b".anahash(&alphabet));
    //.. and more
}
