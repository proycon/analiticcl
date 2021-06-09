from analiticcl import VariantModel, Weights, SearchParameters

model = VariantModel("examples/simple.alphabet.tsv", Weights(), debug=False)
model.read_lexicon("examples/eng.aspell.lexicon")
model.build()
print(model.find_variants("udnerstand", SearchParameters()))
