use std::borrow::Cow;

use ::mtc_token_healing::{
    dfs_token_seq_trie, SortedTokenRange, TokenId, TokenSeqInput, TokenSeqTrieNode,
    VocabPrefixAutomaton,
};
use pyo3::prelude::*;

#[pyfunction(name = "dfs_token_seq_trie")]
fn dfs_token_seq_trie_py(
    token_ids: Vec<Vec<TokenId>>,
    pred_rank_ranges: Vec<SortedTokenRange>,
) -> (Vec<TokenSeqTrieNode>, usize) {
    let inputs = token_ids
        .into_iter()
        .zip(pred_rank_ranges)
        .map(|(s, r)| TokenSeqInput {
            tokens: Cow::Owned(s),
            pred_range: r,
        })
        .collect();
    let nodes = dfs_token_seq_trie(inputs);
    let parent_chain_len = {
        let mut res = 0;
        while res < nodes.len() {
            let node = &nodes[res];
            if node.parent == res.saturating_sub(1) && node.pred_range.is_none() {
                res += 1;
                continue;
            }
            break;
        }
        res
    };
    (nodes, parent_chain_len)
}

#[pymodule]
fn mtc_token_healing(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SortedTokenRange>()?;
    m.add_class::<VocabPrefixAutomaton>()?;
    m.add_class::<TokenSeqTrieNode>()?;
    m.add_function(wrap_pyfunction!(dfs_token_seq_trie_py, m)?)?;
    Ok(())
}
