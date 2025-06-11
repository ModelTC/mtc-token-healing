pub type TokenId = u32;
pub type SortedTokenId = u32;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass(get_all, set_all))]
pub struct SortedTokenRange {
    pub lower: SortedTokenId,
    pub upper: SortedTokenId,
}

#[cfg(feature = "pyo3")]
mod _pyo3 {
    use pyo3::pymethods;

    use super::SortedTokenRange;

    #[pymethods]
    impl SortedTokenRange {
        fn __repr__(&self) -> String {
            let Self { lower, upper } = self;
            format!("SortedTokenRange(lower={}, upper={})", lower, upper)
        }
    }
}
