extern crate analiticcl as libanaliticcl;

use std::str::FromStr;
use rayon::prelude::*;
use pyo3::prelude::*;
use pyo3::types::*;
use pyo3::exceptions::{PyRuntimeError,PyValueError};
//use pyo3::wrap_pymodule;


#[pyclass(dict,name="Weights")]
#[derive(Default,Clone)]
pub struct PyWeights {
    weights: libanaliticcl::Weights
}

#[pymethods]
impl PyWeights {
    #[new]
    #[args(
        kwargs = "**"
    )]
    fn new(kwargs: Option<&PyDict>) -> Self {
        let mut instance = Self::default();
        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs {
                if let Some(key) = key.extract().unwrap() {
                    match key {
                        "ld" => if let Ok(Some(value)) = value.extract() {
                            instance.weights.ld = value
                         },
                        "lcs" => if let Ok(Some(value)) = value.extract() {
                            instance.weights.lcs = value
                         },
                        "prefix" => if let Ok(Some(value)) = value.extract() {
                            instance.weights.prefix = value
                         },
                        "suffix" => if let Ok(Some(value)) = value.extract() {
                            instance.weights.suffix = value
                         },
                        "case" => if let Ok(Some(value)) = value.extract() {
                            instance.weights.case = value
                         },
                        _ => eprintln!("Ignored unknown kwargs option {}", key),
                    }
                }
            }
        }
        instance
    }

    #[getter]
    fn get_ld(&self) -> PyResult<f64> { Ok(self.weights.ld) }
    #[getter]
    fn get_lcs(&self) -> PyResult<f64> { Ok(self.weights.lcs) }
    #[getter]
    fn get_prefix(&self) -> PyResult<f64> { Ok(self.weights.prefix) }
    #[getter]
    fn get_suffix(&self) -> PyResult<f64> { Ok(self.weights.suffix) }
    #[getter]
    fn get_case(&self) -> PyResult<f64> { Ok(self.weights.case) }

    #[setter]
    fn set_ld(&mut self, value: f64) -> PyResult<()> { self.weights.ld = value; Ok(()) }
    #[setter]
    fn set_lcs(&mut self, value: f64) -> PyResult<()> { self.weights.lcs = value; Ok(()) }
    #[setter]
    fn set_prefix(&mut self, value: f64) -> PyResult<()> { self.weights.prefix = value; Ok(()) }
    #[setter]
    fn set_suffix(&mut self, value: f64) -> PyResult<()> { self.weights.suffix = value; Ok(()) }
    #[setter]
    fn set_case(&mut self, value: f64) -> PyResult<()> { self.weights.case = value; Ok(()) }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<&'py PyDict> {
        let dict = PyDict::new(py);
        dict.set_item("ld", self.get_ld()?)?;
        dict.set_item("lcs", self.get_lcs()?)?;
        dict.set_item("prefix", self.get_prefix()?)?;
        dict.set_item("suffix", self.get_suffix()?)?;
        dict.set_item("case", self.get_case()?)?;
        Ok(dict)
    }
}


//should ideally be implemented using FromPyObject but can't do that because libanaliticcl is not considered not crate-internal anymore here
fn extract_distance_threshold(value: &PyAny) -> PyResult<libanaliticcl::DistanceThreshold> {
    if let Ok(Some((v,limit))) = value.extract() {
        Ok(libanaliticcl::DistanceThreshold::RatioWithLimit(v,limit))
    } else if let Ok(Some(v)) = value.extract() {
        Ok(libanaliticcl::DistanceThreshold::Absolute(v))
    } else if let Ok(Some(v)) = value.extract() {
        Ok(libanaliticcl::DistanceThreshold::Ratio(v))
    } else if let Ok(Some(v)) = value.extract() {
        if let Ok(v) = libanaliticcl::DistanceThreshold::from_str(v) {
            Ok(v)
        } else {
            Err(PyValueError::new_err(format!("Unable to convert from string ({}). Must be an integer expressing an absolute value, or float in range 0-1 expressing a ratio. Or a two-tuple expression a ratio with an absolute limit (float;int)",v)))
        }
    } else {
        Err(PyValueError::new_err("Must be an integer expressing an absolute value, or float in range 0-1 expressing a ratio. Or a two-tuple expression a ratio with an absolute limit (float, int)"))
    }
}



