use std::{collections::BTreeMap, future::Future, sync::Arc};

use general_sam::{BTreeTransTable, TRIE_ROOT_NODE_ID, Trie, TrieNodeID};

use crate::{BestChoice, CountInfo, ReorderedTokenId, TokenId, VocabPrefixAutomaton};

#[derive(Debug, thiserror::Error)]
pub enum SearchTreeError {
    #[error("feed infer results to an empty stack?")]
    EmptyStack,
    #[error("invalid sparse choices {choices:?}, expecting {expected:?}")]
    InvalidSparseChoices {
        choices: Vec<Prediction>,
        expected: Vec<ReorderedTokenId>,
    },
    #[error("no sampled result, while expecting one")]
    NoSampledResult,
    #[error("invalid sampled result, {result:?} not in id range [{lower:?}, {upper:?})")]
    InvalidSampledResult {
        result: Prediction,
        lower: ReorderedTokenId,
        upper: ReorderedTokenId,
    },
    #[error("no best choice can be found (neg inf log probs?)")]
    NoBestChoice,
}

pub type SearchTreeResult<T> = Result<T, SearchTreeError>;

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass(get_all, set_all))]
pub struct Prediction {
    pub token_id: ReorderedTokenId,
    pub log_prob: f64,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass(get_all, frozen))]
pub struct InferRequest {
    pub backtrace: usize,
    pub feed: Option<TokenId>,
    pub sampling_id_range: Option<(ReorderedTokenId, ReorderedTokenId)>,
    pub sparse_choices: Vec<ReorderedTokenId>,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass(get_all, set_all))]
pub struct InferResponse {
    pub sampled: Option<Prediction>,
    pub sparse_choices: Vec<Prediction>,
}

#[derive(Debug)]
struct SearchState {
    log_prob: f64,
    sampling_id_range: Option<(ReorderedTokenId, ReorderedTokenId)>,
    next_choices: Vec<Prediction>,
    next_states: Vec<(ReorderedTokenId, TrieNodeID)>,
}

#[derive(Debug)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass)]
pub struct SearchTree {
    automaton: Arc<VocabPrefixAutomaton>,

    max_num_tokens: usize,

    trie: Trie<BTreeTransTable<ReorderedTokenId>>,
    sampling_id_range: BTreeMap<TrieNodeID, (ReorderedTokenId, ReorderedTokenId)>,

    stack: Vec<SearchState>,

    prefilled_token_ids: Vec<TokenId>,

    current_new_token_ids: Vec<TokenId>,
    current_accum_log_prob: f64,

    best_choice: BestChoice,
}

impl SearchTree {
    pub fn prefilled_token_ids(&self) -> &[TokenId] {
        &self.prefilled_token_ids
    }

    pub fn get_best_choice(&self) -> SearchTreeResult<&BestChoice> {
        if self.best_choice.valid() {
            Ok(&self.best_choice)
        } else {
            Err(SearchTreeError::NoSampledResult)
        }
    }

    pub async fn new<Ids, Seq, Fut, E, F, S: AsRef<str>>(
        automaton: Arc<VocabPrefixAutomaton>,
        tokenize_for_multiple_ending_positions: F,
        text: S,
        start_from: usize,
    ) -> Result<Option<(Self, InferRequest)>, E>
    where
        F: FnOnce(Vec<usize>) -> Fut,
        Fut: Future<Output = Result<Seq, E>> + Send,
        Seq: IntoIterator<Item = (usize, Ids)>,
        Ids: IntoIterator<Item = TokenId>,
    {
        let pos_to_cnt_info = BTreeMap::from_iter(automaton.as_ref().parse_chars(text, start_from));
        let end_pos = Vec::from_iter(pos_to_cnt_info.keys().copied());
        let encoded = tokenize_for_multiple_ending_positions(end_pos).await?;
        Ok(Self::from_encoded(automaton, pos_to_cnt_info, encoded))
    }

    pub fn new_sync<Ids, Seq, E, F, S: AsRef<str>>(
        automaton: Arc<VocabPrefixAutomaton>,
        tokenize_for_multiple_ending_positions: F,
        text: S,
        start_from: usize,
    ) -> Result<Option<(Self, InferRequest)>, E>
    where
        F: FnOnce(Vec<usize>) -> Result<Seq, E>,
        Seq: IntoIterator<Item = (usize, Ids)>,
        Ids: IntoIterator<Item = TokenId>,
    {
        let pos_to_cnt_info = BTreeMap::from_iter(automaton.as_ref().parse_chars(text, start_from));
        let end_pos = Vec::from_iter(pos_to_cnt_info.keys().copied());
        let encoded = tokenize_for_multiple_ending_positions(end_pos)?;
        Ok(Self::from_encoded(automaton, pos_to_cnt_info, encoded))
    }

