extern crate clap;

use std::fs::File;
use std::io::{self, BufReader,BufRead};
use clap::{Arg, App};
use std::collections::HashMap;

use analiticcl::*;

fn output_matches_as_tsv(model: &VariantModel, input: &str, variants: &Vec<(VocabId, f64)>) {
    print!("{}",input);
    for (vocab_id, score) in variants {
        print!("\t{}\t{}\t",model.get_vocab(*vocab_id).expect("getting vocab by id").text, score);
    }
    println!();
}

fn output_reverse_index(model: &VariantModel, reverseindex: &ReverseIndex) {
    for (vocab_id, variants) in reverseindex.iter() {
        print!("{}",model.get_vocab(*vocab_id).expect("getting vocab by id").text);
        let mut variants: Vec<&(Variant,f64)> = variants.iter().collect();
        variants.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); //sort by score, descending order
        for (variant, score) in variants {
            let variant_text = match variant {
                Variant::Known(variant_vocab_id) => {
                    model.get_vocab(*variant_vocab_id).expect("getting variant vocab by id").text.as_str()
                },
                Variant::Unknown(variant_text) => {
                    variant_text.as_str()
                }
            };
            print!("\t{}\t{}\t",variant_text, score);
        }
        println!();
    }
}

fn process(model: &VariantModel, input: &str, reverseindex: Option<&mut ReverseIndex>, max_anagram_distance: u8, max_edit_distance: u8) {
    let variants = model.find_variants(&input, max_anagram_distance, max_edit_distance);
    if let Some(reverseindex) = reverseindex {
        //we are asked to build a reverse index
        for (vocab_id,score) in variants.iter() {
            model.add_to_reverse_index(reverseindex, input, *vocab_id, *score);
        }
    } else {
        //Normal output mode
        output_matches_as_tsv(model, input, &variants);
    }
}

