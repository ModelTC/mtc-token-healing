use std::{borrow::Cow, convert::Infallible};

use derive_more::{From, Into};
use general_sam::{BTreeTransTable, TravelEvent, Trie, TrieNodeAlike};
use itertools::{Itertools, multiunzip};
use pyo3::{Py, PyAny, PyErr, PyResult, Python, pyclass, pyfunction, pymethods};

use crate::TokenId;

#[derive(Debug, Into)]
#[pyclass(get_all, set_all, generic)]
pub struct TokenSeqTrieNode {
    pub parent: usize,
    pub subtree_lower: usize,
    pub subtree_upper: usize,
    pub depth: usize,
    pub num_children: usize,
    pub token: TokenId,
    pub value: Option<Py<PyAny>>,
}

#[pymethods]
impl TokenSeqTrieNode {
    fn __repr__<'py>(&self, py: Python<'py>) -> PyResult<String> {
        let Self {
            parent,
            subtree_lower,
            subtree_upper,
            depth,
            num_children,
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
                num_children={num_children}, \
                value={value})",
        ))
    }
}

#[derive(Debug, From)]
#[pyclass(get_all, set_all, generic)]
pub struct TokenSeqTrie {
    pub parents: Vec<usize>,
    pub subtree_lower_seq: Vec<usize>,
    pub subtree_upper_seq: Vec<usize>,
    pub depths: Vec<usize>,
    pub num_children_seq: Vec<usize>,
    pub tokens: Vec<TokenId>,
    pub values: Vec<Option<Py<PyAny>>>,
}

#[pymethods]
impl TokenSeqTrie {
    fn __repr__<'py>(&self, py: Python<'py>) -> PyResult<String> {
        let Self {
            parents,
            subtree_lower_seq,
            subtree_upper_seq,
            depths,
            num_children_seq,
            tokens,
            values,
        } = self;
        let values = values.iter().map(|opt| {
            opt.as_ref()
                .map(|v| {
                    Ok::<_, PyErr>(Cow::Owned(
                        v.call_method0(py, "__repr__")?.extract::<String>(py)?,
                    ))
                })
                .unwrap_or(Ok(Cow::Borrowed("None")))
        });
        let values_repr = Itertools::intersperse_with(values, || Ok(Cow::Borrowed(", ")))
            .collect::<Result<String, _>>()?;
        Ok(format!(
            "TokenSeqTrie(\
                tokens={tokens:?}, \
                parents={parents:?}, \
                subtree_lower_seq={subtree_lower_seq:?}, \
                subtree_upper_seq={subtree_upper_seq:?}, \
                depths={depths:?}, \
                num_children_seq={num_children_seq:?}, \
                values=[{values_repr}])",
        ))
    }

    fn __len__(&self) -> usize {
        self.parents.len()
    }
}

#[derive(Debug)]
struct TokenSeqInput<'a> {
    pub tokens: Cow<'a, [TokenId]>,
    pub value: Option<Py<PyAny>>,
}

fn dfs_token_seq_trie(inputs: Vec<TokenSeqInput>) -> Vec<TokenSeqTrieNode> {
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
                        num_children: 0,
                        token,
                        value: None,
                    });
                }
                TravelEvent::Pop(node, _) => {
                    if let Some(id) = rank[node.node_id]
                        && let Some(parent) = node.get_node().and_then(|n| rank[n.get_parent()])
                    {
                        dfs_order[parent].subtree_upper = dfs_order[id].subtree_upper;
                        dfs_order[parent].num_children += 1;
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

#[pyfunction(name = "dfs_token_seq_trie_as_nodes")]
pub fn dfs_token_seq_trie_py<'py>(
    py: Python<'py>,
    inputs: Vec<(Vec<TokenId>, Option<Py<PyAny>>)>,
) -> (Vec<TokenSeqTrieNode>, usize) {
    debug_assert!(
        inputs
            .iter()
            .all(|(_, o)| o.as_ref().is_none_or(|v| !v.is_none(py)))
    );

    py.detach(|| {
        let inputs = inputs
            .into_iter()
            .map(|(s, v)| TokenSeqInput {
                tokens: Cow::Owned(s),
                value: v,
            })
            .collect();

        let nodes = dfs_token_seq_trie(inputs);

        let prefill_chain_len = {
            let mut res = 0;
            while res < nodes.len() {
                let node = &nodes[res];
                if node.num_children == 1 && node.value.is_none() {
                    res += 1;
                    continue;
                }
                break;
            }
            res
        };

        (nodes, prefill_chain_len)
    })
}

#[pyfunction(name = "dfs_token_seq_trie")]
pub fn dfs_token_seq_trie_soa_py<'py>(
    py: Python<'py>,
    sequences: Vec<Vec<TokenId>>,
    values: Vec<Option<Py<PyAny>>>,
) -> (TokenSeqTrie, usize) {
    let (res, prefill_chain_len) =
        dfs_token_seq_trie_py(py, sequences.into_iter().zip(values).collect());
    py.detach(|| {
        let soa = TokenSeqTrie::from(multiunzip(
            res.into_iter().map(Into::<(_, _, _, _, _, _, _)>::into),
        ));
        (soa, prefill_chain_len)
    })
}
