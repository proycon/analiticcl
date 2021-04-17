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
