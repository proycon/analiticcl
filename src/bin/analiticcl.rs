extern crate clap;
extern crate rayon;

use std::fs::File;
use std::io::{self, BufReader,BufRead,Read,Write};
use clap::{Arg, App, SubCommand};
use std::collections::HashMap;
use std::time::SystemTime;
use std::process::exit;
use rayon::prelude::*;


use analiticcl::*;

#[derive(Debug)]
enum Resource<'a> {
    Lexicon(&'a str),
    VariantList(&'a str),
    ErrorList(&'a str)
}

fn output_matches_as_tsv(model: &VariantModel, input: &str, variants: Option<&Vec<VariantResult>>, selected: Option<usize>, offset: Option<Offset>, output_lexmatch: bool, freq_weight: f32) {
    print!("{}",input);
    if let Some(offset) = offset {
        print!("\t{}:{}",offset.begin, offset.end);
    }
    if let Some(variants) = variants {
        if let Some(selected) = selected {
            //output selected value before all others
            if let Some(result) = variants.get(selected) {
                output_result_as_tsv(&model, &result, output_lexmatch, freq_weight);
            }
        }
        for (i, result) in variants.iter().enumerate() {
            if selected.is_none() || selected.unwrap() != i { //output all others
                output_result_as_tsv(&model, &result, output_lexmatch, freq_weight);
            }
        }
    }
    println!();
}

fn output_result_as_tsv(model: &VariantModel, result: &VariantResult, output_lexmatch: bool, freq_weight: f32) {
    let vocabvalue = model.get_vocab(result.vocab_id).expect("getting vocab by id");
    print!("\t{}\t{}\t", vocabvalue.text, result.score(freq_weight));
    if  output_lexmatch {
        let lexicons: Vec<&str> = model.lexicons.iter().enumerate().filter_map(|(i,name)| {
            if vocabvalue.in_lexicon(i as u8) {
                Some(name.as_str())
            } else {
                None
            }
        }).collect();
        print!("\t\"{}\"", lexicons.join(";"));
    }
}

fn output_matches_as_json(model: &VariantModel, input: &str, variants: Option<&Vec<VariantResult>>, selected: Option<usize>, offset: Option<Offset>, output_lexmatch: bool, freq_weight: f32, seqnr: usize) {
    if seqnr > 1 {
        print!("    ,")
    } else {
        print!("    ")
    }
    print!("{{ \"input\": \"{}\"", input.replace("\"","\\\"").as_str());
    if let Some(offset) = offset {
        print!(", \"begin\": {}, \"end\": {}", offset.begin, offset.end);
    }
    if let Some(variants) = variants {
        println!(", \"variants\": [ ");
        let mut wroteoutput = false;
        if let Some(selected) = selected {
            if let Some(result) = variants.get(selected) {
                if wroteoutput {
                    println!(",");
                }
                output_result_as_json(&model, &result, output_lexmatch, freq_weight);
                wroteoutput = true;
            }
        }
        for (i, result) in variants.iter().enumerate() {
            if selected.is_none() || selected.unwrap() != i { //output all others
                if wroteoutput {
                    println!(",");
                }
                output_result_as_json(&model, &result, output_lexmatch, freq_weight);
                wroteoutput = true;
            }
        }
        println!("");
        println!("    ] }}");
    } else {
        println!(" }}");
    }
}

fn output_result_as_json(model: &VariantModel, result: &VariantResult, output_lexmatch: bool, freq_weight: f32) {
    let vocabvalue = model.get_vocab(result.vocab_id).expect("getting vocab by id");
    print!("        {{ \"text\": \"{}\", \"score\": {}", vocabvalue.text.replace("\"","\\\""), result.score(freq_weight));
    print!(", \"dist_score\": {}", result.dist_score);
    print!(", \"freq_score\": {}", result.freq_score);
    if let Some(via_id) = result.via {
        let viavalue = model.get_vocab(via_id).expect("getting vocab by id");
        print!(", \"via\": \"{}\"", viavalue.text.replace("\"","\\\""));
    }
    if  output_lexmatch {
        let lexicons: Vec<String> = model.lexicons.iter().enumerate().filter_map(|(i,name)| {
            if vocabvalue.in_lexicon(i as u8) {
                Some(format!("\"{}\"", name.replace("\"","\\\"")))
            } else {
                None
            }
        }).collect();
        print!(", \"lexicons\": [ {} ]", lexicons.join(", "));
    }
    print!(" }}");
}


///auxiliary function outputting a single variant
fn output_weighted_variant_as_tsv(text: &str, score: f64, freq: u32, lexindex: u32, multioutput: bool, outfiles: &mut HashMap<u8,File>, model: &VariantModel) {
    if multioutput {
        for lexindex in model.lexicons.iter().enumerate().filter_map(|(i,_name)| {
            if lexindex as usize & (1 << i) == i << i {
                Some(i)
            } else {
                None
            }
        }) {
            let lexindex = lexindex as u8;
            let f = if let Some(f) = outfiles.get_mut(&lexindex) {
                f
            } else {
                let filename: String = format!("{}.variants.tsv", model.lexicons.get(lexindex as usize).expect("lexindex must exist"));
                if let Ok(f) = File::create(filename.as_str()) {
                    outfiles.insert(lexindex, f);
                    outfiles.get_mut(&lexindex).expect("outfile must be prepared")
                } else {
                    panic!("unable to write to {}", filename.as_str());
                }
            };
            f.write(format!("\t{}\t{}\t{}\n", text, score, freq).as_bytes()).expect("error writing to file");
        }
    } else {
        print!("\t{}\t{}", text, score);
    }
}


/// Outputs weighted variants stored in the model as tsv
fn output_weighted_variants_as_tsv(model: &VariantModel, multioutput: bool) {
    let mut outfiles: HashMap<u8,File> = HashMap::new();
    let mut first;
    for vocabitem in model.decoder.iter() {
        if let Some(variants) = &vocabitem.variants {
            first = true;
            for variant in variants {
                if let VariantReference::ReferenceFor((vocab_id, score)) = variant {
                    if first {
                        print!("{}", vocabitem.text);
                        first = false;
                    }
                    let variantitem = model.decoder.get(*vocab_id as usize).expect("vocab id must exist");
                    output_weighted_variant_as_tsv(&variantitem.text, *score, variantitem.frequency, variantitem.lexindex, multioutput, &mut outfiles, model);
                }
            }
            if !first {
                println!();
            }
        }
    }
}

///auxiliary function outputting a single variant
fn output_weighted_variant_as_json(text: &str, score: f64, freq: u32, lexindex: u32, multioutput: bool, outfiles: &mut HashMap<u8,File>, model: &VariantModel) {
    if multioutput {
        for lexindex in model.lexicons.iter().enumerate().filter_map(|(i,_name)| {
            if lexindex as usize & (1 << i) == 1 << i {
                Some(i)
            } else {
                None
            }
        }) {
            let lexindex = lexindex as u8;
            let f = if let Some(f) = outfiles.get_mut(&lexindex) {
                f
            } else {
                let filename: String = format!("{}.variants.json", model.lexicons.get(lexindex as usize).expect("lexindex must exist"));
                if let Ok(f) = File::create(filename.as_str()) {
                    outfiles.insert(lexindex, f);
                    outfiles.get_mut(&lexindex).expect("outfile must be prepared")
                } else {
                    panic!("unable to write to {}", filename.as_str());
                }
            };
            f.write(format!("        {{ \"text\": \"{}\",  \"score\": {}, \"freq\": {} }}, ", text.replace("\"","\\\""), freq, score).as_bytes()).expect("error writing to file");
        }
    } else {
        println!("        {{ \"text\": \"{}\", \"score\": {}, \"freq\": {} }}, ", text.replace("\"","\\\""), score, freq);
    }
}

/// Outputs weighted variants stored in the model as tsv
fn output_weighted_variants_as_json(model: &VariantModel, multioutput: bool) {
    let mut outfiles: HashMap<u8,File> = HashMap::new();
    let mut first;
    println!("{{");
    for vocabitem in model.decoder.iter() {
        first = true;
        if let Some(variants) = &vocabitem.variants {
            for variant in variants {
                if let VariantReference::ReferenceFor((vocab_id, score)) = variant {
                    if first {
                        println!("    \"{}\": [ ", vocabitem.text.replace("\"","\\\"").as_str());
                        first = false;
                    }
                    let variantitem = model.decoder.get(*vocab_id as usize).expect("vocab id must exist");
                    output_weighted_variant_as_json(&variantitem.text, *score, variantitem.frequency, variantitem.lexindex, multioutput, &mut outfiles, model);
                }
            }
        }
        if !first {
            println!("    ]");
        }
    }
    println!("}}")
}

fn process(model: &VariantModel, inputstream: impl Read, searchparams: &SearchParameters, output_lexmatch: bool, json: bool, progress: bool) {
    let mut seqnr = 0;
    let f_buffer = BufReader::new(inputstream);
    let mut progresstime = SystemTime::now();
    for line in f_buffer.lines() {
        if let Ok(input) = line {
            seqnr += 1;
            if progress && seqnr % 1000 == 1 {
                progresstime = show_progress(seqnr, progresstime, 1000);
            }
            let variants = model.find_variants(&input, searchparams);
            if json {
                output_matches_as_json(model, &input, Some(&variants), Some(0), None, output_lexmatch, searchparams.freq_weight, seqnr);
            } else {
                //Normal output mode
                output_matches_as_tsv(model, &input, Some(&variants), Some(0), None,  output_lexmatch, searchparams.freq_weight);
            }
        }
    }
}

const MAX_BATCHSIZE: usize = 1000;

fn process_par(model: &VariantModel, inputstream: impl Read, searchparams: &SearchParameters, output_lexmatch: bool, json: bool, progress: bool) -> io::Result<()> {
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
                (input, model.find_variants(&input, searchparams))
            }).collect();
        for (input, variants) in output {
            seqnr += 1;
            if json {
                output_matches_as_json(model, &input, Some(&variants), Some(0), None, output_lexmatch, searchparams.freq_weight, seqnr);
            } else {
                //Normal output mode
                output_matches_as_tsv(model, &input, Some(&variants), Some(0), None, output_lexmatch, searchparams.freq_weight);
            }
        }
        if progress {
            progresstime = show_progress(seqnr, progresstime, batchsize);
        }
    }
    Ok(())
}

