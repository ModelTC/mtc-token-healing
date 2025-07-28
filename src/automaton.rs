use general_sam::{BoxBisectTable, GeneralSam};

use crate::{
    SmallToken, SortedTokenId, SortedTokenRange, TokenId,
    token::{
        build_sam_of_reversed_tokens, label_rank_range_on_sam_of_rev_tokens, sort_vocab_with_trie,
    },
};

#[derive(Debug)]
#[cfg_attr(feature = "pyo3", ::pyo3::pyclass(frozen))]
pub struct VocabPrefixAutomaton {
    vocab: Vec<SmallToken>,
    order: Vec<TokenId>,
    rank: Vec<SortedTokenId>,
    sam_of_rev_tokens: GeneralSam<BoxBisectTable<u8>>,
    rank_range_on_sam: Vec<Option<SortedTokenRange>>,
}

impl VocabPrefixAutomaton {
    pub fn new<T: AsRef<[u8]>, V: IntoIterator<Item = T>>(vocab: V) -> Self {
        let vocab: Vec<_> = vocab
            .into_iter()
            .map(|token| SmallToken::from(token.as_ref()))
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

    pub fn vocab(&self) -> &[SmallToken] {
        &self.vocab
    }

    pub fn order(&self) -> &[TokenId] {
        &self.order
    }

    pub fn rank(&self) -> &[SortedTokenId] {
        &self.rank
    }

    pub fn get(&self, index: usize) -> Option<&SmallToken> {
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
    ) -> Vec<(SmallToken, SortedTokenRange)> {
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
                    res.push((SmallToken::from(bytes.as_slice()), cnt_info));
                }
            }
        }

        res
    }
}

#[cfg(feature = "pyo3")]
pub mod pyo3 {
    use pyo3::{Bound, FromPyObject, IntoPyObject, Python, pymethods, types::PyBytes};

    use crate::{SortedTokenId, SortedTokenRange, TokenId};

    use super::VocabPrefixAutomaton;

    #[derive(Debug, FromPyObject, IntoPyObject)]
    enum TokenIdSeq {
        TokenId(TokenId),
        Seq(Vec<TokenId>),
    }

    impl TokenIdSeq {
        fn map<F: FnMut(TokenId) -> TokenId>(self, mut f: F) -> Self {
            match self {
                Self::TokenId(id) => Self::TokenId(f(id)),
                Self::Seq(mut items) => {
                    items.iter_mut().for_each(|item| {
                        *item = f(*item);
                    });
                    Self::Seq(items)
                }
            }
        }
    }

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
            py: Python<'_>,
            bytes: &[u8],
            start_from: usize,
        ) -> Vec<(usize, SortedTokenRange)> {
            py.allow_threads(|| self.parse_bytes(bytes, start_from))
        }

        #[pyo3(name = "parse_tokens")]
        fn parse_tokens_py<'py>(
            &self,
            py: Python<'py>,
            tokens: Vec<usize>,
        ) -> Vec<(Bound<'py, PyBytes>, SortedTokenRange)> {
            let res = py.allow_threads(|| self.parse_rev_token_id_seq(tokens.into_iter().rev()));
            res.into_iter()
                .map(|(b, c)| (PyBytes::new(py, &b), c))
                .collect()
        }

        #[pyo3(name = "parse_tokens_str_suffix")]
        fn parse_tokens_str_suffix_py(
            &self,
            py: Python<'_>,
            tokens: Vec<usize>,
        ) -> Vec<(String, SortedTokenRange)> {
            py.allow_threads(|| {
                self.parse_rev_token_id_seq(tokens.into_iter().rev())
                    .into_iter()
                    .filter_map(|(b, c)| String::from_utf8(b.into()).ok().map(|s| (s, c)))
                    .collect()
            })
        }

        fn get_original_token_ids(&self, py: Python<'_>, seq: TokenIdSeq) -> TokenIdSeq {
            py.allow_threads(|| seq.map(|id| self.order.get(id as usize).copied().unwrap_or(id)))
        }

        fn get_sorted_token_ids(&self, py: Python<'_>, seq: TokenIdSeq) -> TokenIdSeq {
            py.allow_threads(|| seq.map(|id| self.rank.get(id as usize).copied().unwrap_or(id)))
        }
    }
}
