
# Analiticcl

## Introduction

Analiticcl is an approximate string matching or fuzzy-matching system that can be used for spelling
correction or text normalisation (such as post-OCR correction or post-HTR correction). Texts can be checked against a
validated or corpus-derived lexicon (with or without frequency information) and spelling variants will be returned.

Please see the [main README.md](../../README.md) for a further introduction.

Analiticcl is written in Rust, this is the Python binding, allowing you to use analiticcl from Python as a module.

## Installation

### with pip

```
pip install analiticcl
```

### from source

To use this method, you need to have Rust installed and in your ``$PATH``. Install it through your package manager or through rustup:

```
curl https://sh.rustup.rs -sSf | sh -s -- -y
export PATH="$HOME/.cargo/bin:$PATH"
```
Once Rust is installed, you can compile the analiticcl binding:

```
# Create a virtual env (you can use yours as well)
python -m venv .env
source .env/bin/activate

# Install `analiticcl` in the current virtual env
pip install setuptools_rust
python setup.py install
```

## Usage

```python
from analiticcl import VariantModel, Weights, SearchParameters
import json

model = VariantModel("examples/simple.alphabet.tsv", Weights(), debug=False)
model.read_lexicon("examples/eng.aspell.lexicon")
model.build()
result = model.find_variants("udnerstand", SearchParameters(max_edit_distance=3))
print(json.dumps(result, ensure_ascii=False, indent=4))
print()
results = model.find_all_matches("I do not udnerstand the probleem", SearchParameters(max_edit_distance=3,max_ngram=1))
print(json.dumps(results, ensure_ascii=False, indent=4))
```

**Note:** all offsets reported by analiticcl are utf-8 byte-offsets, not character offsets!


Output:

```json
[
    {
        "text": "understand",
        "score": 0.8978494623655915,
        "lexicon": "../../../examples/eng.aspell.lexicon"
    },
    {
        "text": "understands",
        "score": 0.6725317693059629,
        "lexicon": "../../../examples/eng.aspell.lexicon"
    },
    {
        "text": "understood",
        "score": 0.6036866359447004,
        "lexicon": "../../../examples/eng.aspell.lexicon"
    },
    {
        "text": "understate",
        "score": 0.5967741935483871,
        "lexicon": "../../../examples/eng.aspell.lexicon"
    }
]
```

```json
[
    {
        "input": "I",
        "offset": {
            "begin": 0,
            "end": 1
        },
        "variants": [
            {
                "text": "I",
                "score": 0.8387096774193549,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "i",
                "score": 0.8064516129032258,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            }
        ]
    },
    {
        "input": "do",
        "offset": {
            "begin": 2,
            "end": 4
        },
        "variants": [
            {
                "text": "do",
                "score": 1.0,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "dog",
                "score": 0.6236559139784946,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "doc",
                "score": 0.6236559139784946,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "doz",
                "score": 0.6236559139784946,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "dob",
                "score": 0.6236559139784946,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "doe",
                "score": 0.6236559139784946,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "dot",
                "score": 0.6236559139784946,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "dos",
                "score": 0.6236559139784946,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "ado",
                "score": 0.6236559139784946,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "don",
                "score": 0.6236559139784946,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "d",
                "score": 0.5967741935483871,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "o",
                "score": 0.5967741935483871,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "DOD",
                "score": 0.5913978494623655,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            }
        ]
    },
    {
        "input": "not",
        "offset": {
            "begin": 5,
            "end": 8
        },
        "variants": [
            {
                "text": "not",
                "score": 1.0,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "knot",
                "score": 0.6370967741935484,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "note",
                "score": 0.6370967741935484,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "snot",
                "score": 0.6370967741935484,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "no",
                "score": 0.6236559139784946,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "nowt",
                "score": 0.5967741935483871,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "No",
                "score": 0.5913978494623655,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "OT",
                "score": 0.5913978494623655,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "pot",
                "score": 0.5698924731182795,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            }
        ]
    },
    {
        "input": "udnerstand",
        "offset": {
            "begin": 9,
            "end": 19
        },
        "variants": [
            {
                "text": "understand",
                "score": 0.8978494623655915,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "understands",
                "score": 0.6725317693059629,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "understood",
                "score": 0.6036866359447004,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "understate",
                "score": 0.5967741935483871,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            }
        ]
    },
    {
        "input": "the",
        "offset": {
            "begin": 20,
            "end": 23
        },
        "variants": [
            {
                "text": "the",
                "score": 1.0,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "thee",
                "score": 0.6908602150537635,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "thew",
                "score": 0.6370967741935484,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "then",
                "score": 0.6370967741935484,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "them",
                "score": 0.6370967741935484,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "they",
                "score": 0.6370967741935484,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "he",
                "score": 0.6236559139784946,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "Thea",
                "score": 0.6048387096774194,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "Th",
                "score": 0.5913978494623655,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "He",
                "score": 0.5913978494623655,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "thy",
                "score": 0.5698924731182795,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "she",
                "score": 0.5698924731182795,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "tho",
                "score": 0.5698924731182795,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "Thu",
                "score": 0.5376344086021505,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "Che",
                "score": 0.5376344086021505,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "THC",
                "score": 0.5376344086021505,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "tee",
                "score": 0.5161290322580645,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "toe",
                "score": 0.5161290322580645,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "tie",
                "score": 0.5161290322580645,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "Te",
                "score": 0.510752688172043,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            }
        ]
    },
    {
        "input": "probleem",
        "offset": {
            "begin": 24,
            "end": 32
        },
        "variants": [
            {
                "text": "problem",
                "score": 0.9231950844854071,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "problems",
                "score": 0.6908602150537635,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "probe",
                "score": 0.5913978494623656,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "proclaim",
                "score": 0.5766129032258065,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "probated",
                "score": 0.543010752688172,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "probates",
                "score": 0.543010752688172,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "prole",
                "score": 0.5322580645161291,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "prowlers",
                "score": 0.4959677419354839,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            },
            {
                "text": "parolees",
                "score": 0.44220430107526887,
                "lexicon": "../../../examples/eng.aspell.lexicon"
            }
        ]
    }
]

```

## Documentation

The python binding exposes only a minimal interface, you can use Python's ``help()`` function to get information on the
classes provided. For more detailed information, please consult the [Analiticcl's rust API documentation](https://docs.rs/analiticcl/). The interfaces that are available in the binding are analogous to the rust versions.
