from mtc_token_healing import SortedTokenRange, dfs_token_seq_trie


def test_dfs_token_seq_trie():
    tokens_seq = [
        [3, 9, 1, 10, 9, 6, 7],
        [3, 9, 1, 10, 9, 5],
        [3, 9, 1, 10, 2],
        [3, 9, 1, 10],
        [3, 9, 1, 11],
    ]
    pred_ranges = [
        SortedTokenRange(4, 6),
        SortedTokenRange(4, 7),
        SortedTokenRange(3, 9),
        SortedTokenRange(1, 10),
        SortedTokenRange(5, 9),
    ]
    return run_test_dfs_token_seq_trie(tokens_seq, pred_ranges)


def test_dfs_trie_value_on_prefix_chain():
    tokens_seq = [
        [3, 9],
        [3, 9, 1, 10, 9, 6, 7],
        [3, 9, 1, 10, 9, 5],
        [3, 9, 1, 10, 2],
        [3, 9, 1, 10],
        [3, 9, 1, 11],
    ]
    pred_ranges = [
        SortedTokenRange(0, 12),
        SortedTokenRange(4, 6),
        SortedTokenRange(4, 7),
        SortedTokenRange(3, 9),
        SortedTokenRange(1, 10),
        SortedTokenRange(5, 9),
    ]
    return run_test_dfs_token_seq_trie(tokens_seq, pred_ranges)


def run_test_dfs_token_seq_trie(tokens_seq, pred_ranges):
    tree, pre_len = dfs_token_seq_trie(tokens_seq, pred_ranges)

    for q in range(len(tree.values)):
        if tree.values[q] is None:
            continue
        seq = []
        for j in range(q):
            if tree.subtree_upper_seq[j] >= tree.subtree_upper_seq[q]:
                seq.append(tree.tokens[j])
        seq.append(tree.tokens[q])
        print(seq, tree.values[q])
        assert seq in tokens_seq
        assert pred_ranges[tokens_seq.index(seq)] == tree.values[q]
        assert tree.depths[q] + 1 == len(seq)

    for q in range(len(tree.values)):
        masks = [
            k <= q and tree.subtree_upper_seq[k] >= tree.subtree_upper_seq[q]
            for k in range(len(tree.values))
        ]
        print("".join(map(str, map(int, masks))))


if __name__ == "__main__":
    test_dfs_token_seq_trie()
    test_dfs_trie_value_on_prefix_chain()
