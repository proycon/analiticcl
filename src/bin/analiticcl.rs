extern crate clap;
extern crate rayon;

use std::fs::File;
use std::io::{self, BufReader,BufRead,Read};
use clap::{Arg, App, SubCommand};
use std::collections::HashMap;
use std::time::SystemTime;
use rayon::prelude::*;


use analiticcl::*;

fn output_matches_as_tsv(model: &VariantModel, input: &str, variants: Option<&Vec<(VocabId, f64)>>, offset: Option<Offset>, output_lexmatch: bool) {
    print!("{}",input);
    if let Some(offset) = offset {
        print!("\t{}:{}",offset.begin, offset.end);
    }
    if let Some(variants) = variants {
        for (vocab_id, score) in variants {
            let vocabvalue = model.get_vocab(*vocab_id).expect("getting vocab by id");
            print!("\t{}\t{}\t", vocabvalue.text, score);
            if  output_lexmatch {
                print!("\t{}", model.lexicons.get(vocabvalue.lexindex as usize).expect("valid lexicon index"));
            }
        }
    }
    println!();
}

fn output_matches_as_json(model: &VariantModel, input: &str, variants: Option<&Vec<(VocabId, f64)>>, offset: Option<Offset>, output_lexmatch: bool, seqnr: usize) {
    if seqnr > 1 {
        println!(",")
    }
    print!("    {{ \"input\": \"{}\"", input.replace("\"","\\\"").as_str());
    if let Some(offset) = offset {
        print!(", \"begin\": {}, \"end\": {}", offset.begin, offset.end);
    }
    if let Some(variants) = variants {
        println!(", \"variants\": [ ");
        let l = variants.len();
        for (i, (vocab_id, score)) in variants.iter().enumerate() {
            let vocabvalue = model.get_vocab(*vocab_id).expect("getting vocab by id");
            print!("        {{ \"text\": \"{}\", \"score\": {}", vocabvalue.text.replace("\"","\\\""), score);
            if  output_lexmatch {
                print!(", \"lexicon\": \"{}\"", model.lexicons.get(vocabvalue.lexindex as usize).expect("valid lexicon index"));
            }
            if i < l - 1 {
                println!(" }},");
            } else {
                println!(" }}");
            }
        }
        println!("    ] }}");
    } else {
        println!(" }}");
    }
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

fn process(model: &VariantModel, inputstream: impl Read, reverseindex: &mut Option<ReverseIndex>, max_anagram_distance: u8, max_edit_distance: u8, max_matches: usize, score_threshold: f64, stop_criterion: StopCriterion, output_lexmatch: bool, json: bool, cache: &mut Option<Cache>, progress: bool) {
    let mut seqnr = 0;
    let f_buffer = BufReader::new(inputstream);
    let mut progresstime = SystemTime::now();
    for line in f_buffer.lines() {
        if let Ok(input) = line {
            seqnr += 1;
            if progress && seqnr % 1000 == 1 {
                progresstime = show_progress(seqnr, progresstime, 1000);
            }
            let variants = model.find_variants(&input, max_anagram_distance, max_edit_distance, max_matches, score_threshold, stop_criterion, cache.as_mut());
            if let Some(reverseindex) = reverseindex.as_mut() {
                //we are asked to build a reverse index
                for (vocab_id,score) in variants.iter() {
                    model.add_to_reverse_index(reverseindex, &input, *vocab_id, *score);
                }
            } else if json {
                output_matches_as_json(model, &input, Some(&variants), None, output_lexmatch, seqnr);
            } else {
                //Normal output mode
                output_matches_as_tsv(model, &input, Some(&variants), None,  output_lexmatch);
            }
            if let Some(cache) = cache {
                cache.check();
            }
        }
    }
}

const MAX_BATCHSIZE: usize = 1000;

fn process_par(model: &VariantModel, inputstream: impl Read, max_anagram_distance: u8, max_edit_distance: u8, max_matches: usize, score_threshold: f64, stop_criterion: StopCriterion, output_lexmatch: bool, json: bool, progress: bool) -> io::Result<()> {
    let mut seqnr = 0;
    let f_buffer = BufReader::new(inputstream);
    let mut progresstime = SystemTime::now();
    let mut line_iter = f_buffer.lines();
    let mut eof = false;
    while !eof {
        let mut batch = vec![];
        for _ in 0..MAX_BATCHSIZE {
            if let Some(input) = line_iter.next() {
                batch.push(input?);
            } else {
                eof = true;
                break;
            }
            if batch.is_empty() {
                break;
            }
        }
        let batchsize = batch.len();
        let output: Vec<_> = batch
            .par_iter()
            .map(|input| {
                (input, model.find_variants(&input, max_anagram_distance, max_edit_distance, max_matches, score_threshold, stop_criterion, None))
            }).collect();
        for (input, variants) in output {
            seqnr += 1;
            if json {
                output_matches_as_json(model, &input, Some(&variants), None, output_lexmatch, seqnr);
            } else {
                //Normal output mode
                output_matches_as_tsv(model, &input, Some(&variants), None, output_lexmatch);
            }
        }
        if progress {
            progresstime = show_progress(seqnr, progresstime, batchsize);
        }
    }
    Ok(())
}

const MAX_BATCHSIZE_SEARCH: usize = 100;

fn process_search(model: &VariantModel, inputstream: impl Read, max_anagram_distance: u8, max_edit_distance: u8, max_matches: usize, score_threshold: f64, stop_criterion: StopCriterion, output_lexmatch: bool, json: bool, progress: bool, max_ngram: u8, newline_as_space: bool, per_line: bool) {
    let mut seqnr = 0;
    let mut prevseqnr = 0;
    let f_buffer = BufReader::new(inputstream);
    let mut progresstime = SystemTime::now();
    let mut line_iter = f_buffer.lines();
    let mut eof = false;
    while !eof {
        let mut batch = String::new();
        for i in 0..MAX_BATCHSIZE_SEARCH {
            if let Some(Ok(input)) = line_iter.next() {
                if i > 0 {
                    batch.push(if newline_as_space {
                                    ' '
                               } else {
                                    '\n'
                               });
                }
                let empty = input.is_empty();
                batch.extend(input.chars());
                if empty || per_line {
                    //an empty line is a good breakpoint for a batch
                    break;
                }
            } else {
                eof = true;
                break;
            }
            if batch.is_empty() {
                break;
            }
        }
        //parallellisation will occur inside this method:
        let output = model.find_all_matches(&batch, max_anagram_distance, max_edit_distance, max_matches, score_threshold, stop_criterion, max_ngram);
        for result_match in output {
            seqnr += 1;
            if json {
                output_matches_as_json(model, result_match.text, result_match.variants.as_ref(), Some(result_match.offset), output_lexmatch, seqnr);
            } else {
                //Normal output mode
                output_matches_as_tsv(model, result_match.text, result_match.variants.as_ref(), Some(result_match.offset), output_lexmatch);
            }
        }
        if progress {
            progresstime = show_progress(seqnr, progresstime, seqnr - prevseqnr);
        }
        prevseqnr = seqnr;
    }
}

fn show_progress(seqnr: usize, lasttime: SystemTime, batchsize: usize) -> SystemTime {
    let now = SystemTime::now();
    if lasttime >= now || seqnr <= 1 {
        eprintln!("@ {}", seqnr);
    } else {
        let elapsed = now.duration_since(lasttime).expect("clock can't go backwards").as_millis();
        let rate = (batchsize as f64) / (elapsed as f64 / 1000.0);
        eprintln!("@ {} - processing speed was {:.0} items per second", seqnr, rate);
    }
    now
}

pub fn common_arguments<'a,'b>() -> Vec<clap::Arg<'a,'b>> {
    let mut args: Vec<Arg> = Vec::new();
    args.push( Arg::with_name("lexicon")
        .long("lexicon")
        .short("l")
        .help("Lexicon against which all matches are made (may be used multiple times). The lexicon should only contain validated items, if not, use --corpus instead. The lexicon should be a tab separated file with each entry on one line, columns may be used for frequency information. This option may be used multiple times for multiple lexicons. Entries need not be single words but may also be ngrams (space separated tokens).")
        .takes_value(true)
        .number_of_values(1)
        .multiple(true)
        .required_unless("corpus"));
    args.push(Arg::with_name("corpus")
        .long("corpus")
        .short("f")
        .help("Corpus-derived lexicon/frequency list against which matches are made (may be used multiple times). Format is the same as for --lexicon. The only difference between --lexicon and --corpus is that items from corpus a lexicon loaded through --corpus is given less weight. This option may be used multiple times.")
        .takes_value(true)
        .number_of_values(1)
        .multiple(true)
        .required_unless("lexicon"));
    args.push(Arg::with_name("variants")
        .long("variants")
        .short("V")
        .help("Loads a variant list, a tab-separated file in which all items on a single line are considered variants of equal weight. This option may be used multiple times.")
        .takes_value(true)
        .number_of_values(1)
        .multiple(true));
    args.push(Arg::with_name("weighted-variants")
        .long("weighted-variants")
        .short("W")
        .help("Loads a weighted variant list, the first column contains the lexicon word and subsequent repeating columns (tab-separated) contain respectively a variant and the score of the variant. This option may be used multiple times.")
        .takes_value(true)
        .number_of_values(1)
        .multiple(true));
    args.push(Arg::with_name("errors")
        .long("errors")
        .short("E")
        .help("This is a form of --weighted-variants in which all the variants are considered erroneous forms, they will be used only to find the authoritative solution from the first column and won't be returned as solutions themselves. This option may be used multiple times.")
        .takes_value(true)
        .number_of_values(1)
        .multiple(true));
    args.push(Arg::with_name("alphabet")
        .long("alphabet")
        .short("a")
        .help("Alphabet file")
        .takes_value(true)
        .required(true));
    args.push(Arg::with_name("confusables")
        .long("confusables")
        .short("C")
        .help("Confusable list with weights. This is an optional TSV file with confusables in sesdiff-format in the first column, and weights in the second column. A weight of > 1.0 will favour a confusable over others, a weight of < 1.0 will penalize a confusable. Confusable weights should be kept close to 1.0 as they will be applied over the whole ranking score.")
        .number_of_values(1)
        .multiple(true)
        .takes_value(true));
    args.push(Arg::with_name("early-confusables")
        .long("early-confusables")
        .help("Process the confusables before pruning rather than after, may lead to more accurate results but has a performance impact")
        .required(false));
    args.push(Arg::with_name("output-lexmatch")
        .long("output-lexmatch")
        .help("Output the matching lexicon name for each variant match")
        .required(false));
    args.push(Arg::with_name("json")
        .long("json")
        .short("j")
        .help("Output json instead of tsv")
        .required(false));
    args.push(Arg::with_name("progress")
        .long("progress")
        .help("Show progress")
        .required(false));
    args.push(Arg::with_name("stop-exact")
        .short("s")
        .long("stop-exact")
        .help("Do not continue looking for variants once an exact match has been found. This significantly speeds up the process")
        .required(false));
    args.push(Arg::with_name("stop-iterative")
        .short("S")
        .long("stop-iterative")
        .help("Seek iteratively and stop after gathering enough matches, as represented by this threshold")
        .takes_value(true)
        .required(false));
    args.push(Arg::with_name("score-threshold")
        .long("score-threshold")
        .short("t")
        .help("Require scores to meet this threshold, they are pruned otherwise")
        .takes_value(true)
        .default_value("0.25")
        .required(false));
    args.push(Arg::with_name("search-cache")
        .long("search-cache")
        .help("Cache visited nodes between searches to speed up the search at the cost of increased memory. Only works for single core currently where it is enabled by default. The value corresponds to the maximum number of anagram values to cache, this should be set to a fairly high number, depending on memory availability, such as 100000. Set to 0 to disable the cache.")
        .takes_value(true)
        .default_value("100000")
        .required(false));
    args.push(Arg::with_name("single-thread")
        .long("single-thread")
        .short("1")
        .help("Run in a single thread, when running this way you can benefit from the --search-cache. If you want more than one thread but less than all available cores, set environment variable RAYON_NUM_THREADS")
        .required(false));
    args.push(Arg::with_name("interactive")
        .long("interactive")
        .short("x")
        .help("Interactive mode, basically just an alias for single-thread mode. Use this when reading from stdin one by one in a terminal.")
        .required(false));
    args.push(Arg::with_name("weight-ld")
        .long("weight-ld")
        .help("Weight attributed to Damarau-Levenshtein distance in scoring")
        .takes_value(true)
        .default_value("1.0"));
    args.push(Arg::with_name("weight-lcs")
        .long("weight-lcs")
        .help("Weight attributed to Longest common substring length in scoring")
        .takes_value(true)
        .default_value("1.0"));
    args.push(Arg::with_name("weight-prefix")
        .long("weight-prefix")
        .help("Weight attributed to longest common prefix length in scoring")
        .takes_value(true)
        .default_value("1.0"));
    args.push(Arg::with_name("weight-suffix")
        .long("weight-suffix")
        .help("Weight attributed to longest common suffix length in scoring")
        .takes_value(true)
        .default_value("1.0"));
    args.push(Arg::with_name("weight-freq")
        .long("weight-freq")
        .help("Weight attributed to frequency in scoring")
        .takes_value(true)
        .default_value("1.0"));
    args.push(Arg::with_name("weight-lex")
        .long("weight-lex")
        .help("Weight attributed to items that are in the lexicon, will always be 0 for items only in the corpus")
        .takes_value(true)
        .default_value("1.0"));
    args.push(Arg::with_name("weight-case")
        .long("weight-case")
        .help("Weight attributed to a difference in casing")
        .takes_value(true)
        .default_value("0.2"));
    args.push(Arg::with_name("max-anagram-distance")
        .long("max-anagram-distance")
        .short("k")
        .help("Maximum anagram distance. This impacts the size of the search space")
        .takes_value(true)
        .default_value("3"));
    args.push(Arg::with_name("max-edit-distance")
        .long("max-edit-distance")
        .short("d")
        .help("Maximum edit distance (levenshtein)")
        .takes_value(true)
        .default_value("3"));
    args.push(Arg::with_name("max-matches")
        .long("max-matches")
        .short("n")
        .help("Number of matches the return per input (set to 0 for unlimited if you want to exhaustively return every possibility within the specified edit distance)")
        .takes_value(true)
        .default_value("10"));
    args.push(Arg::with_name("files")
        .help("Input files")
        .takes_value(true)
        .multiple(true)
        .required(false));
    args
}


fn main() {
    let rootargs = App::new("Analiticcl")
                    .version("0.3.0")
                    .author("Maarten van Gompel (proycon) <proycon@anaproy.nl>")
                    .about("Spelling variant matching / approximate string matching / fuzzy search")
                    .subcommand(
                        SubCommand::with_name("query")
                            .about("Query the model; find all matches in the lexicon of the variants provided in the input, one entry to match per line.")
                            .args(&common_arguments())
                    )
                    .subcommand(
                        SubCommand::with_name("index")
                            .about("Compute and output the anagram index")
                            .args(&common_arguments())
                    )
                    .subcommand(
                        SubCommand::with_name("search")
                            .about("Search entire text input and find and output all possible matches")
                            .args(&common_arguments())
                            .arg(Arg::with_name("per-line")
                                .long("per-line")
                                .help("Will process per line; assumes each line holds a complete unit (e.g. sentence or paragraph) and that n-grams never cross line boundaires"))
                            .arg(Arg::with_name("retain-linebreaks")
                                .long("retain-linebreaks")
                                .help("Retain linebreaks (newline), the default is to treat them as if they were spaces. Retaining them assumes you have a newline as part of your alphabet."))
                            .arg(Arg::with_name("max-ngram-order")
                                .long("max-ngram-order")
                                .short("N")
                                .help("Maximum ngram order (1 for unigrams, 2 for bigrams, etc..). This also requires you to load actual ngram frequency lists using --corpus to have any effect.")
                                .takes_value(true)
                                .default_value("1"))
                            .arg(Arg::with_name("lm")
                                .long("lm")
                                .help("Corpus-derived list of unigrams and bigrams that are used for simple language modelling, i.e. computation the transition probabilities when finding the optimal sequence of variants. This is a TSV file containing the the ngram in the first column (space character acts as token separator), and the absolute frequency count in the second column. It is also recommended it contains the special tokens <bos> (begin of sentence) and <eos> end of sentence. The items in this list are NOT used for variant matching, use --corpus or even --lexicon instead if you want to also match against these items.")
                                .takes_value(true)
                                .number_of_values(1)
                                .multiple(true))
                    )
                    /*.subcommand(
                        SubCommand::with_name("collect")
                            .about("Collect variants from the input data, grouping them for items in the lexicon. Note that this forces single-core mode for now.")
                            .args(&common_arguments())
                    )*/
                    .arg(Arg::with_name("debug")
                        .long("debug")
                        .short("D")
                        .help("Debug")
                        .required(false))
                    .get_matches();

    eprintln!("Initializing model...");

    let args = if let Some(args) = rootargs.subcommand_matches("query") {
        args
    } else if let Some(args) = rootargs.subcommand_matches("collect") {
        args
    } else if let Some(args) = rootargs.subcommand_matches("index") {
        args
    } else if let Some(args) = rootargs.subcommand_matches("search") {
        args
    } else {
       panic!("No command specified");
    };

    let weights = Weights {
        ld: args.value_of("weight-ld").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        lcs: args.value_of("weight-lcs").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        prefix: args.value_of("weight-prefix").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        suffix: args.value_of("weight-suffix").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        freq: args.value_of("weight-freq").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        lex: args.value_of("weight-lex").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
        case: args.value_of("weight-case").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
    };

    let mut cache = if let Some(visited_max_size) = args.value_of("search-cache") {
        let visited_max_size = visited_max_size.parse::<usize>().expect("Cache size should be a large integer");
        if visited_max_size > 0 {
            Some(Cache::new(visited_max_size))
        } else {
            None
        }
    } else {
        None
    };

    let mut model = VariantModel::new(
        args.value_of("alphabet").unwrap(),
        weights,
        rootargs.is_present("debug")
    );

    eprintln!("Loading lexicons...");

    if args.is_present("lexicon") {
        for filename in args.values_of("lexicon").unwrap().collect::<Vec<&str>>() {
            model.read_vocabulary(filename, &VocabParams::default()).expect(&format!("Error reading lexicon {}", filename));
        }
    }

    if args.is_present("corpus") {
        for filename in args.values_of("corpus").unwrap().collect::<Vec<&str>>() {
            model.read_vocabulary(filename, &VocabParams {
                weight: 0.0,
                ..Default::default()
            }).expect(&format!("Error reading corpus lexicon {}", filename));
        }
    }

    if args.is_present("lm") {
        for filename in args.values_of("lm").unwrap().collect::<Vec<&str>>() {
            model.read_vocabulary(filename, &VocabParams {
                weight: 0.0,
                vocab_type: VocabType::NoIndex,
                ..Default::default()
            }).expect(&format!("Error reading lm {}", filename));
        }
    }

    if args.is_present("variants") {
        for filename in args.values_of("variants").unwrap().collect::<Vec<&str>>() {
            model.read_variants(filename, Some(&VocabParams::default())).expect(&format!("Error reading variant list {}", filename));
        }
    }

    if args.is_present("weighted-variants") {
        for filename in args.values_of("weighted-variants").unwrap().collect::<Vec<&str>>() {
            model.read_weighted_variants(filename, Some(&VocabParams::default()), false).expect(&format!("Error reading weighted variant list {}", filename));
        }
    }

    if args.is_present("errors") {
        for filename in args.values_of("errors").unwrap().collect::<Vec<&str>>() {
            model.read_weighted_variants(filename, Some(&VocabParams::default()), true).expect(&format!("Error reading error list {}", filename));
        }
    }

    if args.is_present("confusables") {
        eprintln!("Loading confusable lists...");
        for filename in args.values_of("confusables").unwrap().collect::<Vec<&str>>() {
            model.read_confusablelist(filename).expect(&format!("Error reading confusable list {}", filename));
        }
    }

    eprintln!("Building model...");
    model.build();

    let max_anagram_distance: u8 = args.value_of("max-anagram-distance").unwrap().parse::<u8>().expect("Anagram distance should be an integer between 0 and 255");
    let max_edit_distance: u8 = args.value_of("max-edit-distance").unwrap().parse::<u8>().expect("Anagram distance should be an integer between 0 and 255");
    let max_matches: usize = args.value_of("max-matches").unwrap().parse::<usize>().expect("Maximum matches should should be an integer (0 for unlimited)");
    let score_threshold: f64 = args.value_of("score-threshold").unwrap().parse::<f64>().expect("Score threshold should be a floating point number");
    let output_lexmatch = args.is_present("output-lexmatch");
    let progress = args.is_present("progress");
    let stop_criterion = match (args.is_present("stop-exact"), args.is_present("stop-iterative")) {
        (true, true) => StopCriterion::IterativeStopAtExactMatch(args.value_of("stop-iterative").unwrap().parse::<usize>().expect("Cut-off value should be an integer")),
        (false, true) => StopCriterion::Iterative(args.value_of("stop-iterative").unwrap().parse::<usize>().expect("Stop-iterative threshold should be an integer")),
        (true, false) => StopCriterion::StopAtExactMatch,
        (false, false) => StopCriterion::Exhaustive
    };
    let json = args.is_present("json");
    let singlethread = args.is_present("single-thread") || args.is_present("debug") || args.is_present("interactive");

    //settings for Search mode
    let perline = args.is_present("per-line");
    let retain_linebreaks = args.is_present("retain-linebreaks");
    let max_ngram = if let Some(value) = args.value_of("max-ngram-order") {
        value.parse::<u8>().expect("Score threshold should be a small integer")
    } else {
        0
    };



    if args.is_present("early-confusables") {
        model.set_confusables_before_pruning();
    }


    if rootargs.subcommand_matches("index").is_some() {
        eprintln!("Computing and outputting anagram index...");
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
        //query or collect

        if rootargs.subcommand_matches("query").is_some() {
            eprintln!("Querying the model...");
        } else if rootargs.subcommand_matches("search").is_some() {
            eprintln!("Finding all variants in the input text...");
        } else {
            eprintln!("Collecting variants...");
        }
        let mut reverseindex = if rootargs.subcommand_matches("collect").is_some() {
            Some(HashMap::new())
        } else {
            None
        };

        if json && reverseindex.is_none() {
            println!("[");
        }

        let files: Vec<_> = if args.is_present("files") {
            args.values_of("files").unwrap().collect()
        } else {
            vec!("-")
        };
        for filename in files {
            match filename {
                "-" | "STDIN" | "stdin"  => {
                    let stdin = io::stdin();
                    if rootargs.subcommand_matches("search").is_some() {
                        eprintln!("(accepting standard input; enter text to search for variants, output may be delayed until end of input, enter an empty line to force output earlier)");
                        process_search(&model, stdin, max_anagram_distance, max_edit_distance, max_matches, score_threshold, stop_criterion, output_lexmatch, json, progress, max_ngram, !retain_linebreaks, perline);
                    } else if singlethread || reverseindex.is_some()  {
                        eprintln!("(accepting standard input; enter input to match, one per line)");
                        process(&model, stdin, &mut reverseindex, max_anagram_distance, max_edit_distance, max_matches, score_threshold, stop_criterion, output_lexmatch, json, &mut cache, progress);
                    } else {
                        eprintln!("(accepting standard input; enter input to match, one per line, output may be delayed until end of input due to parallellisation)");
                        //normal parallel behaviour
                        process_par(&model, stdin, max_anagram_distance, max_edit_distance, max_matches, score_threshold, stop_criterion, output_lexmatch, json, progress).expect("I/O Error");
                    }
                },
                _ =>  {
                    let f = File::open(filename).expect(format!("ERROR: Unable to open file {}", filename).as_str());
                    if rootargs.subcommand_matches("search").is_some() {
                        process_search(&model, f, max_anagram_distance, max_edit_distance, max_matches, score_threshold, stop_criterion, output_lexmatch, json, progress, max_ngram, !retain_linebreaks, perline);
                    } else if singlethread || reverseindex.is_some() {
                        process(&model, f, &mut reverseindex, max_anagram_distance, max_edit_distance, max_matches, score_threshold, stop_criterion, output_lexmatch, json, &mut cache, progress);
                    } else {
                        //normal parallel behaviour
                        process_par(&model, f, max_anagram_distance, max_edit_distance, max_matches, score_threshold, stop_criterion, output_lexmatch, json, progress).expect("I/O Error");
                    }
                }
            }
        }

        if json && reverseindex.is_none() {
            println!("]");
        }

        if let Some(reverseindex) = reverseindex {
            eprintln!("Outputting collected variants...");
            output_reverse_index(&model, &reverseindex);
        }
    }
}
