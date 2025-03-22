use ::mtc_token_healing::{
    BestChoice, CountInfo, InferRequest, InferResponse, Prediction, ReorderedTokenId, SearchTree,
    vocab::PyVocabPrefixAutomaton,
};
use pyo3::prelude::*;

#[pymodule]
fn mtc_token_healing(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<BestChoice>()?;
    m.add_class::<CountInfo>()?;
    m.add_class::<InferRequest>()?;
    m.add_class::<InferResponse>()?;
    m.add_class::<Prediction>()?;
    m.add_class::<PyVocabPrefixAutomaton>()?;
    m.add_class::<ReorderedTokenId>()?;
    m.add_class::<SearchTree>()?;
    Ok(())
}
