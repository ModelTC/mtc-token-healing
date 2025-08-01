use std::{borrow::Cow, convert::Infallible};

use general_sam::{BTreeTransTable, TravelEvent, Trie, TrieNodeAlike};
use pyo3::{PyObject, PyResult, Python, pyclass, pyfunction, pymethods};

use crate::TokenId;

#[derive(Debug)]
#[pyclass(get_all, set_all)]
pub struct TokenSeqTrieNode {
    pub parent: usize,
    pub subtree_lower: usize,
    pub subtree_upper: usize,
    pub depth: usize,
    pub token: TokenId,
    pub value: Option<PyObject>,
}

#[pymethods]
impl TokenSeqTrieNode {
    fn __repr__<'py>(&self, py: Python<'py>) -> PyResult<String> {
        let Self {
            parent,
            subtree_lower,
            subtree_upper,
            depth,
            token,
            value,
        } = self;
        let value = value
            .as_ref()
            .map(|v| v.call_method0(py, "__repr__")?.extract::<String>(py))
            .transpose()?
            .unwrap_or("None".to_owned());
        Ok(format!(
            "TokenSeqTrieNode(\
                token={token}, \
                parent={parent}, \
                subtree_lower={subtree_lower}, \
                subtree_upper={subtree_upper}, \
                depth={depth}, \
                value={value})",
        ))
    }
}

#[derive(Debug)]
pub struct TokenSeqInput<'a> {
    pub tokens: Cow<'a, [TokenId]>,
    pub value: Option<PyObject>,
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
                        depth: 0,
                        token,
                        value: None,
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

    for i in 0..dfs_order.len() {
        let parent = dfs_order[i].parent;
        if parent == i {
            continue;
        }
        dfs_order[i].depth = dfs_order[parent].depth + 1;
    }

    for (input, node_id) in inputs.into_iter().zip(seq_last_trie_node_ids) {
        if let Some(id) = rank[node_id] {
            dfs_order[id].value = input.value;
        }
    }

    #[cfg(debug_assertions)]
    for (i, node) in dfs_order.iter().enumerate() {
        debug_assert!(node.subtree_lower <= node.subtree_upper);
        debug_assert!(node.subtree_lower == i);
        debug_assert!(node.parent <= i);
        if node.parent < node.subtree_lower {
            debug_assert!(node.parent != i);
            let parent = &dfs_order[node.parent];
            debug_assert!(parent.subtree_lower < node.subtree_lower);
            debug_assert!(parent.subtree_upper >= node.subtree_lower);
        } else {
            debug_assert!(node.parent == i);
        }
    }

    dfs_order
}

#[pyfunction(name = "dfs_token_seq_trie")]
pub fn dfs_token_seq_trie_py<'py>(
    py: Python<'py>,
    inputs: Vec<(Vec<TokenId>, Option<PyObject>)>,
) -> (Vec<TokenSeqTrieNode>, usize) {
    debug_assert!(
        inputs
            .iter()
            .all(|(_, o)| o.as_ref().is_none_or(|v| !v.is_none(py)))
    );

    py.allow_threads(|| {
        let inputs = inputs
            .into_iter()
            .map(|(s, v)| TokenSeqInput {
                tokens: Cow::Owned(s),
                value: v,
            })
            .collect();

        let nodes = dfs_token_seq_trie(inputs);

        let parent_chain_len = {
            let mut res = 0;
            while res < nodes.len() {
                let node = &nodes[res];
                if node.parent == res.saturating_sub(1) && node.value.is_none() {
                    res += 1;
                    continue;
                }
                break;
            }
            res
        };

        (nodes, parent_chain_len)
    })
}
