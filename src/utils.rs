use std::convert::Infallible;

use general_sam::{
    BTreeTransTable, BoxBisectTable, GeneralSam, SAM_ROOT_NODE_ID, TransitionTable, Trie,
    TrieNodeAlike,
};
use smallvec::SmallVec;

pub type TokenId = u32;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    derive_more::Deref,
    derive_more::AsRef,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass)]
pub struct ReorderedTokenId(pub u32);

#[cfg(feature = "pyo3")]
mod _pyo3 {
    use pyo3::pymethods;

    use crate::ReorderedTokenId;

    #[pymethods]
    impl ReorderedTokenId {
        #[new]
        fn new(value: u32) -> Self {
            Self(value)
        }

        pub fn __int__(&self) -> u32 {
            self.0
        }

        #[getter]
        pub fn get_value(&self) -> u32 {
            self.0
        }

        #[setter]
        pub fn set_value(&mut self, value: u32) {
            self.0 = value;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass(get_all, set_all))]
pub struct CountInfo {
    pub cnt: usize,
    pub tot_cnt_lower: usize,
    pub tot_cnt_upper: usize,
}

pub(crate) type TokenBytes = SmallVec<[u8; 32]>;

#[derive(Debug)]
pub(crate) struct SortResult {
    pub cnt_info_of_vocab: Vec<CountInfo>,
    pub order: Vec<TokenId>,
    pub rank: Vec<ReorderedTokenId>,
}

pub(crate) fn gen_sam_cnt_info<
    T: AsRef<[u8]>,
    V: IntoIterator<Item = T>,
    C: AsRef<[CountInfo]>,
    TransTable: TransitionTable<KeyType = u8>,
>(
    sam_of_rev_tokens: &GeneralSam<TransTable>,
    vocab: V,
    cnt_info_of_vocab: C,
) -> Vec<Option<CountInfo>> {
    let mut cnt_info_of_sam_rev = vec![None; sam_of_rev_tokens.num_of_nodes()];

    for (token, cnt_info) in vocab.into_iter().zip(cnt_info_of_vocab.as_ref().iter()) {
        let mut state = sam_of_rev_tokens.get_root_state();

        state.feed_ref(token.as_ref().iter().rev());

        let mut new_info = cnt_info.clone();
        new_info.cnt = 1;
        cnt_info_of_sam_rev[state.node_id] = Some(new_info);
    }

    for &id in sam_of_rev_tokens
        .get_topo_and_suf_len_sorted_node_ids()
        .iter()
        .rev()
    {
        if id == SAM_ROOT_NODE_ID {
            continue;
        }

        let Some(node) = sam_of_rev_tokens.get_node(id) else {
            continue;
        };

        let Some(cnt_info) = cnt_info_of_sam_rev[id].clone() else {
            continue;
        };

        let link_cnt_info = &mut cnt_info_of_sam_rev[node.get_suffix_parent_id()];

        let Some(link_cnt_info) = link_cnt_info.as_mut() else {
            *link_cnt_info = Some(cnt_info);
            continue;
        };

        link_cnt_info.cnt += cnt_info.cnt;
        link_cnt_info.tot_cnt_lower = link_cnt_info.tot_cnt_lower.min(cnt_info.tot_cnt_lower);
        link_cnt_info.tot_cnt_upper = link_cnt_info.tot_cnt_upper.max(cnt_info.tot_cnt_upper);
    }

    #[cfg(debug_assertions)]
    for (id, cnt_info) in cnt_info_of_sam_rev.iter().enumerate() {
        if id == SAM_ROOT_NODE_ID {
            continue;
        }
        let Some(cnt_info) = cnt_info else {
            continue;
        };
        let Some(node) = sam_of_rev_tokens.get_node(id) else {
            continue;
        };

        let link_cnt_info = cnt_info_of_sam_rev[node.get_suffix_parent_id()].as_ref();

        debug_assert!(link_cnt_info.is_some_and(|link_cnt_info| {
            link_cnt_info.tot_cnt_lower <= cnt_info.tot_cnt_lower
                && link_cnt_info.tot_cnt_upper >= cnt_info.tot_cnt_upper
        }));
    }

    cnt_info_of_sam_rev
}

pub(crate) fn sort_vocab_with_trie<T: AsRef<[u8]>, V: ExactSizeIterator<Item = T>>(
    vocab: V,
) -> SortResult {
    let vocab_size = vocab.len();

    let (trie, trie_node_ids) = {
        let mut trie = Trie::<BTreeTransTable<_>>::default();
        let trie_node_ids: Vec<_> = vocab
            .into_iter()
            .map(|token| trie.insert(token.as_ref().iter().copied()))
            .collect();
        (trie, trie_node_ids)
    };

    let mut cnt_info_of_trie = vec![CountInfo::default(); trie.num_of_nodes()];
    trie_node_ids
        .iter()
        .for_each(|&i| cnt_info_of_trie[i].cnt += 1);

    let mut tot_cnt = 0;

    let res = trie.get_root_state().dfs_travel(|event| {
        match event {
            general_sam::TravelEvent::PushRoot(state)
            | general_sam::TravelEvent::Push(state, _, _) => {
                let id = state.node_id;
                let cnt_info = &mut cnt_info_of_trie[id];
                cnt_info.tot_cnt_lower = tot_cnt;
                tot_cnt += cnt_info.cnt;
            }
            general_sam::TravelEvent::Pop(state, _) => {
                let id = state.node_id;
                let cnt_info = &mut cnt_info_of_trie[id];
                cnt_info.tot_cnt_upper = tot_cnt;
            }
        }
        Ok::<_, Infallible>(())
    });
    match res {
        Ok(()) => {}
        Err(e) => match e {},
    }

    let cnt_info_of_vocab: Vec<_> = (0..vocab_size)
        .map(|i| cnt_info_of_trie[trie_node_ids[i]].clone())
        .collect();

    let order = {
        let mut order: Vec<_> = (0..vocab_size as TokenId).collect();
        order.sort_by_key(|&i| cnt_info_of_vocab[i as usize].tot_cnt_lower);
        order
    };

    let rank = {
        let mut rank = vec![ReorderedTokenId(0); vocab_size];
        order
            .iter()
            .enumerate()
            .for_each(|(k, &i)| rank[i as usize] = ReorderedTokenId(k as _));
        rank
    };

    debug_assert_eq!(order.len(), vocab_size);
    debug_assert_eq!(rank.len(), vocab_size);
    debug_assert_eq!(cnt_info_of_vocab.len(), vocab_size);

    SortResult {
        cnt_info_of_vocab,
        order,
        rank,
    }
}

pub(crate) fn build_sam_of_reversed_tokens<T: AsRef<[u8]>, V: IntoIterator<Item = T>>(
    vocab: V,
) -> GeneralSam<BoxBisectTable<u8>> {
    let trie_of_rev_tokens = {
        let mut trie = Trie::<BTreeTransTable<_>>::default();
        vocab.into_iter().for_each(|token| {
            trie.insert(token.as_ref().iter().copied().rev());
        });
        trie
    };
    GeneralSam::<BTreeTransTable<_>>::from_trie(trie_of_rev_tokens.get_root_state())
        .alter_trans_table_into()
}
