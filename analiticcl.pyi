from __future__ import annotations

from typing import List, Optional, Union, Tuple


class SearchParameters:
    """An instance of this class holds a configuration for variant search."""

    def __init__(self, **kwargs):
        """Weights to assign to various computations done in the :class:`VariantModel`. 
        Values that are not provided as keyword arguments will be set to their defaults.
        Weights don't necessarily have to sum to one if you provide them all, it will be normalised later.

        Keyword Arguments
        -------------------
        max_anagram_distance: Union[int,float,Tuple[float,int]]
            Maximum anagram distance. The difference in characters (regardless of order)
            Must be an integer expressing an absolute value, or float in range 0-1 expressing a ratio. Or a two-tuple expressing a ratio with an absolute limit (float, int)

        max_edit_distance: Union[int,float,Tuple[float,int]]
            Maximum edit distance (levenshtein-damarau). The maximum edit distance according to Levenshtein-Damarau. Insertions, deletions, substitutions and transposition all have the same cost (1). It is recommended to set this value slightly lower than the maximum anagram distance.
            Must be an integer expressing an absolute value, or float in range 0-1 expressing a ratio. Or a two-tuple expressing a ratio with an absolute limit (float, int)

        max_matches: int
            Number of matches to return per input (set to 0 for unlimited if you want to exhaustively return every possibility within the specified anagram and edit distance)

        score_threshold: float
            Require scores to meet this threshold, they are pruned otherwise

        cutoff_threshold: float
            Cut-off threshold: if a score in the ranking is a specific factor greater than the best score, the ranking will be cut-off at that point and the score not included. Should be set to a value like 2.

        stop_criterion: bool
            Determines when to stop searching for matches. Enabling this can speed up the process at the
            cost of lower accuracy

        max_ngram: int
            Maximum ngram order (1 for unigrams, 2 for bigrams, etc..).

        lm_order: int
            Maximum ngram order for Language Models (2 for bigrams, etc..).

        max_seq: int
            Maximum number of candidate sequences to take along to the language modelling stage

        single_thread: bool
            Use only a single-thread instead of leveraging multiple cores (lowers resource use and
            performance)

        context_weight: float
            Weight attributed to the language model in relation to the variant model (e.g. 2.0 = twice
            as much weight) when considering input context and rescoring.

        variantmodel_weight: float
            Weight attributed to the variant model in finding the most likely sequence

        lm_weight: float
            Weight attributed to the language model in finding the most likely sequence

        contextrules_weight: float
            Weight attributed to the context rules model in finding the most likely sequence

        freq_weight: float
            Weight attributed to the frequency information in frequency reranking, in relation to
            the similarity component. 0 = disabled)

        consolidate_matches: bool
            Consolidate matches and extract a single most likely sequence, if set
            to false, all possible matches (including overlapping ones) are returned.

        unicodeoffsets: bool
            Output text offsets in unicode points rather than UTF-8 byte offsets
        """

        def get_max_anagram_distance(self) -> Union[int,float,Tuple[float,int]]:
            """
            Maximum anagram distance. The difference in characters (regardless of order)
            Must be an integer expressing an absolute value, or float in range 0-1 expressing a ratio. Or a two-tuple expressing a ratio with an absolute limit (float, int)
            """

        def get_edit_distance(self) -> Union[int,float,Tuple[float,int]]:
            """
            Maximum edit distance (levenshtein-damarau). The maximum edit distance according to Levenshtein-Damarau. Insertions, deletions, substitutions and transposition all have the same cost (1). It is recommended to set this value slightly lower than the maximum anagram distance.
            Must be an integer expressing an absolute value, or float in range 0-1 expressing a ratio. Or a two-tuple expressing a ratio with an absolute limit (float, int)
            """

        def get_max_matches(self) -> int:
            """Returns number of matches to return per input (set to 0 for unlimited if you want to exhaustively return every possibility within the specified anagram and edit distance)"""

        def get_score_threshold(self) -> float:
            """Require scores to meet this threshold, they are pruned otherwise"""

        def get_cutoff_threshold(self) -> float:
            """Cut-off threshold: if a score in the ranking is a specific factor greater than the best score, the ranking will be cut-off at that point and the score not included. Should be set to a value like 2."""

        def get_stop_criterion(self) -> bool:
            """Determines when to stop searching for matches. Enabling this can speed up the process at the
            cost of lower accuracy"""

        def get_max_ngram(self) -> int:
            """Maximum ngram order (1 for unigrams, 2 for bigrams, etc..)."""

        def get_lm_order(self) -> int:
            """Maximum ngram order for Language Models (2 for bigrams, etc..)."""

        def get_max_seq(self) -> int:
            """Maximum number of candidate sequences to take along to the language modelling stage"""

        def get_single_thread(self) -> bool:
            """Use only a single-thread instead of leveraging multiple cores (lowers resource use and
            performance)"""

        def get_context_weight(self) -> float:
            """Weight attributed to the language model in relation to the variant model (e.g. 2.0 = twice
            as much weight) when considering input context and rescoring."""

        def get_variantmodel_weight(self) -> float:
            """Weight attributed to the variant model in finding the most likely sequence"""

        def get_lm_weight(self) -> float:
            """Weight attributed to the language model in finding the most likely sequence"""

        def get_contextrules_weight(self) -> float:
            """Weight attributed to the context rules model in finding the most likely sequence"""

        def get_freq_weight(self) -> float:
            """Weight attributed to the frequency information in frequency reranking, in relation to
            the similarity component. 0 = disabled)"""

        def get_consolidate_matches(self) -> bool:
            """Consolidate matches and extract a single most likely sequence, if set
            to false, all possible matches (including overlapping ones) are returned."""

        def get_unicodeoffsets(self) -> bool:
            """Output text offsets in unicode points rather than UTF-8 byte offsets"""

        def to_dict(self) -> dict:
            """Returns all parameters in a dictionary"""