    pub fn from_encoded<
        Seq: IntoIterator<Item = (usize, Ids)>,
        Ids: IntoIterator<Item = TokenId>,
    >(
        automaton: Arc<VocabPrefixAutomaton>,
        pos_to_cnt_info: BTreeMap<usize, CountInfo>,
        encoded: Seq,
    ) -> Option<(Self, InferRequest)> {
        let mut tree = SearchTree {
            automaton: automaton.clone(),

            max_num_tokens: 0,

            trie: Default::default(),
            sampling_id_range: Default::default(),

            stack: Default::default(),

            prefilled_token_ids: Default::default(),

            current_new_token_ids: Default::default(),
            current_accum_log_prob: 0.0,

            best_choice: Default::default(),
        };

        encoded.into_iter().for_each(|(pos, ids)| {
            let Some(cnt_info) = pos_to_cnt_info.get(&pos) else {
                return;
            };
            let mut num_tokens = 0;
            let node_id = tree.trie.insert(ids.into_iter().map(|x| {
                num_tokens += 1;
                automaton.rank()[x as usize]
            }));
            tree.max_num_tokens = tree.max_num_tokens.max(num_tokens);
            tree.sampling_id_range.insert(
                node_id,
                (
                    ReorderedTokenId(cnt_info.tot_cnt_lower as _),
                    ReorderedTokenId(cnt_info.tot_cnt_upper as _),
                ),
            );
        });

        let mut node_id = TRIE_ROOT_NODE_ID;
        while !tree.sampling_id_range.contains_key(&node_id) {
            let Some(node) = tree.trie.get_node(node_id) else {
                break;
            };
            if node.get_trans().len() > 1 {
                break;
            }
            let Some((&token_id, &next_node_id)) = node.get_trans().first_key_value() else {
                break;
            };
            tree.prefilled_token_ids
                .push(automaton.order()[token_id.0 as usize]);
            node_id = next_node_id;
        }

        let node = tree.trie.get_node(node_id)?;
        let sampling_id_range = tree.sampling_id_range.get(&node_id).copied();
        if node.get_trans().is_empty() && sampling_id_range.is_none() {
            return None;
        }

        let next_states = Vec::from_iter(node.get_trans().iter().map(|(&u, &v)| (u, v)));
        let next_token_ids = next_states.iter().map(|i| i.0).collect();
        tree.stack.push(SearchState {
            log_prob: 0.0,
            sampling_id_range,
            next_choices: Default::default(),
            next_states,
        });
        let request = InferRequest {
            backtrace: 0,
            feed: None,
            sampling_id_range,
            sparse_choices: next_token_ids,
        };

        Some((tree, request))
    }

    pub fn feed(&mut self, res: InferResponse) -> SearchTreeResult<Option<InferRequest>> {
        let Some(top) = self.stack.last_mut() else {
            return Err(SearchTreeError::EmptyStack);
        };

        if let Some((lower, upper)) = top.sampling_id_range.take() {
            let Some(sampled) = res.sampled else {
                return Err(SearchTreeError::NoSampledResult);
            };
            if sampled.token_id < lower || sampled.token_id >= upper {
                return Err(SearchTreeError::InvalidSampledResult {
                    result: sampled,
                    lower,
                    upper,
                });
            }
            self.current_new_token_ids
                .push(self.automaton.order()[sampled.token_id.0 as usize]);
            self.best_choice.update(
                self.current_new_token_ids.iter().copied(),
                self.current_accum_log_prob + sampled.log_prob,
            );
            self.current_new_token_ids.pop();
        }

        if top.next_choices.len() != top.next_states.len() {
            debug_assert!(top.next_choices.is_empty());

            let expected_token_ids = Vec::from_iter(top.next_states.iter().map(|i| i.0));
            if res.sparse_choices.len() != expected_token_ids.len()
                || expected_token_ids
                    .iter()
                    .zip(res.sparse_choices.iter())
                    .any(|(&i, j)| i != j.token_id)
            {
                return Err(SearchTreeError::InvalidSparseChoices {
                    choices: res.sparse_choices,
                    expected: expected_token_ids,
                });
            }

            top.next_choices = res.sparse_choices;
        }

        let mut backtrace = 0;
        while self
            .stack
            .last()
            .is_some_and(|top| top.next_choices.is_empty())
        {
            let res = self.stack.pop().unwrap();
            self.current_accum_log_prob -= res.log_prob;
            self.current_new_token_ids.pop();
            backtrace += 1;
        }

        let Some(top) = self.stack.last_mut() else {
            return Ok(None);
        };

        let prediction = top.next_choices.pop().unwrap();
        let (token_id, node_id) = top.next_states.pop().unwrap();

        let node = self.trie.get_node(node_id).unwrap();

        let sampling_id_range = self.sampling_id_range.get(&node_id).copied();

        let next_states = Vec::from_iter(node.get_trans().iter().map(|(&u, &v)| (u, v)));
        let next_token_ids = next_states.iter().map(|i| i.0).collect();

        self.stack.push(SearchState {
            log_prob: prediction.log_prob,
            sampling_id_range,
            next_choices: Default::default(),
            next_states,
        });

        let token_id = self.automaton.order()[token_id.0 as usize];
        self.current_new_token_ids.push(token_id);
        self.current_accum_log_prob += prediction.log_prob;

        let request = InferRequest {
            backtrace,
            feed: Some(token_id),
            sampling_id_range,
            sparse_choices: next_token_ids,
        };

        Ok(Some(request))
    }

