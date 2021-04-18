use std::collections::HashMap;
use std::cmp::min;
use crate::types::*;

///Compute levenshtein distance between two normalised strings
///Returns None if the maximum distance is exceeded
pub fn levenshtein(a: &[CharIndexType], b: &[CharIndexType], max_distance: CharIndexType) -> Option<CharIndexType> {
    //Freely adapted from levenshtein-rs (MIT licensed, 2016 Titus Wormer <tituswormer@gmail.com>)
    if a == b {
        return Some(0);
    }


    let length_a = a.len();
    let length_b = b.len();

    if length_a == 0 {
        if length_b > max_distance as usize {
            return None;
        } else {
            return Some(length_b as CharIndexType);
        }
    } else if length_a > length_b {
        if length_a - length_b > max_distance as usize {
            return None;
        }
    }
    if length_b == 0 {
        if length_a > max_distance as usize {
            return None;
        } else {
            return Some(length_a as CharIndexType);
        }
    } else if length_b > length_a {
        if length_b - length_a > max_distance as usize {
            return None;
        }
    }

    let mut cache: Vec<usize> = (1..).take(length_a).collect();
    let mut distance_a;
    let mut distance_b;
    let mut result = 0;

    for (index_b, elem_b) in b.iter().enumerate() {
        result = index_b;
        distance_a = index_b;

        for (index_a, elem_a) in a.iter().enumerate() {
            distance_b = if elem_a == elem_b {
                distance_a
            } else {
                distance_a + 1
            };

            distance_a = cache[index_a];

            result = if distance_a > result {
                if distance_b > result {
                    result + 1
                } else {
                    distance_b
                }
            } else if distance_b > distance_a {
                distance_a + 1
            } else {
                distance_b
            };

            cache[index_a] = result;
        }
    }

    if result > max_distance as usize {
        None
    } else {
        Some(result as CharIndexType)
    }
}


/// Calculates the Damerau-Levenshtein distance between two strings.
///
/// This implementation was adapted from the one in the distance crate by Marcus Brummer (Apache 2 License)
///
/// # Damerau-Levenshtein distance
///
/// The [Damerau-Levenshtein distance](https://en.wikipedia.org/wiki/Damerau%E2%80%93Levenshtein_distance) is the number of per-character changes
/// (insertion, deletion, substitution & transposition) that are neccessary to convert one string into annother.
/// The original Levenshtein distance does not take transposition into account.
/// This implementation does fully support unicode strings.
///
/// ## Complexity
/// m := len(s) + 2
/// n := len(t) + 2
///
/// Time complexity:   O(mn)
/// Space complexity:  O(mn + m)
pub fn damerau_levenshtein(s: &[CharIndexType], t: &[CharIndexType], max_distance: CharIndexType) -> Option<CharIndexType> {
    let len_s = s.len();
    let len_t = t.len();


    if len_s == 0 {
        if len_t > max_distance as usize {
            return None;
        } else {
            return Some(len_t as CharIndexType);
        }
    } else if len_s > len_t {
        if len_s - len_t > max_distance as usize {
            return None;
        }
    }
    if len_t == 0 {
        if len_s > max_distance as usize {
            return None;
        } else {
            return Some(len_s as CharIndexType);
        }
    } else if len_t > len_s {
        if len_t - len_s > max_distance as usize {
            return None;
        }
    }

    let distance_upper_bound = len_t + len_s;

    // initialize the matrix
    let mut mat: Vec<Vec<usize>> = vec![vec![0; len_t + 2]; len_s + 2];
    mat[0][0] = distance_upper_bound;
    for i in 0..(len_s + 1) {
        mat[i+1][0] = distance_upper_bound;
        mat[i+1][1] = i;
    }
    for i in 0..(len_t + 1) {
        mat[0][i+1] = distance_upper_bound;
        mat[1][i+1] = i;
    }

    let mut char_map: HashMap<CharIndexType, CharIndexType> = HashMap::new();
    // apply edit operations
    for (i, s_char) in s.iter().enumerate() {
        let mut db = 0;
        let i = i + 1;

        for (j, t_char) in t.iter().enumerate() {
            let j = j + 1;
            let last: usize = *char_map.get(&t_char).unwrap_or(&0) as usize;

            let cost = if s_char == t_char { 0 } else { 1 };
            mat[i+1][j+1] = min4(
                mat[i+1][j] + 1,     // deletion
                mat[i][j+1] + 1,     // insertion
                mat[i][j] + cost,    // substitution
                mat[last][db] + (i - last - 1) + 1 + (j - db - 1) // transposition
            );

            // that's like s_char == t_char but more efficient
            if cost == 0 {
                db = j;
            }
        }

        char_map.insert(*s_char, i as CharIndexType);
    }

    let result = mat[len_s + 1][len_t + 1];
    if result > max_distance.into() {
        None
    } else {
        Some(mat[len_s + 1][len_t + 1] as CharIndexType)
    }
}

pub fn longest_common_substring_length(s1: &[CharIndexType], s2: &[CharIndexType]) -> u16 {
    let mut lcs = 0;

    for i in 0..s1.len() {
        for j in 0..s2.len() {
            if s1[i] == s2[j] {
                let mut tmp_lcs = 1;
                let mut tmp_i = i + 1;
                let mut tmp_j = j + 1;

                while tmp_i < s1.len() && tmp_j < s2.len() && s1[tmp_i] == s2[tmp_j] {
                    tmp_lcs += 1;
                    tmp_i += 1;
                    tmp_j += 1;
                }

                if tmp_lcs > lcs {
                    lcs = tmp_lcs;
                }
            }
        }
    }

    lcs
}

///Computes if the strings share a common prefix, and if so, how long it is
pub fn common_prefix_length(s1: &[CharIndexType], s2: &[CharIndexType]) -> u16 {
    let mut prefixlen = 0;
    for i in 0..min(s1.len(),s2.len()) {
        if s1[i] == s2[i] {
            prefixlen += 1;
        } else {
            break;
        }
    }
    prefixlen
}

///Computes if the strings share a common suffix, and if so, how long it is
pub fn common_suffix_length(s1: &[CharIndexType], s2: &[CharIndexType]) -> u16 {
    let mut suffixlen = 0;
    for i in 0..min(s1.len(),s2.len()) {
        if s1[s1.len() - i - 1] == s2[s2.len() - i - 1] {
            suffixlen += 1;
        } else {
            break;
        }
    }
    suffixlen
}


#[inline(always)]
pub fn min4(a: usize, b: usize, c: usize, d: usize) -> usize {
   return min(min(min(a, b), c), d);
}
