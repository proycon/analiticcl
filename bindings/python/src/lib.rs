extern crate analiticcl as libanaliticcl;

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::wrap_pymodule;


#[pyclass(dict,name="Weights")]
#[derive(Default,Clone)]
pub struct PyWeights {
    weights: libanaliticcl::Weights
}

#[pymethods]
impl PyWeights {
    #[new]
    fn new() -> Self {
        Self::default()
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
    fn new() -> Self {
        Self::default()
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
    fn new() -> Self {
        Self::default()
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

    fn freqhandling_sum(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::Sum; }
    fn freqhandling_max(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::Max; }
    fn freqhandling_min(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::Min; }
    fn freqhandling_replace(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::Replace; }
    fn freqhandling_sumifmoreweight(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::SumIfMoreWeight; }
    fn freqhandling_maxifmoreweight(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::MaxIfMoreWeight; }
    fn freqhandling_minifmoreweight(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::MinIfMoreWeight; }
    fn freqhandling_replaceifmoreweight(&mut self) { self.data.freq_handling = libanaliticcl::FrequencyHandling::ReplaceIfMoreWeight; }

    fn vocabtype_normal(&mut self) { self.data.vocab_type = libanaliticcl::VocabType::Normal}
    fn vocabtype_intermediate(&mut self) { self.data.vocab_type = libanaliticcl::VocabType::Intermediate}
    fn vocabtype_noindex(&mut self) { self.data.vocab_type = libanaliticcl::VocabType::NoIndex}
}


#[pyclass(dict,name="VariantModel")]
pub struct PyVariantModel {
    model: libanaliticcl::VariantModel,
}

#[pymethods]
impl PyVariantModel {
    #[new]
    fn __new__(alphabet_file: &str, weights: PyRef<PyWeights>, debug: bool) -> Self {
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


    ///Read vocabulary (a lexicon or corpus-derived lexicon) from a TSV file
    ///May contain frequency information
    ///The parameters define what value can be read from what column
    fn read_vocabulary(&mut self, filename: &str, params: PyRef<PyVocabParams>) -> PyResult<()> {
        match self.model.read_vocabulary(filename, &params.data) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyRuntimeError::new_err(format!("{}", e)))
        }
    }

    fn __contains__(&self, text: &str) -> bool {
        self.model.has(text)
    }

    /// Find variants in the vocabulary for a given string (in its totality), returns a list of variants and score tuples
    fn find_variants(&self, input: &str, params: PyRef<PySearchParameters>) -> Vec<(&str, f64)> {
        self.model.find_variants(input, &params.data, None).iter().map(|(vocab_id,score)| {
                let vocabvalue = self.model.get_vocab(*vocab_id).expect("getting vocab by id");
                (vocabvalue.text.as_str(), *score)
        }).collect()
    }
}


#[pymodule]
fn analiticcl(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyWeights>()?;
    m.add_class::<PySearchParameters>()?;
    m.add_class::<PyVariantModel>()?;
    Ok(())
}

