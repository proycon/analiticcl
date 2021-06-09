import sys
import os
import json
from analiticcl import VariantModel, Weights, SearchParameters

try:
    basedir = sys.argv[1]
except:
    basedir = "../../../"

model = VariantModel(os.path.join(basedir,"examples","simple.alphabet.tsv"), Weights(), debug=False)
model.read_lexicon(os.path.join(basedir, "examples","eng.aspell.lexicon"))
model.build()
result = model.find_variants("udnerstand", SearchParameters(max_edit_distance=3))
print(json.dumps(result, ensure_ascii=False, indent=4))