#[pyclass(dict,name="SearchParameters")]
#[derive(Default,Clone)]
pub struct PySearchParameters {
    data: libanaliticcl::SearchParameters
}

#[pymethods]
impl PySearchParameters {
    #[new]
    #[args(
        kwargs = "**"
    )]
    fn new(kwargs: Option<&PyDict>) -> Self {
        let mut instance = Self::default();
        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs {
                if let Some(key) = key.extract().unwrap() {
                    match key {
                        "max_anagram_distance" => match extract_distance_threshold(value) {
                            Ok(v) => instance.data.max_anagram_distance = v,
                            Err(v) => eprintln!("{}", v)
                        },
                        "max_edit_distance" => match extract_distance_threshold(value) {
                            Ok(v) => instance.data.max_edit_distance = v,
                            Err(v) => eprintln!("{}", v)
                        },
                        "max_matches" => match value.extract() {
                            Ok(Some(value)) => instance.data.max_matches = value,
                            Ok(None) => eprintln!("No value specified for max_matches parameter"),
                            Err(v) => eprintln!("{}", v)
                         },
                        "score_threshold" => match value.extract() {
                            Ok(Some(value)) => instance.data.score_threshold = value,
                            Ok(None) => eprintln!("No value specified for score_threshold parameter"),
                            Err(v) => eprintln!("{}", v)
                         },
                        "cutoff_threshold" => match value.extract() {
                            Ok(Some(value)) => instance.data.cutoff_threshold = value,
                            Ok(None) => eprintln!("No value specified for cutoff_threshold parameter"),
                            Err(v) => eprintln!("{}", v)
                         },
                        "max_ngram" => match value.extract() {
                            Ok(Some(value)) => instance.data.max_ngram = value,
                            Ok(None) => eprintln!("No value specified for cutoff_threshold parameter"),
                            Err(v) => eprintln!("{}", v)
                         },
                        "max_seq" => match value.extract() {
                            Ok(Some(value)) => instance.data.max_seq = value,
                            Ok(None) => eprintln!("No value specified for cutoff_threshold parameter"),
                            Err(v) => eprintln!("{}", v)
                         },
                        "stop_at_exact_match" => {
                            if let Ok(Some(value)) = value.extract() {
                                if value {
                                    instance.data.stop_criterion = libanaliticcl::StopCriterion::StopAtExactMatch;
                                } else {
                                    instance.data.stop_criterion = libanaliticcl::StopCriterion::Exhaustive;
                                }
                            }
                         },
                        "single_thread" => match value.extract() {
                            Ok(Some(value)) => instance.data.single_thread = value,
                            Ok(None) => eprintln!("No value specified for cutoff_threshold parameter"),
                            Err(v) => eprintln!("{}", v)
                         },
                        "freq_weight" => match value.extract() {
                            Ok(Some(value)) => instance.data.freq_weight = value,
                            Ok(None) => eprintln!("No value specified for cutoff_threshold parameter"),
                            Err(v) => eprintln!("{}", v)
                         },
                        "lm_weight" => match value.extract() {
                            Ok(Some(value)) => instance.data.lm_weight = value,
                            Ok(None) => eprintln!("No value specified for cutoff_threshold parameter"),
                            Err(v) => eprintln!("{}", v)
                         },
                        "variantmodel_weight" => match value.extract() {
                            Ok(Some(value)) => instance.data.variantmodel_weight = value,
                            Ok(None) => eprintln!("No value specified for cutoff_threshold parameter"),
                            Err(v) => eprintln!("{}", v)
                         },
                        "context_weight" => match value.extract() {
                            Ok(Some(value)) => instance.data.context_weight = value,
                            Ok(None) => eprintln!("No value specified for cutoff_threshold parameter"),
                            Err(v) => eprintln!("{}", v)
                         },
                        "consolidate_matches" => match value.extract() {
                            Ok(Some(value)) => instance.data.consolidate_matches = value,
                            Ok(None) => eprintln!("No value specified for cutoff_threshold parameter"),
                            Err(v) => eprintln!("{}", v)
                         },
                        _ => eprintln!("Ignored unknown kwargs option {}", key),
                    }
                }
            }
        }
        instance
    }

    #[getter]
    fn get_max_anagram_distance<'a>(&self, py: Python<'a>) -> PyResult<&'a PyAny> {
        match self.data.max_anagram_distance {
            libanaliticcl::DistanceThreshold::Absolute(value) => {
                Ok(value.into_py(py).into_ref(py))
            },
            libanaliticcl::DistanceThreshold::Ratio(value) => {
                Ok(value.into_py(py).into_ref(py))
            },
            libanaliticcl::DistanceThreshold::RatioWithLimit(value, limit) => {
                let dict = PyDict::new(py);
                dict.set_item("ratio", value)?;
                dict.set_item("limit", limit)?;
                Ok( dict )
            }
        }
    }
    #[getter]
    fn get_max_edit_distance<'a>(&self, py: Python<'a>) -> PyResult<&'a PyAny> {
        match self.data.max_edit_distance {
            libanaliticcl::DistanceThreshold::Absolute(value) => {
                Ok(value.into_py(py).into_ref(py))
            },
            libanaliticcl::DistanceThreshold::Ratio(value) => {
                Ok(value.into_py(py).into_ref(py))
            },
            libanaliticcl::DistanceThreshold::RatioWithLimit(value, limit) => {
                let dict = PyDict::new(py);
                dict.set_item("ratio", value)?;
                dict.set_item("limit", limit)?;
                Ok( dict )
            }
        }
    }
    #[getter]
    fn get_max_matches(&self) -> PyResult<usize> { Ok(self.data.max_matches) }
    #[getter]
    fn get_score_threshold(&self) -> PyResult<f64> { Ok(self.data.score_threshold) }
    #[getter]
    fn get_cutoff_threshold(&self) -> PyResult<f64> { Ok(self.data.cutoff_threshold) }
    #[getter]
    fn get_max_ngram(&self) -> PyResult<u8> { Ok(self.data.max_ngram) }
    #[getter]
    fn get_max_seq(&self) -> PyResult<usize> { Ok(self.data.max_seq) }
    #[getter]
    fn get_single_thread(&self) -> PyResult<bool> { Ok(self.data.single_thread) }
    #[getter]
    fn get_context_weight(&self) -> PyResult<f32> { Ok(self.data.context_weight) }
    #[getter]
    fn get_freq_weight(&self) -> PyResult<f32> { Ok(self.data.freq_weight) }
    #[getter]
    fn get_lm_weight(&self) -> PyResult<f32> { Ok(self.data.lm_weight) }
    #[getter]
    fn get_variantmodel_weight(&self) -> PyResult<f32> { Ok(self.data.variantmodel_weight) }
    #[getter]
    fn get_consolidate_matches(&self) -> PyResult<bool> { Ok(self.data.consolidate_matches) }

    #[setter]
    fn set_max_anagram_distance(&mut self, value: &PyAny) -> PyResult<()> {
        let v = extract_distance_threshold(value)?;
        self.data.max_anagram_distance = v;
        Ok(())
    }
    #[setter]
    fn set_max_edit_distance(&mut self, value: &PyAny) -> PyResult<()> {
        let v = extract_distance_threshold(value)?;
        self.data.max_edit_distance = v;
        Ok(())
    }
    #[setter]
    fn set_max_matches(&mut self, value: usize) -> PyResult<()> { self.data.max_matches = value; Ok(()) }
    #[setter]
    fn set_max_ngram(&mut self, value: u8) -> PyResult<()> { self.data.max_ngram = value; Ok(()) }
    #[setter]
    fn set_max_seq(&mut self, value: usize) -> PyResult<()> { self.data.max_seq = value; Ok(()) }
    #[setter]
    fn set_single_thread(&mut self, value: bool) -> PyResult<()> { self.data.single_thread = value; Ok(()) }
    #[setter]
    fn set_context_weight(&mut self, value: f32) -> PyResult<()> { self.data.context_weight = value; Ok(()) }
    #[setter]
    fn set_freq_weight(&mut self, value: f32) -> PyResult<()> { self.data.freq_weight = value; Ok(()) }
    #[setter]
    fn set_lm_weight(&mut self, value: f32) -> PyResult<()> { self.data.lm_weight = value; Ok(()) }
    #[setter]
    fn set_variantmodel_weight(&mut self, value: f32) -> PyResult<()> { self.data.variantmodel_weight = value; Ok(()) }

    #[setter]
    fn set_consolidate_matches(&mut self, value: bool) -> PyResult<()> { self.data.consolidate_matches = value; Ok(()) }

    #[setter]
    fn set_stop_at_exact_match(&mut self, value: bool) -> PyResult<()> { if value { self.data.stop_criterion = libanaliticcl::StopCriterion::StopAtExactMatch; } else { self.data.stop_criterion = libanaliticcl::StopCriterion::Exhaustive; }; Ok(()) }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<&'py PyDict> {
        let dict = PyDict::new(py);
        dict.set_item("max_anagram_distance", self.get_max_anagram_distance(py)?)?;
        dict.set_item("max_edit_distance", self.get_max_edit_distance(py)?)?;
        dict.set_item("max_matches", self.get_max_matches()?)?;
        dict.set_item("score_threshold", self.get_score_threshold()?)?;
        dict.set_item("cutoff_threshold", self.get_cutoff_threshold()?)?;
        dict.set_item("max_ngram", self.get_max_ngram()?)?;
        dict.set_item("max_seq", self.get_max_seq()?)?;
        dict.set_item("single_thread", self.get_single_thread()?)?;
        dict.set_item("context_weight", self.get_context_weight()?)?;
        dict.set_item("freq_weight", self.get_freq_weight()?)?;
        dict.set_item("lm_weight", self.get_lm_weight()?)?;
        dict.set_item("variantmodel_weight", self.get_variantmodel_weight()?)?;
        dict.set_item("consolidate_matches", self.get_consolidate_matches()?)?;
        Ok(dict)
    }
}

