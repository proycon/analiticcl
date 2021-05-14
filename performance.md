# Performance Experiments

Command: ``$ time cat htr.tok.lexicon.tsv | cut -f 1 |  analiticcl query --score-threshold
0.7 --progress --alphabet ~W/analiticcl/examples/simple.alphabet.tsv --lexicon groundtruth.tok.lexicon.tsv``

Default max distance left to 3

Used data can be found in: https://github.com/knaw-huc/golden-agents-htr/tree/master/experiments

Computed on a octa-core Intel(R) Core(TM) i7-4770K CPU @ 3.50GHz

## Anagram hashing

with old big-int library (analiticcl <= v0.1.1), single-threaded, no search cache:

```
@ 1001 - processing speed was 238 items per second
@ 2001 - processing speed was 161 items per second
@ 3001 - processing speed was 172 items per second
@ 4001 - processing speed was 168 items per second
```

with ibig library, single-threaded, no search cache:

```
@ 1001 - processing speed was 580 items per second
@ 2001 - processing speed was 433 items per second
@ 3001 - processing speed was 435 items per second
@ 4001 - processing speed was 439 items per second
```

single-threaded with search cache:

```
@ 1001 - processing speed was 1218 items per second
@ 2001 - processing speed was 1139 items per second
@ 3001 - processing speed was 791 items per second
@ 4001 - processing speed was 814 items per second
```

multi-threaded (8 threads), no search cache (can't be shared efficiently over threads):

```
@ 1000 - processing speed was 2532 items per second
@ 2000 - processing speed was 1880 items per second
@ 3000 - processing speed was 1969 items per second
@ 4000 - processing speed was 1992 items per second
@ 5000 - processing speed was 1664 items per second
```

## Finite State Transducer Map with Levensthein Automatons

Using the [fst](https://github.com/BurntSushi/fst) library, not using any anagram hashing whatsoever (see experimental
fst branch of analiticcl).

Note: has significantly higher memory usage (in the order of 250-400MB)

single-threaded (no caching):

```
@ 1001 - processing speed was 148 items per second
@ 2001 - processing speed was 111 items per second
@ 3001 - processing speed was 116 items per second
@ 4001 - processing speed was 110 items per second
@ 5001 - processing speed was 107 items per second
```

multi-threaded (8 threads, no caching):

```
@ 1000 - processing speed was 383 items per second
@ 2000 - processing speed was 280 items per second
@ 3000 - processing speed was 287 items per second
@ 4000 - processing speed was 268 items per second
@ 5000 - processing speed was 266 items per second
```
