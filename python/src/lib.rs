use std::borrow::Cow;

use ::mtc_token_healing::{
    SortedTokenRange, TokenId, TokenSeqInput, TokenSeqTrieNode, VocabPrefixAutomaton,
    dfs_token_seq_trie,
};
use pyo3::prelude::*;

#[pyfunction(name = "dfs_token_seq_trie")]
fn dfs_token_seq_trie_py(
    token_ids: Vec<Vec<TokenId>>,
    pred_rank_ranges: Vec<SortedTokenRange>,
) -> Vec<TokenSeqTrieNode> {
    let inputs = token_ids
        .into_iter()
        .zip(pred_rank_ranges)
        .map(|(s, r)| TokenSeqInput {
            tokens: Cow::Owned(s),
            pred_range: r,
        })
        .collect();
    dfs_token_seq_trie(inputs)
}

#[pymodule]
fn mtc_token_healing(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SortedTokenRange>()?;
    m.add_class::<VocabPrefixAutomaton>()?;
    m.add_class::<TokenSeqTrieNode>()?;
    m.add_function(wrap_pyfunction!(dfs_token_seq_trie_py, m)?)?;
    Ok(())
}
