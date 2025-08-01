mod prefix_dfs;

use ::mtc_token_healing::{SortedTokenRange, TokenId, VocabPrefixAutomaton};
use pyo3::prelude::*;

use crate::prefix_dfs::{TokenSeqTrieNode, dfs_token_seq_trie_py};

#[pymodule]
fn mtc_token_healing(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SortedTokenRange>()?;
    m.add_class::<VocabPrefixAutomaton>()?;
    m.add_class::<TokenSeqTrieNode>()?;
    m.add_function(wrap_pyfunction!(dfs_token_seq_trie_py, m)?)?;
    Ok(())
}
