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
    nodes, pre_len = dfs_token_seq_trie(list(zip(tokens_seq, pred_ranges)))

    print(f"{pre_len=}")
    print([node.token for node in nodes])
    print([node.depth for node in nodes])
    print([node.subtree_upper for node in nodes])

    for q, node in enumerate(nodes):
        if node.value is None:
            continue
        seq = []
        for j in range(q):
            if nodes[j].subtree_upper >= node.subtree_upper:
                seq.append(nodes[j].token)
        seq.append(node.token)
        print(seq, node.value)
        assert seq in tokens_seq
        assert pred_ranges[tokens_seq.index(seq)] == node.value
        assert node.depth + 1 == len(seq)

    for q in range(len(nodes)):
        masks = [
            k <= q and nodes[k].subtree_upper >= nodes[q].subtree_upper
            for k in range(len(nodes))
        ]
        print("".join(map(str, map(int, masks))))


if __name__ == "__main__":
    test_dfs_token_seq_trie()
    test_dfs_trie_value_on_prefix_chain()
