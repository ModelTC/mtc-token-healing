from mtc_token_healing import VocabPrefixAutomaton


def test_vocab_simple():
    vocab = ["bcd", "abc", "cc", "hello", "world", " ", "yes", "no", "."]
    order = [5, 8, 1, 0, 2, 3, 7, 4, 6]

    assert len(vocab) == len(order)

    automaton = VocabPrefixAutomaton(vocab)

    assert automaton.vocab_size == len(vocab)
    assert automaton.get_order() == order

    assert all(vocab[order[i]] < vocab[order[i + 1]] for i in range(len(order) - 1))
