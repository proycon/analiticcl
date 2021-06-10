extern crate analiticcl as libanaliticcl;

use rayon::prelude::*;
use pyo3::prelude::*;
use pyo3::types::*;
use pyo3::exceptions::PyRuntimeError;
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
                        "freq" => if let Ok(Some(value)) = value.extract() {
                            instance.weights.freq = value
                         },
                        "prefix" => if let Ok(Some(value)) = value.extract() {
                            instance.weights.prefix = value
                         },
                        "suffix" => if let Ok(Some(value)) = value.extract() {
                            instance.weights.suffix = value
                         },
                        "lex" => if let Ok(Some(value)) = value.extract() {
                            instance.weights.lex = value
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
    fn get_freq(&self) -> PyResult<f64> { Ok(self.weights.freq) }
    #[getter]
    fn get_prefix(&self) -> PyResult<f64> { Ok(self.weights.prefix) }
    #[getter]
    fn get_suffix(&self) -> PyResult<f64> { Ok(self.weights.suffix) }
    #[getter]
    fn get_lex(&self) -> PyResult<f64> { Ok(self.weights.lex) }
    #[getter]
    fn get_case(&self) -> PyResult<f64> { Ok(self.weights.case) }

    #[setter]
    fn set_ld(&mut self, value: f64) -> PyResult<()> { self.weights.ld = value; Ok(()) }
    #[setter]
    fn set_lcs(&mut self, value: f64) -> PyResult<()> { self.weights.lcs = value; Ok(()) }
    #[setter]
    fn set_freq(&mut self, value: f64) -> PyResult<()> { self.weights.freq = value; Ok(()) }
    #[setter]
    fn set_prefix(&mut self, value: f64) -> PyResult<()> { self.weights.prefix = value; Ok(()) }
    #[setter]
    fn set_suffix(&mut self, value: f64) -> PyResult<()> { self.weights.suffix = value; Ok(()) }
    #[setter]
    fn set_lex(&mut self, value: f64) -> PyResult<()> { self.weights.lex = value; Ok(()) }
    #[setter]
    fn set_case(&mut self, value: f64) -> PyResult<()> { self.weights.case = value; Ok(()) }
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
                        "max_anagram_distance" => if let Ok(Some(value)) = value.extract() {
                            instance.data.max_anagram_distance = value
                         },
                        "max_edit_distance" => if let Ok(Some(value)) = value.extract() {
                            instance.data.max_edit_distance = value
                         },
                        "max_matches" => if let Ok(Some(value)) = value.extract() {
                            instance.data.max_matches = value
                         },
                        "score_threshold" => if let Ok(Some(value)) = value.extract() {
                            instance.data.score_threshold = value
                         },
                        "max_ngram" => if let Ok(Some(value)) = value.extract() {
                            instance.data.max_ngram = value
                         },
                        "max_seq" => if let Ok(Some(value)) = value.extract() {
                            instance.data.max_seq = value
                         },
                        "single_thread" => if let Ok(Some(value)) = value.extract() {
                            instance.data.single_thread = value
                         },
                        "lm_weight" => if let Ok(Some(value)) = value.extract() {
                            instance.data.lm_weight = value
                         },
                        "variantmodel_weight" => if let Ok(Some(value)) = value.extract() {
                            instance.data.variantmodel_weight = value
                         },
                        _ => eprintln!("Ignored unknown kwargs option {}", key),
                    }
                }
            }
        }
        instance
    }

    #[getter]
    fn get_max_anagram_distance(&self) -> PyResult<u8> { Ok(self.data.max_anagram_distance) }
    #[getter]
    fn get_max_edit_distance(&self) -> PyResult<u8> { Ok(self.data.max_edit_distance) }
    #[getter]
    fn get_max_matches(&self) -> PyResult<usize> { Ok(self.data.max_matches) }
    #[getter]
    fn get_score_threshold(&self) -> PyResult<f64> { Ok(self.data.score_threshold) }
    #[getter]
    fn get_max_ngram(&self) -> PyResult<u8> { Ok(self.data.max_ngram) }
    #[getter]
    fn get_max_seq(&self) -> PyResult<usize> { Ok(self.data.max_seq) }
    #[getter]
    fn get_single_thread(&self) -> PyResult<bool> { Ok(self.data.single_thread) }
    #[getter]
    fn get_lm_weight(&self) -> PyResult<f32> { Ok(self.data.lm_weight) }
    #[getter]
    fn get_variantmodel_weight(&self) -> PyResult<f32> { Ok(self.data.variantmodel_weight) }

    #[setter]
    fn set_max_anagram_distance(&mut self, value: u8) -> PyResult<()> { self.data.max_anagram_distance = value; Ok(()) }
    #[setter]
    fn set_max_edit_distance(&mut self, value: u8) -> PyResult<()> { self.data.max_edit_distance = value; Ok(()) }
    #[setter]
    fn set_max_matches(&mut self, value: usize) -> PyResult<()> { self.data.max_matches = value; Ok(()) }
    #[setter]
    fn set_max_ngram(&mut self, value: u8) -> PyResult<()> { self.data.max_ngram = value; Ok(()) }
    #[setter]
    fn set_max_seq(&mut self, value: usize) -> PyResult<()> { self.data.max_seq = value; Ok(()) }
    #[setter]
    fn set_single_thread(&mut self, value: bool) -> PyResult<()> { self.data.single_thread = value; Ok(()) }
    #[setter]
    fn set_lm_weight(&mut self, value: f32) -> PyResult<()> { self.data.lm_weight = value; Ok(()) }
    #[setter]
    fn set_variantmodel_weight(&mut self, value: f32) -> PyResult<()> { self.data.variantmodel_weight = value; Ok(()) }

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
                        "weight" => if let Ok(Some(value)) = value.extract() {
                            instance.data.weight = value
                         },
                        "index" => if let Ok(Some(value)) = value.extract() {
                            instance.data.index = value
                         },
                        _ => eprintln!("Ignored unknown kwargs option {}", key),
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
    fn get_weight(&self) -> PyResult<f32> { Ok(self.data.weight) }
    #[getter]
    fn get_index(&self) -> PyResult<u8> { Ok(self.data.index) }

    #[setter]
    fn set_text_column(&mut self, value: u8) -> PyResult<()> { self.data.text_column = value; Ok(()) }
    #[setter]
    fn set_freq_column(&mut self, value: Option<u8>) -> PyResult<()> { self.data.freq_column = value; Ok(()) }
    #[setter]
    fn set_weight(&mut self, value: f32) -> PyResult<()> { self.data.weight = value; Ok(()) }
    #[setter]
    fn set_index(&mut self, value: u8) -> PyResult<()> { self.data.index = value; Ok(()) }

    fn with_freqhandling_sum(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::Sum; }
    fn with_freqhandling_max(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::Max; }
    fn with_freqhandling_min(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::Min; }
    fn with_freqhandling_replace(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::Replace; }
    fn with_freqhandling_sumifmoreweight(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::SumIfMoreWeight; }
    fn with_freqhandling_maxifmoreweight(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::MaxIfMoreWeight; }
    fn with_freqhandling_minifmoreweight(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::MinIfMoreWeight; }
    fn with_freqhandling_replaceifmoreweight(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::ReplaceIfMoreWeight; }

    fn with_vocabtype_normal(&mut self) { self.data.vocab_type = libanaliticcl::VocabType::Normal}
    fn with_vocabtype_intermediate(&mut self) { self.data.vocab_type = libanaliticcl::VocabType::Intermediate}
    fn with_vocabtype_noindex(&mut self) { self.data.vocab_type = libanaliticcl::VocabType::NoIndex}
}


