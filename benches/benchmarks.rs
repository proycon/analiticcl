use criterion::{BenchmarkId,Throughput,black_box, criterion_group, criterion_main, Criterion};

use analiticcl::*;
use analiticcl::test::*;

pub fn benchmarks(c: &mut Criterion) {
    let (alphabet, alphabet_size) = get_test_alphabet();


    let inputs: &[&str] = &["a","rat","houses","benchmarking","the lazy dog jumped over the quick brown fox"];

    let mut group = c.benchmark_group("anahash_benchmark");
    for input in inputs {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("anahash",format!("input {} chars",input.chars().count())), &input, |b, input| b.iter(||{
            input.anahash(&alphabet)
        }));
    }

    group.finish();

    let mut group = c.benchmark_group("anahash_insertion_benchmark");

    let input_avs: Vec<AnaValue> = inputs.iter().map(|input| input.anahash(&alphabet)).collect();
    let change: AnaValue = "change".anahash(&alphabet);

    for (input_av, input) in input_avs.iter().zip(inputs) {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("anahash",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.insert(black_box(&change));
        }));
    }

    group.finish();

    let mut group = c.benchmark_group("anahash_contains_benchmark");

    let input_avs: Vec<AnaValue> = inputs.iter().map(|input| input.anahash(&alphabet)).collect();
    let change: AnaValue = "change".anahash(&alphabet);

    for (input_av, input) in input_avs.iter().zip(inputs) {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("anahash",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.contains(black_box(&change));
        }));
    }

    group.finish();

    let mut group = c.benchmark_group("anahash_deletion_benchmark");

    let input_avs: Vec<AnaValue> = inputs.iter().map(|input| input.anahash(&alphabet)).collect();
    let change: AnaValue = "change".anahash(&alphabet);

    for (input_av, input) in input_avs.iter().zip(inputs) {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("anahash",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.delete(black_box(&change));
        }));
    }

    group.finish();


    let simple_lexicon: &[&str] = &["rites","tiers", "tires","tries","tyres","rides","brides","dire"];

    let mut model = VariantModel::new_with_alphabet(get_test_alphabet().0, Weights::default(), false);

    c.bench_function("model_add_vocab", |b| b.iter(||{
        for item in black_box(simple_lexicon) {
            model.add_to_vocabulary(item,None,None);
        }
    }));


    c.bench_function("model_init_and_train", |b| b.iter(||{
        let mut model = VariantModel::new_with_alphabet(get_test_alphabet().0, Weights::default(), false);
        for item in black_box(simple_lexicon) {
            model.add_to_vocabulary(item,None,None);
        }
        model.train()
    }));

}

/*
pub fn model_benchmark(c: &mut Criterion) {
    let (alphabet, alphabet_size) = get_test_alphabet();
    let lexicon: &[&str] = &["rites","tiers", "tires","tries","tyres","rides","brides","dire"];

    c.bench_with_input(BenchmarkId::new("model_load","alphabet"), &alphabet, |b, alphabet| b.iter(||{
        let model = VariantModel::new_with_alphabet(alphabet, Weights::default(), true);
    }));

}
*/

criterion_group!(benches, benchmarks);
criterion_main!(benches);
