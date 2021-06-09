
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

To use this method, you need to have the Rust installed and in your ``$PATH``. Install it through your package manager or through rustup:

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

model = VariantModel("examples/simple.alphabet.tsv", Weights(), debug=False)
model.read_lexicon("examples/eng.aspell.lexicon")
model.build()
print(model.find_variants("udnerstand", SearchParameters()))
```

Output:

```python
[{'text': 'understand', 'score': 0.8978494623655915, 'lexicon': '/home/proycon/work/analiticcl/examples/eng.aspell.lexicon'}, {'text': 'understands', 'score': 0.6725317693059629, 'lexicon': '/home/proycon/work/analiticcl/examples/eng.aspell.lexicon'}, {'text': 'understood', 'score': 0.6036866359447004, 'lexicon': '/home/proycon/work/analiticcl/examples/eng.aspell.lexicon'}, {'text': 'understate', 'score': 0.5967741935483871, 'lexicon': '/home/proycon/work/analiticcl/examples/eng.aspell.lexicon'}]
```
