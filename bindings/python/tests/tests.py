import unittest

from analiticcl import VariantModel, Weights, SearchParameters
from icecream import ic

LEXICON_AMPHIBIANS = 'tests/amphibians.tsv'
LEXICON_REPTILES = 'tests/reptiles.tsv'


class AnaliticclPythonBindingTests(unittest.TestCase):

    def test_find_all_matches_with_multiple_lexicons(self):
        model = VariantModel("../../examples/simple.alphabet.tsv", Weights(), debug=False)
        model.read_lexicon(LEXICON_AMPHIBIANS)
        model.read_lexicon(LEXICON_REPTILES)
        model.build()
        results = model.find_all_matches("Salamander lizard frog snake toad",
                                         SearchParameters(max_edit_distance=3, max_ngram=1))
        ic(results)

        self.assertEqual(len(results), 5)
        self.assert_result(results[0], 'Salamander', LEXICON_AMPHIBIANS, 'salamander')
        self.assert_result(results[1], 'lizard', LEXICON_REPTILES)
        self.assert_result(results[2], 'frog', LEXICON_AMPHIBIANS)
        self.assert_result(results[3], 'snake', LEXICON_REPTILES)
        self.assert_result(results[4], 'toad', LEXICON_AMPHIBIANS)

    def assert_result(self, result, orig_term, lexicon, lex_term=None):
        if not lex_term:
            lex_term = orig_term
        self.assertEqual(result['input'], orig_term)
        assert len(result['variants']) > 0
        best_match = result['variants'][0]
        self.assertEqual(best_match['text'], lex_term)
        self.assertEqual(best_match['lexicons'], [lexicon])


if __name__ == '__main__':
    unittest.main()