fn main() {
    let args = App::new("Analiticcl")
                    .version("0.1")
                    .author("Maarten van Gompel (proycon) <proycon@anaproy.nl>")
                    .about("Spelling variant matching / approximate string matching / fuzzy search")
                    //snippet hints --> addargb,addargs,addargi,addargf,addargpos
                    .arg(Arg::with_name("lexicon")
                        .long("lexicon")
                        .short("l")
                        .help("Lexicon against which all matches are made (may be used multiple times). The lexicon should only contain validated items, if not, use --corpus instead. The lexicon should be a tab separated file with each entry on one line, columns may be used for frequency information. This option may be used multiple times for multiple lexicons.")
                        .takes_value(true)
                        .number_of_values(1)
                        .multiple(true)
                        .required_unless("corpus"))
                    .arg(Arg::with_name("corpus")
                        .long("corpus")
                        .short("f")
                        .help("Corpus-derived lexicon against which matches are made (may be used multiple times). Format is the same as for --lexicon. This optionmay be used multiple times.")
                        .takes_value(true)
                        .number_of_values(1)
                        .multiple(true)
                        .required_unless("lexicon"))
                    .arg(Arg::with_name("alphabet")
                        .long("alphabet")
                        .short("a")
                        .help("Alphabet file")
                        .takes_value(true)
                        .required(true))
                    .arg(Arg::with_name("max_anagram_distance")
                        .long("max-anagram-distance")
                        .short("k")
                        .help("Maximum anagram distance. This impacts the size of the search space")
                        .takes_value(true)
                        .default_value("3"))
                    .arg(Arg::with_name("max_edit_distance")
                        .long("max-edit-distance")
                        .short("d")
                        .help("Maximum edit distance (levenshtein)")
                        .takes_value(true)
                        .default_value("3"))
                    .arg(Arg::with_name("files")
                        .help("Input files")
                        .takes_value(true)
                        .multiple(true)
                        .required(false))
                    .arg(Arg::with_name("debug")
                        .long("debug")
                        .short("D")
                        .help("Debug")
                        .required(false))
                    .arg(Arg::with_name("output-index")
                        .long("output-index")
                        .short("O")
                        .help("Output the entire anagram hash index")
                        .required(false))
                    .arg(Arg::with_name("reverse-index")
                        .long("reverse-index")
                        .short("r")
                        .help("Collect and output all variants for each item in the input lexicon")
                        .required(false))
                    .arg(Arg::with_name("weight-ld")
                        .long("weight-ld")
                        .help("Weight attributed to Damarau-Levenshtein distance in scoring")
                        .takes_value(true)
                        .default_value("1.0"))
                    .arg(Arg::with_name("weight-lcs")
                        .long("weight-lcs")
                        .help("Weight attributed to Longest common substring length in scoring")
                        .takes_value(true)
                        .default_value("1.0"))
                    .arg(Arg::with_name("weight-prefix")
                        .long("weight-prefix")
                        .help("Weight attributed to longest common prefix length in scoring")
                        .takes_value(true)
                        .default_value("1.0"))
                    .arg(Arg::with_name("weight-suffix")
                        .long("weight-suffix")
                        .help("Weight attributed to longest common suffix length in scoring")
                        .takes_value(true)
                        .default_value("1.0"))
                    .arg(Arg::with_name("weight-freq")
                        .long("weight-freq")
                        .help("Weight attributed to frequency in scoring")
                        .takes_value(true)
                        .default_value("1.0"))
                    .arg(Arg::with_name("weight-lex")
                        .long("weight-lex")
                        .help("Weight attributed to items that are in the lexicon, will always be 0 for items only in the corpus")
                        .takes_value(true)
                        .default_value("1.0"))
                    .get_matches();

    eprintln!("Initializing model...");

    let weights = Weights {
        ld: args.value_of("weight-ld").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        lcs: args.value_of("weight-lcs").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        prefix: args.value_of("weight-prefix").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        suffix: args.value_of("weight-suffix").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        freq: args.value_of("weight-freq").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        lex: args.value_of("weight-lex").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
    };

    let mut model = VariantModel::new(
        args.value_of("alphabet").unwrap(),
        weights,
        args.is_present("debug")
    );

    eprintln!("Loading lexicons...");

    if args.is_present("lexicon") {
        for filename in args.values_of("lexicon").unwrap().collect::<Vec<&str>>() {
            model.read_vocabulary(filename, &VocabParams::default(), 1.0).expect(&format!("Error reading {}", filename));
        }
    }

    if args.is_present("corpus") {
        for filename in args.values_of("corpus").unwrap().collect::<Vec<&str>>() {
            model.read_vocabulary(filename, &VocabParams::default(), 0.0).expect(&format!("Error reading {}", filename));
        }
    }

    eprintln!("Training model...");
    model.train();

    let max_anagram_distance: u8 = args.value_of("max_anagram_distance").unwrap().parse::<u8>().expect("Anagram distance should be an integer between 0 and 255");
    let max_edit_distance: u8 = args.value_of("max_edit_distance").unwrap().parse::<u8>().expect("Anagram distance should be an integer between 0 and 255");

    let mut reverseindex = if args.is_present("reverse-index") {
        Some(HashMap::new())
    } else {
        None
    };

    if args.is_present("output-index") {
        for (anahash, indexnode) in model.index.iter() {
            if !indexnode.instances.is_empty() {
                print!("{}", anahash);
                for instance in indexnode.instances.iter() {
                    let vocabvalue = model.decoder.get(*instance as usize).expect("decoding instance");
                    print!("\t{}", vocabvalue.text);
                }
                println!()
            }
        }

    } else {
        eprintln!("Testing against model...");
        let files: Vec<_> = if args.is_present("files") {
            args.values_of("files").unwrap().collect()
        } else {
            vec!("-")
        };
        for filename in files {
            match filename {
                "-" | "STDIN" | "stdin"  => {
                    eprintln!("(accepting standard input)");
                    let stdin = io::stdin();
                    let f_buffer = BufReader::new(stdin);
                    for line in f_buffer.lines() {
                        if let Ok(line) = line {
                            process(&model, &line, reverseindex.as_mut(), max_anagram_distance, max_edit_distance);
                        }
                    }
                },
                _ =>  {
                    let f = File::open(filename).expect(format!("ERROR: Unable to open file {}", filename).as_str());
                    let f_buffer = BufReader::new(f);
                    for line in f_buffer.lines() {
                        if let Ok(line) = line {
                            process(&model, &line, reverseindex.as_mut(), max_anagram_distance, max_edit_distance);
                        }
                    }
                }
            }
        }

        if let Some(reverseindex) = reverseindex {
            output_reverse_index(&model, &reverseindex);
        }
    }
}
