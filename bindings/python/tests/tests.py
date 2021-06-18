import unittest

from analiticcl import VariantModel, Weights, SearchParameters
from icecream import ic

LEXICON_AMPHIBIANS = 'tests/amphibians.tsv'
LEXICON_REPTILES = 'tests/reptiles.tsv'


class AnaliticclPythonBindingTests(unittest.TestCase):

    def test_find_all_matches_with_multiple_lexicons(self):
        model = VariantModel("tests/simple.alphabet.tsv", Weights(), debug=True)
        model.read_lexicon(LEXICON_AMPHIBIANS)
        model.read_lexicon(LEXICON_REPTILES)
        model.build()
        results = model.find_all_matches("Salamander lizard frog snake toad",
                                         SearchParameters(max_edit_distance=3, max_ngram=1))
        ic(results)

        assert len(results) == 5
        assert_result(results[0], 'Salamander', LEXICON_AMPHIBIANS, 'salamander')
        assert_result(results[1], 'lizard', LEXICON_REPTILES)
        assert_result(results[2], 'frog', LEXICON_AMPHIBIANS)
        assert_result(results[3], 'snake', LEXICON_REPTILES)
        assert_result(results[4], 'toad', LEXICON_AMPHIBIANS)


def assert_result(result, orig_term, lexicon, lex_term=None):
    if not lex_term:
        lex_term = orig_term
    assert result['input'] == orig_term, f"expected {orig_term}, but was: {result['input']}"
    assert len(result['variants']) > 0
    best_match = result['variants'][0]
    assert best_match['text'] == lex_term, f"expected {lex_term}, but was: {best_match['text']}"
    assert best_match['lexicon'] == lexicon, f"expected {lexicon}, but was: {best_match['lexicon']}"


if __name__ == '__main__':
    unittest.main()