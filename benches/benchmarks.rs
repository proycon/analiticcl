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

    let mut group = c.benchmark_group("anahash_edit");

    let input_avs: Vec<AnaValue> = inputs.iter().map(|input| input.anahash(&alphabet)).collect();
    let change: AnaValue = "change".anahash(&alphabet);

    for (input_av, input) in input_avs.iter().zip(inputs) {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("insert",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.insert(black_box(&change))
        }));

        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("contains",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.contains(black_box(&change))
        }));

        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("delete",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.delete(black_box(&change))
        }));

    }

    group.finish();

    let searchparams = SearchParams {
        ..Default::default()
    };

    let searchparams_bfs = SearchParams {
        breadthfirst: true,
        allow_duplicates: false,
        allow_empty_leaves: false,
        ..Default::default()
    };

    let inputs: &[&str] = &["rat","houses","benchmarking"];

    let mut group = c.benchmark_group("anahash_iter");
    for (input_av, input) in input_avs.iter().zip(inputs) {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("parents",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.iter_parents(alphabet.len() as u8).count()
        }));

        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("singlebeam",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.iter(alphabet.len() as u8).count()
        }));

        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("recursive_bfs_nodups",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.iter_recursive(alphabet.len() as u8, &searchparams_bfs).count()
        }));
    }

    group.finish();


    /*
    let mut group = c.benchmark_group("anahash_iter_parents_deletions_recursive_dfs");

    for (input_av, input) in input_avs.iter().zip(inputs) {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("anahash_iter_parent_deletions_recursive_dfs",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.iter_recursive(alphabet.len() as u8, &searchparams).count()
        }));
    }
    group.finish();

    let mut group = c.benchmark_group("anahash_iter_parents_deletions_recursive_bfs");

    for (input_av, input) in input_avs.iter().zip(inputs) {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("anahash_iter_parent_deletions_recursive_bfs",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.iter_recursive(alphabet.len() as u8, &searchparams).count()
        }));
    }
    group.finish();

    let mut group = c.benchmark_group("anahash_iter_parents_deletions_recursive_dfs_nodups");

    let searchparams = SearchParams {
        allow_duplicates: false,
        allow_empty_leaves: false,
        ..Default::default()
    };

    for (input_av, input) in input_avs.iter().zip(inputs) {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("anahash_iter_parent_deletions_recursive_dfs_nodups",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.iter_recursive(alphabet.len() as u8, &searchparams).count()
        }));
    }

    group.finish();

    let mut group = c.benchmark_group("anahash_iter_parents_deletions_recursive_bfs_nodups");

    let searchparams = SearchParams {
        breadthfirst: true,
        allow_duplicates: false,
        allow_empty_leaves: false,
        ..Default::default()
    };

    for (input_av, input) in input_avs.iter().zip(inputs) {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("anahash_iter_parent_deletions_recursive_bfs_nodups",format!("input {} chars",input.chars().count())), &input_av, |b, input_av| b.iter(||{
            input_av.iter_recursive(alphabet.len() as u8, &searchparams).count()
        }));
    }

    group.finish();

    */

    let simple_lexicon: &[&str] = &["rites","tiers", "tires","tries","tyres","rides","brides","dire"];

    let mut model = VariantModel::new_with_alphabet(get_test_alphabet().0, Weights::default(), false);

    c.bench_function("model_add_vocab", |b| b.iter(||{
        for item in black_box(simple_lexicon) {
            model.add_to_vocabulary(item,None,None,0);
        }
    }));


    c.bench_function("model_init_and_build", |b| b.iter(||{
        let mut model = VariantModel::new_with_alphabet(get_test_alphabet().0, Weights::default(), false);
        for item in black_box(simple_lexicon) {
            model.add_to_vocabulary(item,None,None,0);
        }
        model.build()
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
