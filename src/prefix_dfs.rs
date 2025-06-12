use std::{borrow::Cow, convert::Infallible};

use general_sam::{BTreeTransTable, TravelEvent, Trie, TrieNodeAlike};

use crate::{SortedTokenRange, TokenId};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "pyo3", ::pyo3::pyclass(get_all, set_all))]
pub struct TokenSeqTrieNode {
    pub parent: usize,
    pub subtree_lower: usize,
    pub subtree_upper: usize,
    pub token: TokenId,
    pub pred_range: Option<SortedTokenRange>,
}

#[cfg(feature = "pyo3")]
mod _pyo3 {
    use pyo3::pymethods;

    use super::TokenSeqTrieNode;

    #[pymethods]
    impl TokenSeqTrieNode {
        fn __repr__(&self) -> String {
            let Self {
                parent,
                subtree_lower,
                subtree_upper,
                token,
                pred_range,
            } = self;
            let pred_range = pred_range
                .as_ref()
                .map(|r| r.repr_py())
                .unwrap_or("None".to_owned());
            format!(
                "TokenSeqTrieNode(\
                token={token}, \
                pred_range={pred_range}, \
                parent={parent}, \
                subtree_lower={subtree_lower}, \
                subtree_upper={subtree_upper})",
            )
        }
    }
}

#[derive(Debug)]
pub struct TokenSeqInput<'a> {
    pub tokens: Cow<'a, [TokenId]>,
    pub pred_range: SortedTokenRange,
}

pub fn dfs_token_seq_trie(inputs: Vec<TokenSeqInput>) -> Vec<TokenSeqTrieNode> {
    let (trie, seq_last_trie_node_ids) = {
        let mut trie = Trie::<BTreeTransTable<_>>::default();
        let mut seq_last_node_ids = vec![0; inputs.len()];
        for (i, seq) in inputs.iter().enumerate() {
            seq_last_node_ids[i] = trie.insert(seq.tokens.iter().copied());
        }
        (trie, seq_last_node_ids)
    };

    let mut dfs_order = Vec::new();
    let mut rank = vec![None; trie.num_of_nodes()];
    let res = trie
        .get_root_state()
        .dfs_travel(|event| -> Result<(), Infallible> {
            match event {
                TravelEvent::PushRoot(_) => {}
                TravelEvent::Push(node, _, token) => {
                    let dfs_order_id = dfs_order.len();
                    rank[node.node_id] = Some(dfs_order_id);
                    let parent = node
                        .get_node()
                        .and_then(|n| rank[n.get_parent()])
                        .unwrap_or(dfs_order_id);
                    dfs_order.push(TokenSeqTrieNode {
                        parent,
                        subtree_lower: dfs_order_id,
                        subtree_upper: dfs_order_id,
                        token,
                        pred_range: None,
                    });
                }
                TravelEvent::Pop(node, _) => {
                    if let Some(id) = rank[node.node_id] {
                        if let Some(parent) = node.get_node().and_then(|n| rank[n.get_parent()]) {
                            dfs_order[parent].subtree_upper = dfs_order[id].subtree_upper;
                        }
                    }
                }
            }
            Ok(())
        });
    match res {
        Ok(()) => {}
        Err(e) => match e {},
    }

    for (input, node_id) in inputs.into_iter().zip(seq_last_trie_node_ids) {
        if let Some(id) = rank[node_id] {
            dfs_order[id].pred_range = Some(input.pred_range);
        }
    }

    #[cfg(debug_assertions)]
    for (i, node) in dfs_order.iter().enumerate() {
        debug_assert!(node.subtree_lower <= node.subtree_upper);
        if node.parent < node.subtree_lower {
            debug_assert!(node.parent != i);
            let parent = &dfs_order[node.parent];
            debug_assert!(parent.subtree_lower < node.subtree_lower);
            debug_assert!(parent.subtree_upper >= node.subtree_lower);
        }
    }

    dfs_order
}