class VocabParams:
    """Configuration passed when loading vocabularies (lexicons, frequency lists) etc"""

    
    def __init__(self, **kwargs):
       """Configuration passed when loading vocabularies (lexicons, frequency lists) etc.

       Keyword Arguments
       --------------------
        
        text_column: int
            Column containing the Text (if any, 0-indexed)

        freq_column: int
            Column containing the frequency (if any, 0-indexed)

        freq_handling: str
            Frequency handling in case of duplicate items (may be across multiple lexicons), can be "sum","max","min","replace"

        vocabtype: str
            "NONE", "INDEXED", "TRANSPARENT" or "LM"
       """


class Weights:
    """Holds the weights for the :class:`VariantModel`"""

    def __init__(self, **kwargs):
        """Weights to assign to various computations done in the :class:`VariantModel`. 
        Values that are not provided as keyword arguments will be set to their defaults.
        Weights don't necessarily have to sum to one if you provide them all, it will be normalised later.

        Keyword Arguments
        -------------------

        ld: float
            Weight for the Levenshtein (or Damarau-Levenshtein) distance

        lcs: float
            Weight for the Longest common substring length

        prefix: float
            Weight for the prefix length

        suffix: float
            Weight for the suffix length

        case: float
            Weight to assign to difference in case (lowercase/uppercase)
        """

        def get_ld(self) -> float:
            """Returns the weight for the Levenshtein (or Damarau-Levenshtein) distance"""

        def get_lcs(self) -> float:
            """Returns the weight for the Longest common substring length"""

        def get_prefix(self) -> float:
            """Returns the weight for the prefix length"""

        def get_suffix(self) -> float:
            """Returns the weight for the suffix length"""

        def get_case(self) -> float:
            """Returns the weight for the case differences"""

        def set_ld(self, value:float):
            """Sets the weight for the Levenshtein (or Damarau-Levenshtein) distance"""

        def set_lcs(self, value: float):
            """Sets the weight for the Longest common substring length"""

        def set_prefix(self, value: float):
            """Sets the weight for the prefix length"""

        def set_suffix(self, value: float):
            """Sets the weight for the suffix length"""

        def set_case(self, value: float):
            """Sets the weight for the case differences"""

        def to_dict(self) -> dict:
            """Returns all weights as a dictionary"""


class VariantModel:
    """The VariantModel is the most high-level model of analiticcl, it holds all data required for variant matching."""

    def __init__(self, alphabet_file: str, weights: Weights, debug: int = 0):
        """Instantiate a new variant model

        Parameters
        --------------

        alphabet_file: str
            Path to the alphabet file to load for this model

        weights: Weights
            Weights for the model

        debug: int
            Debug level
        """

    def build(self):
        """
        Build the anagram index (and secondary index) so the model
        is ready for variant matching
        """

    def add_to_vocabulary(self, text: str, frequency: Optional[int], params: VocabParams):
        """
        Add an item to the vocabulary. This is a lower-level interface.
        """

    def read_vocabulary(self, filename: str, params: VocabParams):
        """
        Load vocabulary (a lexicon or corpus-derived lexicon) from a TSV file
        May contain frequency information. This is a lower-level interface.
        The parameters define what value can be read from what column
        """

    def add_contextrule(self, pattern: str, score: float, tag: List[str], tagoffset: List[str]):
        pass

    def read_lexicon(self, filename: str):
        """
        Higher order function to load a lexicon and make it available to the model.
        Wraps around read_vocabulary() with default parameters.
        """

    def read_lm(self, filename: str):
        """
        Higher order function to load a language model and make it available to the model.
        Wraps around read_vocabulary() with default parameters.
        """

    def read_variants(self, filename: str):
        """
        Load a weighted variant list (set transparent to true if this is an error list and you
        don't want the variants themselves to be returned when matching; i.e. they are transparent)
        """

    def read_confusiblelist(self, filename: str):
        """
        Load a confusable list
        """

    def read_contextrules(self, filename: str):
        """
        Load context rules from a TSV file
        """

    def __contains__(self, text: str):
        """Is this exact text in a loaded lexicon?"""

    def find_variants(self, input: str, params: SearchParameters) -> List[dict]:
        """Find variants in the vocabulary for a given string (in its totality), returns a list of variants with scores and their source lexicons"""

    def find_variants_par(self, input: List[str], params: SearchParameters) -> List[dict]:
        """Find variants in the vocabulary for all multiple string items at once, provided in in the input list. Returns a list of variants with scores and their source lexicons. Will use parallellisation under the hood."""

    def find_all_matches(self, text: str, params: SearchParameters) -> List[dict]:
        """Searches a text and returns all highest-ranking variants found in the text"""

    def set_confusables_before_pruning(self):
        """
        Configure the model to match against known confusables prior to pruning on maximum weight.
        This corresponds to the `--early-confusables` option for the CLI version
        """
