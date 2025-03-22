use general_sam::{BoxBisectTable, GeneralSam};

use crate::{
    CountInfo, ReorderedTokenId, TokenId,
    utils::{TokenBytes, build_sam_of_reversed_tokens, gen_sam_cnt_info, sort_vocab_with_trie},
};

#[derive(Clone, Debug)]
pub struct VocabPrefixAutomaton {
    vocab: Vec<TokenBytes>,
    order: Vec<TokenId>,
    rank: Vec<ReorderedTokenId>,
    sam_of_rev_tokens: GeneralSam<BoxBisectTable<u8>>,
    cnt_info_of_sam_rev: Vec<Option<CountInfo>>,
}

impl VocabPrefixAutomaton {
    pub fn new<T: AsRef<[u8]>, V: IntoIterator<Item = T>>(vocab: V) -> Self {
        let vocab: Vec<_> = vocab
            .into_iter()
            .map(|token| TokenBytes::from_slice(token.as_ref()))
            .collect();
        let sort_result = sort_vocab_with_trie(vocab.iter().map(|x| x.as_slice()));
        let sam_of_rev_tokens = build_sam_of_reversed_tokens(vocab.iter().map(|x| x.as_slice()));
        let cnt_info_of_sam_rev = gen_sam_cnt_info(
            &sam_of_rev_tokens,
            vocab.iter().map(|x| x.as_slice()),
            &sort_result.cnt_info_of_vocab,
        );
        Self {
            vocab,
            order: sort_result.order,
            rank: sort_result.rank,
            sam_of_rev_tokens,
            cnt_info_of_sam_rev,
        }
    }

    pub fn vocab(&self) -> &[TokenBytes] {
        &self.vocab
    }

    pub fn order(&self) -> &[TokenId] {
        &self.order
    }

    pub fn rank(&self) -> &[ReorderedTokenId] {
        &self.rank
    }

    pub fn parse_chars<S: AsRef<str>>(
        &self,
        text: S,
        start_from: usize,
    ) -> Vec<(usize, CountInfo)> {
        let text = text.as_ref();

        let mut last = text.len();
        let mut state = self.sam_of_rev_tokens.get_root_state();
        let mut res = Vec::new();

        for (pos, _) in text.char_indices().rev() {
            if pos < start_from {
                break;
            }
            let c = &text.as_bytes()[pos..last];
            state.feed_ref(c.iter().rev());
            if state.is_nil() {
                break;
            }
            if let Some(cnt_info) = self.cnt_info_of_sam_rev[state.node_id].clone() {
                res.push((pos, cnt_info));
            }
            last = pos;
        }

        res
    }
}

#[cfg(feature = "pyo3")]
mod _pyo3 {
    use std::sync::Arc;

    use pyo3::{pyclass, pymethods};

    use crate::{ReorderedTokenId, TokenId, utils::CountInfo};

    use super::VocabPrefixAutomaton;

    #[derive(Clone, Debug, derive_more::Deref)]
    #[pyclass(name = "VocabPrefixAutomaton", frozen)]
    pub struct PyVocabPrefixAutomaton(pub Arc<VocabPrefixAutomaton>);

    #[pymethods]
    impl PyVocabPrefixAutomaton {
        #[new]
        fn py_new(vocab: Vec<String>) -> Self {
            Self(Arc::new(VocabPrefixAutomaton::new(vocab)))
        }

        #[getter("vocab_size")]
        fn vocab_size_py(&self) -> usize {
            self.vocab.len()
        }

        #[pyo3(name = "get_order")]
        fn get_order_py(&self) -> Vec<TokenId> {
            self.order.clone()
        }

        #[pyo3(name = "get_rank")]
        fn get_rank_py(&self) -> Vec<ReorderedTokenId> {
            self.rank.clone()
        }

        #[pyo3(name = "parse_chars")]
        fn parse_chars_py(&self, text: &str, start_from: usize) -> Vec<(usize, CountInfo)> {
            self.parse_chars(text, start_from)
        }
    }
}

#[cfg(feature = "pyo3")]
pub use self::_pyo3::PyVocabPrefixAutomaton;
