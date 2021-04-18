[![Crate](https://img.shields.io/crates/a/analiticcl.svg)](https://crates.io/crates/analiticcl)
[![GitHub release](https://img.shields.io/github/release/proycon/analiticcl.svg)](https://GitHub.com/procon/analiticcl/releases/)
[![Project Status: WIP – Initial development is in progress, but there has not yet been a stable, usable release suitable for the public.](https://www.repostatus.org/badges/latest/wip.svg)](https://www.repostatus.org/#wip)
# Analiticcl

## Introduction

Analiticcl is an approximate string matching or fuzzy-matching system that can be used for spelling
correction or text normalisation (such as post-OCR correction or post-HTR correction). Texts can be checked against a
validated or corpus-derived lexicon (with or without frequency information) and spelling variants will be returned.

The distinguishing feature of the system is the usage of anagram hashing to drastically reduce the search space and make
quick lookups possible even over larger edit distances. The underlying idea is largely derived from prior work *TICCL*
(Reynaert 2010; Reynaert 2004), which was implemented in [ticcltools](https://github.com/languagemachines/ticcltools).
This *analiticcl* implementation attempts to re-implement the core of these ideas from scratch, but also introduces some
novelties, such as the introduction of prime factors for improved anagram hashing. We aim at a high-performant lightweight
implementation writted in [Rust](https://www.rust-lang.org).


## Features

(section to write still)


## Installation

You can build and install analiticcl using Rust's package manager:

```
cargo install analiticcl
```

No cargo/rust on your system yet? Do ``sudo apt install cargo`` on Debian/ubuntu based systems, ``brew install rust`` on mac, or use [rustup](https://rustup.rs/).

## Usage

(section to write still)

```
analiticcl --help
```


## References

* Boytsov, Leonid. (2011). Indexing methods for approximate dictionary searching: Comparative analysis. ACM Journal of Experimental Algorithmics. 16. https://doi.org/10.1145/1963190.1963191.
* Reynaert, Martin. (2004) Text induced spelling correction. In: Proceedings COLING 2004, Geneva (2004). https://doi.org/10.3115/1220355.1220475
* Reynaert, Martin. (2011) Character confusion versus focus word-based correction of spelling and OCR variants in corpora. IJDAR 14, 173–187 (2011). https://doi.org/10.1007/s10032-010-0133-5



