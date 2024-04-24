use std::collections::BTreeSet;

use crate::VocabPrefixAutomaton;

fn testcase_parse_chars<T: AsRef<str>>(
    automaton: &VocabPrefixAutomaton,
    vocab_sorted: &[&str],
    text: T,
) {
    let text = text.as_ref();
    let res: BTreeSet<_> = automaton
        .parse_chars(text, 0)
        .into_iter()
        .map(|(pos, cnt_info)| (pos, cnt_info.tot_cnt_lower, cnt_info.tot_cnt_upper))
        .collect();
    println!("{text}: {res:?}");
    for (pos, _) in text.char_indices() {
        let prefix = &text[pos..];
        let range = vocab_sorted
            .iter()
            .enumerate()
            .fold(None, |state, (k, token)| {
                if token.starts_with(prefix) {
                    if let Some((u, v)) = state {
                        assert_eq!(v, k);
                        Some((u, k + 1))
                    } else {
                        Some((k, k + 1))
                    }
                } else {
                    state
                }
            });
        if let Some((lower, upper)) = range {
            println!("{pos}: {:?}", &vocab_sorted[lower..upper]);
            assert!(res.contains(&(pos, lower, upper)));
        }
    }
}

fn testcase_vocab_prefix(vocab: &[&str], texts: &[&str]) {
    let automaton = VocabPrefixAutomaton::new(vocab);
    let vocab_sorted = automaton
        .order()
        .iter()
        .map(|&i| vocab[i as usize])
        .collect::<Vec<_>>();

    println!("vocab_sorted: {vocab_sorted:?}");

    for text in texts {
        testcase_parse_chars(&automaton, vocab_sorted.as_slice(), text);
    }
}

#[test]
fn test_chinese_vocab_prefix() {
    let vocab = ["歌曲", "聆听歌曲", "播放歌曲", "歌词", "查看歌词"];
    let texts = [
        "歌曲",
        "聆听歌曲",
        "聆听歌曲",
        "聆听歌曲",
        "播放歌曲",
        "播放歌曲",
        "播放歌曲",
        "歌词",
        "查看歌词",
        "查看歌词",
        "听歌曲",
        "听歌曲",
        "放歌曲",
        "听歌",
        "放歌",
        "词",
        "查看",
        "bba",
        "bbb",
        "bba",
        "bba",
        "cacab",
        "ccc",
    ];
    testcase_vocab_prefix(&vocab, &texts);
}

#[test]
fn test_simple_vocab_prefix() {
    let vocab = ["bb", "ca", "ab", "c", "aa", "bbaa", "a", "cc", "b"];
    let texts = ["bba", "bbb", "bba", "bba", "cacab", "ccc"];
    testcase_vocab_prefix(&vocab, &texts);
}
