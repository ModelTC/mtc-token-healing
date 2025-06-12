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

    nodes = dfs_token_seq_trie(tokens_seq, pred_ranges)

    print([node.token for node in nodes])

    for i, node in enumerate(nodes):
        if node.pred_range is None:
            continue
        seq = []
        for j in range(i):
            if nodes[j].subtree_upper >= node.subtree_upper:
                seq.append(nodes[j].token)
        seq.append(node.token)
        print(seq)
        assert seq in tokens_seq

    for i in range(len(nodes)):
        masks = [
            j < i and nodes[j].subtree_upper >= nodes[i].subtree_upper
            for j in range(len(nodes))
        ]
        print("".join(map(str, map(int, masks))))


if __name__ == "__main__":
    test_dfs_token_seq_trie()