    pub fn max_num_tokens(&self) -> usize {
        self.max_num_tokens
    }
}

#[cfg(feature = "pyo3")]
mod _pyo3 {
    use std::collections::BTreeMap;

    use pyo3::{
        Bound, PyErr, PyObject, PyRefMut, PyResult, Python, exceptions::PyValueError, pymethods,
        types::PyType,
    };

    use crate::{
        BestChoice, InferRequest, InferResponse, Prediction, ReorderedTokenId, SearchTree,
        SearchTreeError, TokenId, vocab::PyVocabPrefixAutomaton,
    };

    use super::SearchTreeResult;

    impl From<SearchTreeError> for PyErr {
        fn from(value: SearchTreeError) -> Self {
            PyValueError::new_err(value.to_string())
        }
    }

    #[pymethods]
    impl Prediction {
        #[new]
        pub fn new_py(token_id: ReorderedTokenId, log_prob: f64) -> Self {
            Self { token_id, log_prob }
        }
    }

    #[pymethods]
    impl InferResponse {
        #[new]
        #[pyo3(signature = (sampled=None, sparse_choices=None))]
        pub fn new_py(
            sampled: Option<Prediction>,
            sparse_choices: Option<Vec<Prediction>>,
        ) -> Self {
            Self {
                sampled,
                sparse_choices: sparse_choices.unwrap_or_default(),
            }
        }
    }

    #[pymethods]
    impl SearchTree {
        #[pyo3(name = "get_prefilled_token_ids")]
        pub fn prefilled_token_ids_py(&self) -> Vec<TokenId> {
            self.prefilled_token_ids.clone()
        }

        #[pyo3(name = "get_best_choice")]
        pub fn get_best_choice_py(&self) -> SearchTreeResult<BestChoice> {
            if self.best_choice.valid() {
                Ok(self.best_choice.clone())
            } else {
                Err(SearchTreeError::NoSampledResult)
            }
        }

        #[classmethod]
        #[pyo3(name = "new")]
        pub fn new_py(
            _cls: &Bound<'_, PyType>,
            py: Python<'_>,
            automaton: &'_ PyVocabPrefixAutomaton,
            tokenize_for_multiple_ending_positions: PyObject,
            text: &str,
            start_from: usize,
        ) -> PyResult<Option<(Self, InferRequest)>> {
            let pos_to_cnt_info =
                BTreeMap::from_iter(automaton.as_ref().parse_chars(text, start_from));

            let end_pos = Vec::from_iter(pos_to_cnt_info.keys().copied());

            let encoded: Vec<(usize, Vec<TokenId>)> = tokenize_for_multiple_ending_positions
                .call1(py, (end_pos,))?
                .extract(py)?;

            Ok(Self::from_encoded(
                automaton.0.clone(),
                pos_to_cnt_info,
                encoded,
            ))
        }

        #[pyo3(name = "feed")]
        pub fn feed_py(
            mut self_: PyRefMut<'_, Self>,
            res: InferResponse,
        ) -> SearchTreeResult<Option<InferRequest>> {
            self_.feed(res)
        }

        #[getter("max_num_tokens")]
        pub fn max_num_tokens_py(&self) -> usize {
            self.max_num_tokens
        }
    }
}