fn process_learn(model: &mut VariantModel, inputstream: impl Read, searchparams: &SearchParameters, iterations: u8, json: bool, multioutput: bool, strict: bool, newline_as_space: bool, per_line: bool) -> io::Result<()> {
    let f_buffer = BufReader::new(inputstream);
    let mut line_iter = f_buffer.lines();
    let mut lines = vec![]; //load all lines in memory
    while let Some(Ok(input)) = line_iter.next() {
        lines.push(input);
    }
    if strict {
        //batch for learning in strict mode simply contains all input data at once
        let batch_size = lines.len();
        for i in 0..iterations {
            let count = model.learn_variants(&lines, searchparams, strict, true);
            eprintln!("(Iteration #{}: learned {} variants (out of a total of {} input strings)", i+1, count, batch_size);
            if count == 0 && i+1 < iterations {
                eprintln!("(Halting further iterations)");
                break;
            }
        }
    } else {
        for i in 0..iterations {
            let mut eof = false;
            let mut line_iter = lines.iter();
            while !eof {
                let mut batch = String::new();
                for j in 0..MAX_BATCHSIZE_SEARCH {
                    if let Some(input) = line_iter.next() {
                        if j > 0 {
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
            }
            let count = model.learn_variants(&lines, searchparams, strict, true);
            eprintln!("(Iteration #{}: learned {} variants", i+1, count);
            if count == 0 && i+1 < iterations {
                eprintln!("(Halting further iterations)");
                break;
            }
        }
    }
    if json {
        output_weighted_variants_as_json(model, multioutput);
    } else {
        output_weighted_variants_as_tsv(model, multioutput);
    }
    Ok(())
}

const MAX_BATCHSIZE_SEARCH: usize = 100;

fn process_search(model: &VariantModel, inputstream: impl Read, searchparams: &SearchParameters, output_lexmatch: bool, json: bool, progress: bool, newline_as_space: bool, per_line: bool) {
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
        let output = model.find_all_matches(&batch, searchparams);
        if seqnr > 0 && !output.is_empty() {
            println!();
        }
        for result_match in output {
            seqnr += 1;
            if json {
                output_matches_as_json(model, result_match.text, result_match.variants.as_ref(), result_match.selected, Some(result_match.offset), output_lexmatch, searchparams.freq_weight, seqnr);
            } else {
                //Normal output mode
                output_matches_as_tsv(model, result_match.text, result_match.variants.as_ref(), result_match.selected, Some(result_match.offset), output_lexmatch, searchparams.freq_weight);
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
        .help("Lexicon against which all matches are made (may be used multiple times). The lexicon should be a tab separated file with each entry on one line, columns may be used for frequency information. This option may be used multiple times for multiple lexicons. Entries need not be single words but may also be ngrams (space separated tokens).")
        .takes_value(true)
        .number_of_values(1)
        .multiple(true)
        .required_unless("variants"));
    args.push(Arg::with_name("variants")
        .long("variants")
        .short("V")
        .help("Loads a (weighted) variant list, the first column contains the lexicon word and subsequent repeating columns (tab-separated) contain respectively a variant and the score of the variant. This option may be used multiple times.")
        .takes_value(true)
        .number_of_values(1)
        .multiple(true));
    args.push(Arg::with_name("errors")
        .long("errors")
        .short("E")
        .help("This is a form of --variants in which all the variants are considered erroneous forms, they will be used only to find the authoritative solution from the first column and won't be returned as solutions themselves (i.e. they are transparent). This option may be used multiple times.")
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
        .help("Do not continue looking for variants once an exact match has been found. This significantly speeds up the process.")
        .takes_value(false)
        .required(false));
    args.push(Arg::with_name("score-threshold")
        .long("score-threshold")
        .short("t")
        .help("Require variant scores to meet this threshold, they are pruned otherwise. This is an absolute score threshold. It will be applied prior to score reweighing against confusible lists.")
        .takes_value(true)
        .default_value("0.25")
        .required(false));
    args.push(Arg::with_name("cutoff-threshold")
        .long("cutoff-threshold")
        .short("T")
        .help("If a score in variant ranking is this factor worse than the best score, the ranking is cut off at this point and this score and all lower ones are pruned. This is a relative score threshold. Value must be equal or greater than one, or 0 to disable. It will be applied after score reweighing against confusible lists.")
        .takes_value(true)
        .default_value("2.0")
        .required(false));
    args.push(Arg::with_name("freq-ranking")
        .short("F")
        .long("freq-ranking")
        .help("Consider frequency information and not just similarity scores when ranking variant candidates. The actual ranking will be a weighted combination between the similarity score and the frequency score. The value for this parameter is the weight you want to attribute to the frequency component in ranking, in relation to similarity. (a value between 0 and 1.0). Note that even if this parameter is not set, frequency information will always be used to break ties in case of similarity score")
        .takes_value(true));
    args.push(Arg::with_name("single-thread")
        .long("single-thread")
        .short("1")
        .help("Run in a single thread, If you want more than one thread but less than all available cores, set environment variable RAYON_NUM_THREADS instead")
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
        .default_value("0.5"));
    args.push(Arg::with_name("weight-lcs")
        .long("weight-lcs")
        .help("Weight attributed to Longest common substring length in scoring")
        .takes_value(true)
        .default_value("0.125"));
    args.push(Arg::with_name("weight-prefix")
        .long("weight-prefix")
        .help("Weight attributed to longest common prefix length in scoring")
        .takes_value(true)
        .default_value("0.125"));
    args.push(Arg::with_name("weight-suffix")
        .long("weight-suffix")
        .help("Weight attributed to longest common suffix length in scoring")
        .takes_value(true)
        .default_value("0.125"));
    /*args.push(Arg::with_name("weight-freq")
        .long("weight-freq")
        .help("Weight attributed to frequency in scoring")
        .takes_value(true)
        .default_value("1.0"));*/
    args.push(Arg::with_name("weight-case")
        .long("weight-case")
        .help("Weight attributed to a difference in casing")
        .takes_value(true)
        .default_value("0.125"));
    args.push(Arg::with_name("max-anagram-distance")
        .long("max-anagram-distance")
        .short("k")
        .help("Maximum anagram distance. Can either be an absolute value (integer), or a ratio of the input length (float between 0.0 and 1.0), or a combination of a ratio with an absolute maximum, separated by a semicolon (ratio;limit). The anagram distance impacts the size of the search space. Each insertion or deletion has cost 1, substitutions can not be separately tracked so they counts as 2 (deletion+insertion). It is therefore recommended to set this value slightly higher than the max edit distance.")
        .takes_value(true)
        .default_value("3"));
    args.push(Arg::with_name("max-edit-distance")
        .long("max-edit-distance")
        .short("d")
        .help("Maximum edit distance (levenshtein-damerau). The maximum edit distance according to Levenshtein-Damarau. Can either be an absolute value (integer), or a ratio of the input length (float between 0.0 and 1.0), or a combination of a ratio with an absolute maximum, separated by a semicolon (ratio;limit). When a ratio is expressed, longer inputs use a higher edit distance than shorter ones. Insertions, deletions, substitutions and transposition all have the same cost (1). It is recommended to set this value slightly lower than the maximum anagram distance.")
        .takes_value(true)
        .default_value("2"));
    args.push(Arg::with_name("max-matches")
        .long("max-matches")
        .short("n")
        .help("Number of matches to return per input (set to 0 for unlimited if you want to exhaustively return every possibility within the specified anagram and edit distance)")
        .takes_value(true)
        .default_value("10"));
    args.push(Arg::with_name("files")
        .help("Input files")
        .takes_value(true)
        .multiple(true)
        .required(false));
    args
}

pub fn search_arguments<'a,'b>() -> Vec<clap::Arg<'a,'b>> {
    let mut args: Vec<Arg> = Vec::new();
        args.push(Arg::with_name("per-line")
            .long("per-line")
            .help("Will process per line; assumes each line holds a complete unit (e.g. sentence or paragraph) and that n-grams never cross line boundaires"));
        args.push(Arg::with_name("retain-linebreaks")
            .long("retain-linebreaks")
            .help("Retain linebreaks (newline), the default is to treat them as if they were spaces. Retaining them assumes you have a newline as part of your alphabet."));
        args.push(Arg::with_name("max-ngram-order")
            .long("max-ngram-order")
            .short("N")
            .help("Maximum ngram order for variant lookup (1 for unigrams, 2 for bigrams, etc..)")
            .takes_value(true)
            .default_value("3"));
        args.push(Arg::with_name("max-seq")
            .long("max-seq")
            .short("Q")
            .help("Maximum number of candidate sequences to take along to the language modelling stage")
            .takes_value(true)
            .default_value("250"));
        args.push(Arg::with_name("lm")
            .long("lm")
            .help("Language model, a corpus-derived list of n-grams with absolute frequency counts. This is a TSV file containing the the ngram in the first column (space character acts as token separator), and the absolute frequency count in the second column. It is also recommended it contains the special tokens <bos> (begin of sentence) and <eos> end of sentence. The items in this list are NOT used for variant matching, use --corpus or even --lexicon instead if you want to also match against these items. Conversely, files provides through --lexicon and --corpus and other options are NOT used for language modelling.")
            .takes_value(true)
            .number_of_values(1)
            .multiple(true));
        args.push(Arg::with_name("lm-order")
            .long("lm-order")
            .short("L")
            .help("N-gram order for Language models (2 for bigrams, 3 for trigrams, etc..)")
            .takes_value(true)
            .default_value("3"));
        args.push(Arg::with_name("weight-lm")
            .long("weight-lm")
            .help("Weight attributed to the language model in finding the most likely sequence in search mode")
            .takes_value(true)
            .default_value("1.0"));
        args.push(Arg::with_name("weight-variant-model")
            .long("weight-variant-model")
            .help("Weight attributed to the variant model in finding the most likely sequence in search mode")
            .takes_value(true)
            .default_value("1.0"));
        args.push(Arg::with_name("weight-context")
            .long("weight-context")
            .help("For rescoring against input context using a language model: weight attributed to the language model in relation to the variant model. (0=disabled, default, 1.0=equal weight, 0.5=half as strong as the variant model). Setting this forces consideration of input context in an earlier stage. Only relevant for search mode.")
            .takes_value(true)
            .default_value("0.0"));
        args.push(Arg::with_name("allow-overlap")
            .long("allow-overlap")
            .help("Do not consolidate multiple matches by finding a most likely sequence, but simply return all matches as-is, even if they overlap.")
            .takes_value(false));
    args
}

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let rootargs = App::new("Analiticcl")
                    .version(VERSION)
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
                            .args(&search_arguments())
                    )
                    .subcommand(
                        SubCommand::with_name("learn")
                            .about("Learn variants from the input data. Outputs a (weighted) variant list.")
                            .args(&common_arguments())
                            .arg(Arg::with_name("iterations")
                                .short("I")
                                .long("iterations")
                                .help("The number of iterations to use for learning, more iterations means more edit distance can be covered and more words will be tied to something, but the accuracy may suffer as the iterations go up.")
                                .takes_value(true)
                                .default_value("1"))
                            .arg(Arg::with_name("multi-output")
                                .short("O")
                                .long("multi-output")
                                .help("Output to multiple (weighted) variant lists rather than to standard output, each variant lists corresponds to an input lexicon. This allows keeping the link with the original lexicon."))
                            .arg(Arg::with_name("strict")
                                .long("strict")
                                .help("Strict learning: the input is to learn from is itself a list or lexicon, one item per line. This offers a more controlled form of learning that produces better results."))
                            .args(&search_arguments())
                    )
                    .arg(Arg::with_name("debug")
                        .long("debug")
                        .short("D")
                        .help("Set debug level, can be set in range 0-4")
                        .takes_value(true)
                        .required(false))
                    .get_matches();

    eprintln!("Initializing model...");

    let args = if let Some(args) = rootargs.subcommand_matches("query") {
        args
    } else if let Some(args) = rootargs.subcommand_matches("learn") {
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
        case: args.value_of("weight-case").unwrap().parse::<f64>().expect("Weights should be a floating point value"),
    };


    let mut model = VariantModel::new(
        args.value_of("alphabet").unwrap(),
        weights,
        rootargs.value_of("debug").unwrap_or("0").parse::<u8>().expect("Debug level should be integer in range 0-4")
    );


    eprintln!("Loading lexicons...");

    //Gathering everything to load, in the exact order specified
    let mut resources: Vec<(usize, Resource)> = Vec::new();

    if args.is_present("lexicon") {
        let lexicons = args.values_of("lexicon").unwrap().collect::<Vec<&str>>();
        let lexicon_indices = args.indices_of("lexicon").unwrap().collect::<Vec<usize>>();
        for (filename, index) in lexicons.iter().zip(lexicon_indices) {
            resources.push((index, Resource::Lexicon(filename)));
        }
    }
    if args.is_present("variants") {
        let variantlists = args.values_of("variants").unwrap().collect::<Vec<&str>>();
        let variantlist_indices = args.indices_of("variants").unwrap().collect::<Vec<usize>>();
        for (filename, index) in variantlists.iter().zip(variantlist_indices) {
            resources.push((index, Resource::VariantList(filename)));
        }
    }

    if args.is_present("errors") {
        let errorlists = args.values_of("errors").unwrap().collect::<Vec<&str>>();
        let errorlist_indices = args.indices_of("errors").unwrap().collect::<Vec<usize>>();
        for (filename, index) in errorlists.iter().zip(errorlist_indices) {
            resources.push((index, Resource::ErrorList(filename)));
        }
    }

    //sort by index
    resources.sort_by_key(|x| x.0);

    for (_, resource) in resources {
        match resource {
            Resource::Lexicon(filename) => model.read_vocabulary(filename, &VocabParams::default()).expect(&format!("Error reading lexicon {}", filename)),
            Resource::VariantList(filename) => model.read_variants(filename, Some(&VocabParams::default()), false).expect(&format!("Error reading weighted variant list {}", filename)),
            Resource::ErrorList(filename) => model.read_variants(filename, Some(&VocabParams::default()), true).expect(&format!("Error reading weighted variant list {}", filename)),
        }
    }

    if args.is_present("lm") {
        for filename in args.values_of("lm").unwrap().collect::<Vec<&str>>() {
            model.read_vocabulary(filename, &VocabParams {
                vocab_type: VocabType::LM,
                ..Default::default()
            }).expect(&format!("Error reading lm {}", filename));
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

    let output_lexmatch = args.is_present("output-lexmatch");
    let progress = args.is_present("progress");
    let json = args.is_present("json");

    //settings for Search mode
    let perline = args.is_present("per-line");
    let retain_linebreaks = args.is_present("retain-linebreaks");

    let searchparams = SearchParameters {
        max_anagram_distance: args.value_of("max-anagram-distance").unwrap().parse::<DistanceThreshold>().expect("Anagram distance should be an integer between 0 and 255 (absolute) or a float between 0 and 1 (ratio)"),
        max_edit_distance: args.value_of("max-edit-distance").unwrap().parse::<DistanceThreshold>().expect("Anagram distance should be an integer between 0 and 255 (absolute) or a float between 0 and 1 (ratio)"),
        max_matches: args.value_of("max-matches").unwrap().parse::<usize>().expect("Maximum matches should should be an integer (0 for unlimited)"),
        score_threshold: args.value_of("score-threshold").unwrap().parse::<f64>().expect("Score threshold should be a floating point number"),
        cutoff_threshold: args.value_of("cutoff-threshold").unwrap().parse::<f64>().expect("Cutoff threshold should be a floating point number"),
        stop_criterion: if args.is_present("stop-exact") {
            StopCriterion::StopAtExactMatch
        } else {
            StopCriterion::Exhaustive
        },
        single_thread: args.is_present("single-thread") || args.is_present("debug") || args.is_present("interactive"),
        consolidate_matches: !args.is_present("allow-overlap"),
        max_ngram: if let Some(value) = args.value_of("max-ngram-order") {
            value.parse::<u8>().expect("Max n-gram should be a small integer")
        } else {
            1
        },
        freq_weight: if args.is_present("freq-ranking") {
            args.value_of("freq-ranking").unwrap().parse::<f32>().expect("Frequency weight for frequency ranking should be a floating point number (between 0 and 1)")
        } else {
            0.0
        },
        lm_order: if let Some(value) = args.value_of("lm-order") {
            value.parse::<u8>().expect("LM order should be a small integer")
        } else {
            1
        },
        lm_weight: if args.is_present("weight-lm") {
            args.value_of("weight-lm").unwrap().parse::<f32>().expect("Language model weight should be a floating point number")
        } else {
            1.0
        },
        variantmodel_weight: if args.is_present("weight-variant-model") {
            args.value_of("weight-variant-model").unwrap().parse::<f32>().expect("Variant model weight should be a floating point number")
        } else {
            1.0
        },
        context_weight: if args.is_present("weight-context") {
            args.value_of("weight-context").unwrap().parse::<f32>().expect("Context weight should be a floating point number")
        } else {
            1.0
        },
        max_seq: if args.is_present("max-seq") {
            args.value_of("max-seq").unwrap().parse::<usize>().expect("max-seq must be an integer")
        } else {
            250
        },
    };


    if searchparams.cutoff_threshold < 1.0 && searchparams.cutoff_threshold != 0.0  {
        eprintln!("ERROR: Cutoff-threshold must be >= 1.0, or 0 to disable");
        exit(2);
    }

    eprintln!("Search parameters:");
    eprintln!("{}", searchparams);

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

        if json {
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
                    if rootargs.subcommand_matches("learn").is_some() {
                        let iterations = args.value_of("iterations").unwrap().parse::<u8>().expect("Iterations should be an integer between 0 and 255");
                        process_learn(&mut model, stdin, &searchparams,  iterations, json, args.is_present("multi-output"), args.is_present("strict"), !retain_linebreaks, perline).expect("I/O Error");
                    } else if rootargs.subcommand_matches("search").is_some() {
                        eprintln!("(accepting standard input; enter text to search for variants, output may be delayed until end of input, enter an empty line to force output earlier)");
                        process_search(&model, stdin, &searchparams, output_lexmatch, json, progress, !retain_linebreaks, perline);
                    } else if searchparams.single_thread {
                        eprintln!("(accepting standard input; enter input to match, one per line)");
                        process(&model, stdin,  &searchparams, output_lexmatch, json, progress);
                    } else {
                        eprintln!("(accepting standard input; enter input to match, one per line, output may be delayed until end of input due to parallellisation)");
                        //normal parallel behaviour
                        process_par(&model, stdin, &searchparams, output_lexmatch, json, progress).expect("I/O Error");
                    }
                },
                _ =>  {
                    let f = File::open(filename).expect(format!("ERROR: Unable to open file {}", filename).as_str());
                    if rootargs.subcommand_matches("learn").is_some() {
                        let iterations = args.value_of("iterations").unwrap().parse::<u8>().expect("Iterations should be an integer between 0 and 255");
                        process_learn(&mut model, f, &searchparams, iterations, json, args.is_present("multi-output"), args.is_present("strict"), !retain_linebreaks, perline).expect("I/O Error");
                    } else if rootargs.subcommand_matches("search").is_some() {
                        process_search(&model, f, &searchparams, output_lexmatch, json, progress, !retain_linebreaks, perline);
                    } else if searchparams.single_thread {
                        process(&model, f, &searchparams, output_lexmatch, json, progress);
                    } else {
                        //normal parallel behaviour
                        process_par(&model, f, &searchparams, output_lexmatch, json, progress).expect("I/O Error");
                    }
                }
            }
        }

        if json  {
            println!("]");
        }

    }
}