#[pyclass(dict,name="VariantModel")]
pub struct PyVariantModel {
    model: libanaliticcl::VariantModel,
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

    /// Higher order function to load a corpus background lexicon and make it available to the model.
    /// Wraps around read_vocabulary() with default parameters.
    fn read_corpus(&mut self, filename: &str) -> PyResult<()> {
        match self.model.read_vocabulary(filename, &libanaliticcl::VocabParams::default().with_weight(0.0)) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyRuntimeError::new_err(format!("{}", e)))
        }
    }

    /// Higher order function to load a language model and make it available to the model.
    /// Wraps around read_vocabulary() with default parameters.
    fn read_lm(&mut self, filename: &str) -> PyResult<()> {
        match self.model.read_vocabulary(filename, &libanaliticcl::VocabParams::default().with_weight(0.0).with_vocab_type(libanaliticcl::VocabType::NoIndex)) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyRuntimeError::new_err(format!("{}", e)))
        }
    }

    ///Load a variant list
    fn read_variants(&mut self, filename: &str) -> PyResult<()> {
        match self.model.read_variants(filename, Some(&libanaliticcl::VocabParams::default())) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyRuntimeError::new_err(format!("{}", e)))
        }
    }

    ///Load a weighted variant list (set intermediate to true if this is an error list and you
    ///don't want the variants to be used in matching)
    fn read_weighted_variants(&mut self, filename: &str, intermediate: bool) -> PyResult<()> {
        match self.model.read_weighted_variants(filename, Some(&libanaliticcl::VocabParams::default()), intermediate) {
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
        let result = PyList::empty(py);
        let results = self.model.find_variants(input, &params.data, None);
        for (vocab_id,score) in results {
            let dict = PyDict::new(py);
            let vocabvalue = self.model.get_vocab(vocab_id).expect("getting vocab by id");
            let lexicon = self.model.lexicons.get(vocabvalue.lexindex as usize).expect("valid lexicon index");
            dict.set_item("text", vocabvalue.text.as_str())?;
            dict.set_item("score", score)?;
            dict.set_item("lexicon", lexicon.as_str())?;
            result.append(dict)?;
        }
        Ok(result)
    }

    /// Find variants in the vocabulary for all multiple string items at once, provided in in the input list. Returns a list of variants with scores and their source lexicons. Will use parallellisation under the hood.
    fn find_variants_par<'py>(&self, input: Vec<&str>, params: PyRef<PySearchParameters>, py: Python<'py>) -> PyResult<&'py PyList> {
        let params_data = &params.data;
        let output: Vec<(&str,Vec<(libanaliticcl::VocabId,f64)>)> = input
            .par_iter()
            .map(|input_str| {
                (*input_str, self.model.find_variants(input_str, params_data, None))
            }).collect();
        let results = PyList::empty(py);
        for (input_str, variants) in output {
            let odict = PyDict::new(py);
            let olist = PyList::empty(py);
            odict.set_item("input", input_str)?;
            for (vocab_id, score) in variants {
                let dict = PyDict::new(py);
                let vocabvalue = self.model.get_vocab(vocab_id).expect("getting vocab by id");
                let lexicon = self.model.lexicons.get(vocabvalue.lexindex as usize).expect("valid lexicon index");
                dict.set_item("text", vocabvalue.text.as_str())?;
                dict.set_item("score", score)?;
                dict.set_item("lexicon", lexicon.as_str())?;
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
                    if let Some((vocab_id, score)) = variants.get(selected) {
                        let dict = PyDict::new(py);
                        let vocabvalue = self.model.get_vocab(*vocab_id).expect("getting vocab by id");
                        let lexicon = self.model.lexicons.get(vocabvalue.lexindex as usize).expect("valid lexicon index");
                        dict.set_item("text", vocabvalue.text.as_str())?;
                        dict.set_item("score", score)?;
                        dict.set_item("lexicon", lexicon.as_str())?;
                        olist.append(dict)?;
                    }
                }
                for (i, (vocab_id, score)) in variants.iter().enumerate() {
                    if m.selected.is_none() || m.selected.unwrap() != i { //output all others
                        let dict = PyDict::new(py);
                        let vocabvalue = self.model.get_vocab(*vocab_id).expect("getting vocab by id");
                        let lexicon = self.model.lexicons.get(vocabvalue.lexindex as usize).expect("valid lexicon index");
                        dict.set_item("text", vocabvalue.text.as_str())?;
                        dict.set_item("score", score)?;
                        dict.set_item("lexicon", lexicon.as_str())?;
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

