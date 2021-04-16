//#[macro_use]
//extern crate matches;

use std::str;
use analiticcl::*;
use analiticcl::test::*;

#[test]
fn test001_get_alphabet() {
    let (alphabet, alphabet_size) = get_test_alphabet();
    assert_eq!(alphabet.len(), 27);
}

#[test]
fn test001_hash() {
    assert!(true);
        //  assert!(false, format!("Instantiation failed with error: {}",err));
}
