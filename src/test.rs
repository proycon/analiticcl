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

