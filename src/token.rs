use std::convert::Infallible;

use general_sam::{
    BTreeTransTable, BoxBisectTable, GeneralSam, SAM_ROOT_NODE_ID, TransitionTable, TravelEvent,
    Trie, TrieNodeAlike,
};
use tinyvec::TinyVec;

pub type TokenId = u32;
pub type SortedTokenId = u32;

pub type SmallToken = TinyVec<[u8; 29]>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass(get_all, set_all))]
pub struct SortedTokenRange {
    pub lower: SortedTokenId,
    pub upper: SortedTokenId,
}

#[cfg(feature = "pyo3")]
mod _pyo3 {
    use pyo3::pymethods;

    use super::{SortedTokenId, SortedTokenRange};

    impl SortedTokenRange {
        pub(crate) fn repr_py(&self) -> String {
            let Self { lower, upper } = self;
            format!("SortedTokenRange(lower={}, upper={})", lower, upper)
        }
    }

    #[pymethods]
    impl SortedTokenRange {
        #[new]
        #[pyo3(signature=(lower=0, upper=0))]
        fn py_new(lower: SortedTokenId, upper: SortedTokenId) -> Self {
            Self { lower, upper }
        }

        fn __repr__(&self) -> String {
            self.repr_py()
        }
    }
}

pub(crate) fn build_sam_of_reversed_tokens<
    I: Ord + Clone,
    T: AsRef<[I]>,
    V: IntoIterator<Item = T>,
>(
    vocab: V,
) -> GeneralSam<BoxBisectTable<I>> {
    let trie_of_rev_tokens = {
        let mut trie = Trie::<BTreeTransTable<_>>::default();
        vocab.into_iter().for_each(|token| {
            trie.insert(token.as_ref().iter().cloned().rev());
        });
        trie
    };
    GeneralSam::<BTreeTransTable<_>>::from_trie(trie_of_rev_tokens.get_root_state())
        .alter_trans_table_into()
}

#[derive(Debug)]
pub(crate) struct SortResult {
    pub rank_ranges: Vec<SortedTokenRange>,
    pub order: Vec<TokenId>,
    pub rank: Vec<SortedTokenId>,
}

pub(crate) fn sort_vocab_with_trie<I: Ord + Clone, T: AsRef<[I]>, V: IntoIterator<Item = T>>(
    vocab: V,
) -> SortResult {
    let (trie, trie_node_ids) = {
        let mut trie = Trie::<BTreeTransTable<_>>::default();
        let trie_node_ids: Vec<_> = vocab
            .into_iter()
            .map(|token| trie.insert(token.as_ref().iter().cloned()))
            .collect();
        (trie, trie_node_ids)
    };

    let vocab_size = trie_node_ids.len();

    let mut rank_range_in_trie = vec![SortedTokenRange::default(); trie.num_of_nodes()];
    let mut cnt_tokens_in_trie = vec![0 as SortedTokenId; trie.num_of_nodes()];
    trie_node_ids
        .iter()
        .for_each(|&i| cnt_tokens_in_trie[i] += 1);

    let mut tot_cnt: SortedTokenId = 0;

    let res = trie.get_root_state().dfs_travel(|event| {
        match event {
            TravelEvent::PushRoot(state) | TravelEvent::Push(state, _, _) => {
                let id = state.node_id;
                let rank_range = &mut rank_range_in_trie[id];
                rank_range.lower = tot_cnt;
                tot_cnt += cnt_tokens_in_trie[id];
            }
            TravelEvent::Pop(state, _) => {
                let id = state.node_id;
                let rank_range = &mut rank_range_in_trie[id];
                rank_range.upper = tot_cnt;
            }
        }
        Ok::<_, Infallible>(())
    });
    match res {
        Ok(()) => {}
        Err(e) => match e {},
    }

    let rank_ranges: Vec<_> = (0..vocab_size)
        .map(|i| rank_range_in_trie[trie_node_ids[i]].clone())
        .collect();

    let order = {
        let mut order: Vec<_> = (0..vocab_size as TokenId).collect();
        order.sort_by_key(|&i| rank_ranges[i as usize].lower);
        order
    };

    let rank = {
        let mut rank = vec![0; vocab_size];
        order
            .iter()
            .enumerate()
            .for_each(|(k, &i)| rank[i as usize] = k as SortedTokenId);
        rank
    };

    debug_assert_eq!(order.len(), vocab_size);
    debug_assert_eq!(rank.len(), vocab_size);
    debug_assert_eq!(rank_ranges.len(), vocab_size);

    SortResult {
        rank_ranges,
        order,
        rank,
    }
}

pub(crate) fn label_rank_range_on_sam_of_rev_tokens<
    K: Ord + Clone,
    T: AsRef<[K]>,
    V: IntoIterator<Item = (T, SortedTokenRange)>,
    TransTable: TransitionTable<KeyType = K>,
>(
    sam_of_rev_tokens: &GeneralSam<TransTable>,
    vocab_and_rank_ranges: V,
) -> Vec<Option<SortedTokenRange>> {
    let mut rank_ranges = vec![None; sam_of_rev_tokens.num_of_nodes()];

    for (token, rank_range) in vocab_and_rank_ranges {
        let mut state = sam_of_rev_tokens.get_root_state();
        state.feed_ref(token.as_ref().iter().rev());
        rank_ranges[state.node_id] = Some(rank_range);
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
        let Some(rank_range) = rank_ranges[id].clone() else {
            continue;
        };
        let link_rank_range =
            rank_ranges[node.get_suffix_parent_id()].get_or_insert_with(|| rank_range.clone());
        link_rank_range.lower = link_rank_range.lower.min(rank_range.lower);
        link_rank_range.upper = link_rank_range.upper.max(rank_range.upper);
    }

    #[cfg(debug_assertions)]
    for (id, rank_range) in rank_ranges.iter().enumerate() {
        if id == SAM_ROOT_NODE_ID {
            continue;
        }
        let Some(rank_range) = rank_range else {
            continue;
        };
        let Some(node) = sam_of_rev_tokens.get_node(id) else {
            continue;
        };

        let link_rank_range = rank_ranges[node.get_suffix_parent_id()].as_ref();

        debug_assert!(link_rank_range.is_some_and(|link_rank_range| {
            link_rank_range.lower <= rank_range.lower && link_rank_range.upper >= rank_range.upper
        }));
    }

    rank_ranges
}
