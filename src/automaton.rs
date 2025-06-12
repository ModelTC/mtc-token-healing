use compact_bytes::CompactBytes;
use general_sam::{BoxBisectTable, GeneralSam};

use crate::{
    SortedTokenId, SortedTokenRange, TokenId,
    token::{
        build_sam_of_reversed_tokens, label_rank_range_on_sam_of_rev_tokens, sort_vocab_with_trie,
    },
};

#[derive(Debug)]
#[cfg_attr(feature = "pyo3", ::pyo3::pyclass(frozen))]
pub struct VocabPrefixAutomaton {
    vocab: Vec<CompactBytes>,
    order: Vec<TokenId>,
    rank: Vec<SortedTokenId>,
    sam_of_rev_tokens: GeneralSam<BoxBisectTable<u8>>,
    rank_range_on_sam: Vec<Option<SortedTokenRange>>,
}

impl VocabPrefixAutomaton {
    pub fn new<T: AsRef<[u8]>, V: IntoIterator<Item = T>>(vocab: V) -> Self {
        let vocab: Vec<_> = vocab
            .into_iter()
            .map(|token| CompactBytes::new(token.as_ref()))
            .collect();
        let sort_result = sort_vocab_with_trie(vocab.iter().map(|x| x.as_slice()));
        let sam_of_rev_tokens = build_sam_of_reversed_tokens(vocab.iter().map(|x| x.as_slice()));
        let cnt_info_of_sam_rev = label_rank_range_on_sam_of_rev_tokens(
            &sam_of_rev_tokens,
            vocab
                .iter()
                .map(|x| x.as_slice())
                .zip(sort_result.rank_ranges),
        );
        Self {
            vocab,
            order: sort_result.order,
            rank: sort_result.rank,
            sam_of_rev_tokens,
            rank_range_on_sam: cnt_info_of_sam_rev,
        }
    }

    pub fn vocab(&self) -> &[CompactBytes] {
        &self.vocab
    }

    pub fn order(&self) -> &[TokenId] {
        &self.order
    }

    pub fn rank(&self) -> &[SortedTokenId] {
        &self.rank
    }

    pub fn get(&self, index: usize) -> Option<&CompactBytes> {
        self.vocab.get(index).filter(|t| !t.is_empty())
    }

    pub fn parse_bytes<B: AsRef<[u8]>>(
        &self,
        bytes: B,
        start_from: usize,
    ) -> Vec<(usize, SortedTokenRange)> {
        let bytes = bytes.as_ref();

        let mut state = self.sam_of_rev_tokens.get_root_state();
        let mut res = Vec::new();

        for (pos, byte) in bytes
            .iter()
            .enumerate()
            .rev()
            .take_while(|(pos, _)| *pos >= start_from)
        {
            state.goto(byte);
            if state.is_nil() {
                break;
            }
            if let Some(cnt_info) = self.rank_range_on_sam[state.node_id].clone() {
                res.push((pos, cnt_info));
            }
        }

        res
    }

    pub fn parse_rev_token_id_seq<S: IntoIterator<Item = usize>>(
        &self,
        rev_tokens: S,
    ) -> Vec<(CompactBytes, SortedTokenRange)> {
        let mut state = self.sam_of_rev_tokens.get_root_state();
        let mut res = Vec::new();
        let mut bytes_rev = Vec::new();

        for id in rev_tokens {
            if state.is_nil() {
                break;
            }
            let Some(token) = self.get(id) else {
                break;
            };
            for byte in token.iter().rev() {
                state.goto(byte);
                bytes_rev.push(*byte);
                if state.is_nil() {
                    break;
                }
                if let Some(cnt_info) = self.rank_range_on_sam[state.node_id].clone() {
                    let mut bytes = bytes_rev.clone();
                    bytes.reverse();
                    res.push((CompactBytes::from(bytes), cnt_info));
                }
            }
        }

        res
    }
}

#[cfg(feature = "pyo3")]
pub mod pyo3 {
    use pyo3::{Bound, Python, pymethods, types::PyBytes};

    use crate::{SortedTokenId, SortedTokenRange, TokenId};

    use super::VocabPrefixAutomaton;

    #[pymethods]
    impl VocabPrefixAutomaton {
        #[new]
        fn py_new(vocab: Vec<Vec<u8>>) -> Self {
            Self::new(vocab)
        }

        #[getter("vocab_size")]
        fn vocab_size_py(&self) -> usize {
            self.vocab.len()
        }

        #[pyo3(name = "get_order")]
        fn get_order_py(&self) -> &[TokenId] {
            &self.order
        }

        #[pyo3(name = "get_rank")]
        fn get_rank_py(&self) -> &[SortedTokenId] {
            &self.rank
        }

        #[pyo3(name = "parse_bytes")]
        fn parse_bytes_py(
            &self,
            bytes: &[u8],
            start_from: usize,
        ) -> Vec<(usize, SortedTokenRange)> {
            self.parse_bytes(bytes, start_from)
        }

        #[pyo3(name = "parse_tokens")]
        fn parse_tokens_py<'py>(
            &self,
            py: Python<'py>,
            tokens: Vec<usize>,
        ) -> Vec<(Bound<'py, PyBytes>, SortedTokenRange)> {
            self.parse_rev_token_id_seq(tokens.into_iter().rev())
                .into_iter()
                .map(|(b, c)| (PyBytes::new(py, &b), c))
                .collect()
        }
    }
}
