//#[macro_use]
//extern crate matches;

use std::str;
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
