use ::mtc_token_healing::{SortedTokenRange, VocabPrefixAutomaton};
use pyo3::prelude::*;

#[pymodule]
fn mtc_token_healing(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SortedTokenRange>()?;
    m.add_class::<VocabPrefixAutomaton>()?;
    Ok(())
}