#[pyclass(dict,name="VocabParams")]
#[derive(Default,Clone)]
pub struct PyVocabParams {
    data: libanaliticcl::VocabParams
}

#[pymethods]
impl PyVocabParams {
    #[new]
    #[args(
        kwargs = "**"
    )]
    fn new(kwargs: Option<&PyDict>) -> Self {
        let mut instance = Self::default();
        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs {
                if let Some(key) = key.extract().unwrap() {
                    match key {
                        "text_column" => if let Ok(Some(value)) = value.extract() {
                            instance.data.text_column = value
                         },
                        "freq_column" => if let Ok(Some(value)) = value.extract() {
                            instance.data.freq_column = value
                         },
                        "index" => if let Ok(Some(value)) = value.extract() {
                            instance.data.index = value
                         },
                         "freqhandling" => if let Ok(Some(value)) = value.extract() {
                             match value {
                                 "sum" => instance.data.freq_handling = libanaliticcl::FrequencyHandling::Sum,
                                 "max" => instance.data.freq_handling = libanaliticcl::FrequencyHandling::Max,
                                 "min" => instance.data.freq_handling = libanaliticcl::FrequencyHandling::Min,
                                 "replace" => instance.data.freq_handling = libanaliticcl::FrequencyHandling::Replace,
                                 _ =>  eprintln!("WARNING: Ignored unknown value for VocabParams.freqhandling ({})", value),
                             }
                         },
                         "vocabtype" => if let Ok(Some(value)) = value.extract() {
                             match value {
                                 "NONE" => instance.data.vocab_type = libanaliticcl::VocabType::NONE,
                                 "INDEXED" => instance.data.vocab_type = libanaliticcl::VocabType::INDEXED,
                                 "TRANSPARENT" => instance.data.vocab_type = libanaliticcl::VocabType::TRANSPARENT | libanaliticcl::VocabType::INDEXED,
                                 "LM" => instance.data.vocab_type = libanaliticcl::VocabType::LM,
                                 _ =>  eprintln!("WARNING: Ignored unknown value for VocabParams.vocabtype ({})", value),
                            }
                        },
                        _ => eprintln!("WARNING: Ignored unknown VocabParams kwargs option {}", key),
                    }
                }
            }
        }
        instance
    }

    #[getter]
    fn get_text_column(&self) -> PyResult<u8> { Ok(self.data.text_column) }
    #[getter]
    fn get_freq_column(&self) -> PyResult<Option<u8>> { Ok(self.data.freq_column) }
    #[getter]
    fn get_index(&self) -> PyResult<u8> { Ok(self.data.index) }

    #[setter]
    fn set_text_column(&mut self, value: u8) -> PyResult<()> { self.data.text_column = value; Ok(()) }
    #[setter]
    fn set_freq_column(&mut self, value: Option<u8>) -> PyResult<()> { self.data.freq_column = value; Ok(()) }
    #[setter]
    fn set_index(&mut self, value: u8) -> PyResult<()> { self.data.index = value; Ok(()) }
}


