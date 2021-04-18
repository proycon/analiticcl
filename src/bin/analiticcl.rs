extern crate clap;

use std::fs::File;
use std::io::{self, BufReader,BufRead};
use clap::{Arg, App};

use analiticcl::*;

fn process_tsv(model: &VariantModel, input: &str, max_anagram_distance: u8, max_edit_distance: u8) {
    let variants = model.find_variants(&input, max_anagram_distance, max_edit_distance);
    print!("{}",input);
    for (variant, score) in variants {
        print!("\t{}\t{}\t",variant, score);
    }
    println!();
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
                        .help("Lexicon against which all matches are made")
                        .takes_value(true)
                        .required(true))
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
                    .arg(Arg::with_name("printindex")
                        .long("printindex")
                        .short("I")
                        .help("Output the entire index")
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
                        .long("weight-prefix")
                        .help("Weight attributed to longest common suffix length in scoring")
                        .takes_value(true)
                        .default_value("1.0"))
                    .arg(Arg::with_name("weight-freq")
                        .long("weight-freq")
                        .help("Weight attributed to frequency in scoring")
                        .takes_value(true)
                        .default_value("1.0"))
                    .get_matches();

    eprintln!("Loading model resources...");

    let weights = Weights {
        ld: args.value_of("weight-ld").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        lcs: args.value_of("weight-lcs").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        prefix: args.value_of("weight-prefix").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        suffix: args.value_of("weight-suffix").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        freq: args.value_of("weight-freq").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
    };

    let mut model = VariantModel::new(
        args.value_of("alphabet").unwrap(),
        args.value_of("lexicon").unwrap(),
        Some(VocabParams::default()),
        weights,
        args.is_present("debug")

    );

    eprintln!("Training model...");
    model.train();

    let max_anagram_distance: u8 = args.value_of("max_anagram_distance").unwrap().parse::<u8>().expect("Anagram distance should be an integer between 0 and 255");
    let max_edit_distance: u8 = args.value_of("max_edit_distance").unwrap().parse::<u8>().expect("Anagram distance should be an integer between 0 and 255");

    if args.is_present("printindex") {
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
            eprintln!("(accepting standard input)");
            vec!("-")
        };
        for filename in files {
            match filename {
                "-" | "STDIN" | "stdin"  => {
                    let stdin = io::stdin();
                    let f_buffer = BufReader::new(stdin);
                    for line in f_buffer.lines() {
                        if let Ok(line) = line {
                            process_tsv(&model, &line, max_anagram_distance, max_edit_distance);
                        }
                    }
                },
                _ =>  {
                    let f = File::open(filename).expect(format!("ERROR: Unable to open file {}", filename).as_str());
                    let f_buffer = BufReader::new(f);
                    for line in f_buffer.lines() {
                        if let Ok(line) = line {
                            process_tsv(&model, &line, max_anagram_distance, max_edit_distance);
                        }
                    }
                }
            }
        }
    }
}
