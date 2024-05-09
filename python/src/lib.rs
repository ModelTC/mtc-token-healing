use ::mtc_token_healing::CountInfo;
use pyo3::prelude::*;

#[pymodule]
fn mtc_token_healing(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<CountInfo>()?;
    Ok(())
}