#[pyclass(dict,name="VariantModel")]
pub struct PyVariantModel {
    model: libanaliticcl::VariantModel,
}

impl PyVariantModel {
    fn variantresult_to_dict<'py>(&self, result: &libanaliticcl::VariantResult, freq_weight: f32, py: Python<'py>) -> PyResult<&'py PyDict> {
        let dict = PyDict::new(py);
        let vocabvalue = self.model.get_vocab(result.vocab_id).expect("getting vocab by id");
        dict.set_item("text", vocabvalue.text.as_str())?;
        dict.set_item("score", result.score(freq_weight))?;
        dict.set_item("dist_score", result.dist_score)?;
        dict.set_item("freq_score", result.freq_score)?;
        if let Some(via_id) = result.via {
            let viavalue = self.model.get_vocab(via_id).expect("getting vocab by id");
            dict.set_item("via", viavalue.text.as_str())?;
        }
        let lexicons: Vec<&str> = self.model.lexicons.iter().enumerate().filter_map(|(i,name)| {
            if vocabvalue.in_lexicon(i as u8) {
                Some(name.as_str())
            } else {
                None
            }
        }).collect();
        dict.set_item("lexicons", lexicons)?;
        Ok(dict)
    }
}

#[pymethods]
impl PyVariantModel {
    #[new]
    #[args(
        alphabet_file,
        weights,
        debug=0
    )]
    fn new(alphabet_file: &str, weights: PyRef<PyWeights>, debug: u8) -> Self {
        Self {
            model: libanaliticcl::VariantModel::new(alphabet_file, weights.weights.clone(), debug)
        }
    }

    /// Build the anagram index (and secondary index) so the model
    /// is ready for variant matching
    fn build(&mut self) -> PyResult<()> {
        self.model.build();
        Ok(())
    }

    /// Add an item to the vocabulary. This is a lower-level interface.
    pub fn add_to_vocabulary(&mut self, text: &str, frequency: Option<u32>, params: PyRef<PyVocabParams>) -> PyResult<()> {
        self.model.add_to_vocabulary(text, frequency, &params.data);
        Ok(())
    }


    /// Load vocabulary (a lexicon or corpus-derived lexicon) from a TSV file
    /// May contain frequency information. This is a lower-level interface.
    /// The parameters define what value can be read from what column
    fn read_vocabulary(&mut self, filename: &str, params: PyRef<PyVocabParams>) -> PyResult<()> {
        match self.model.read_vocabulary(filename, &params.data) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyRuntimeError::new_err(format!("{}", e)))
        }
    }

    /// Higher order function to load a lexicon and make it available to the model.
    /// Wraps around read_vocabulary() with default parameters.
    fn read_lexicon(&mut self, filename: &str) -> PyResult<()> {
        match self.model.read_vocabulary(filename, &libanaliticcl::VocabParams::default()) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyRuntimeError::new_err(format!("{}", e)))
        }
    }

    /// Higher order function to load a language model and make it available to the model.
    /// Wraps around read_vocabulary() with default parameters.
    fn read_lm(&mut self, filename: &str) -> PyResult<()> {
        match self.model.read_vocabulary(filename, &libanaliticcl::VocabParams::default().with_vocab_type(libanaliticcl::VocabType::LM)) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyRuntimeError::new_err(format!("{}", e)))
        }
    }

    ///Load a weighted variant list (set transparent to true if this is an error list and you
    ///don't want the variants themselves to be returned when matching; i.e. they are transparent)
    fn read_variants(&mut self, filename: &str, transparent: bool) -> PyResult<()> {
        match self.model.read_variants(filename, Some(&libanaliticcl::VocabParams::default()), transparent) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyRuntimeError::new_err(format!("{}", e)))
        }
    }

    ///Load a confusable list
    fn read_confusablelist(&mut self, filename: &str) -> PyResult<()> {
        match self.model.read_confusablelist(filename) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyRuntimeError::new_err(format!("{}", e)))
        }
    }


    ///Is this exact text in a loaded lexicon?
    fn __contains__(&self, text: &str) -> bool {
        self.model.has(text)
    }


    /// Find variants in the vocabulary for a given string (in its totality), returns a list of variants with scores and their source lexicons
    fn find_variants<'py>(&self, input: &str, params: PyRef<PySearchParameters>, py: Python<'py>) -> PyResult<&'py PyList> {
        let pyresults = PyList::empty(py);
        let results = self.model.find_variants(input, &params.data);
        for result in results {
            let dict = self.variantresult_to_dict(&result, params.data.freq_weight, py)?;
            pyresults.append(dict)?;
        }
        Ok(pyresults)
    }

    /// Find variants in the vocabulary for all multiple string items at once, provided in in the input list. Returns a list of variants with scores and their source lexicons. Will use parallellisation under the hood.
    fn find_variants_par<'py>(&self, input: Vec<&str>, params: PyRef<PySearchParameters>, py: Python<'py>) -> PyResult<&'py PyList> {
        let params_data = &params.data;
        let output: Vec<(&str,Vec<libanaliticcl::VariantResult>)> = input
            .par_iter()
            .map(|input_str| {
                (*input_str, self.model.find_variants(input_str, params_data))
            }).collect();
        let results = PyList::empty(py);
        for (input_str, variants) in output {
            let odict = PyDict::new(py);
            let olist = PyList::empty(py);
            odict.set_item("input", input_str)?;
            for result in variants {
                let dict = self.variantresult_to_dict(&result, params.data.freq_weight, py)?;
                olist.append(dict)?;
            }
            odict.set_item("variants", olist)?;
            results.append(odict)?;
        }
        Ok(results)
    }

    ///Searches a text and returns all highest-ranking variants found in the text
    fn find_all_matches<'py>(&self, text: &str, params: PyRef<PySearchParameters>, py: Python<'py>) -> PyResult<&'py PyList> {
        let params_data = &params.data;
        let matches = self.model.find_all_matches(text, params_data);
        let results = PyList::empty(py);
        for m in matches {
            let odict = PyDict::new(py);
            odict.set_item("input", m.text)?;
            let offsetdict = PyDict::new(py);
            offsetdict.set_item("begin", m.offset.begin)?;
            offsetdict.set_item("end", m.offset.end)?;
            odict.set_item("offset", offsetdict)?;
            let olist = PyList::empty(py);
            if let Some(variants) = m.variants {
                if let Some(selected) = m.selected {
                    if let Some(result) = variants.get(selected) {
                        let dict = self.variantresult_to_dict(&result, params.data.freq_weight, py)?;
                        olist.append(dict)?;
                    }
                }
                for (i, result) in variants.iter().enumerate() {
                    if m.selected.is_none() || m.selected.unwrap() != i { //output all others
                        let dict = self.variantresult_to_dict(&result, params.data.freq_weight, py)?;
                        olist.append(dict)?;
                    }
                }
            }
            odict.set_item("variants", olist)?;
            results.append(odict)?;
        }
        Ok(results)
    }
}


#[pymodule]
fn analiticcl(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyWeights>()?;
    m.add_class::<PySearchParameters>()?;
    m.add_class::<PyVocabParams>()?;
    m.add_class::<PyVariantModel>()?;
    Ok(())
}

