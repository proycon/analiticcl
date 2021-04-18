use criterion::{BenchmarkId,black_box, criterion_group, criterion_main, Criterion};

use analiticcl::*;
use analiticcl::test::*;

pub fn anahash_benchmark(c: &mut Criterion) {
    let (alphabet, alphabet_size) = get_test_alphabet();

    c.bench_with_input(BenchmarkId::new("anahash_single_char","alphabet"), &alphabet, |b, alphabet| b.iter(||{
        "a".anahash(&alphabet)
    }));

    c.bench_with_input(BenchmarkId::new("anahash_word_6_chars","alphabet"), &alphabet, |b, alphabet| b.iter(||{
        "houses".anahash(&alphabet)
    }));

    c.bench_with_input(BenchmarkId::new("anahash_word_12_chars","alphabet"), &alphabet, |b, alphabet| b.iter(||{
        "benchmarking".anahash(&alphabet)
    }));

    c.bench_with_input(BenchmarkId::new("anahash_sentence_34_chars","alphabet"), &alphabet, |b, alphabet| b.iter(||{
        "the lazy dog jumped over the quick brown fox".anahash(&alphabet)
    }));
}

criterion_group!(benches, anahash_benchmark);
criterion_main!(benches);
