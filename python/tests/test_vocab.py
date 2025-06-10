from mtc_token_healing import VocabPrefixAutomaton


def test_vocab_simple():
    vocab = ["bcd", "abc", "cc", "hello", "world", " ", "yes", "no", "."]
    order = sorted(range(len(vocab)), key=lambda i: vocab[i])

    assert len(vocab) == len(order)

    automaton = VocabPrefixAutomaton(vocab)

    assert automaton.vocab_size == len(vocab)
    assert automaton.get_order() == order

    assert all(vocab[order[i]] < vocab[order[i + 1]] for i in range(len(order) - 1))
