use std::sync::{Arc, OnceLock};

use clap::Parser;
use color_eyre::{Result, eyre::eyre};
use mtc_token_healing::{
    InferRequest, InferResponse, Prediction, ReorderedTokenId, SearchTree, TokenId,
    VocabPrefixAutomaton,
};
use regex::Regex;
use tokenizers::{AddedToken, Tokenizer};
use tokio::runtime::Runtime;

pub struct DummyInfer {
    tree: SearchTree,
    current_tokens_buffer: Vec<TokenId>,
}

impl DummyInfer {
    pub async fn new(tree: SearchTree) -> Result<Self> {
        Ok(Self {
            tree,
            current_tokens_buffer: Default::default(),
        })
    }

    pub async fn handle_infer_req(&mut self, req: InferRequest) -> Result<InferResponse> {
        println!("request: {req:?}");

        if req.backtrace > 0 {
            let buf = &mut self.current_tokens_buffer;
            assert!(buf.len() >= req.backtrace);
            buf.drain(buf.len() - req.backtrace..);
            println!("backtracing: {}", req.backtrace);
        }

        if let Some(token) = req.feed {
            self.current_tokens_buffer.push(token);
            println!("decoding: {token:?}\n{:?}", self.current_tokens_buffer);
        } else {
            assert!(self.current_tokens_buffer.is_empty());
            // println!("prefilling:\n{:?}", self.tree.prefilled_token_ids())
        }

        let decoded_len = self.current_tokens_buffer.len() as i32;

        let sampled = if let Some((lower, upper)) = req.sampling_id_range.as_ref() {
            assert!(lower < upper);
            let id = rand::random::<u32>() % (upper.0 - lower.0) + lower.0;
            Some(Prediction {
                token_id: ReorderedTokenId(id),
                // log_prob: rand::random(),
                // NOTE: The factor is to normalize accumulated random fake log_prob.
                // **It is not needed for real log_prob generated from language models.**
                log_prob: rand::random::<f64>() * f64::powi(0.5, decoded_len),
            })
        } else {
            None
        };

        let sparse_choices = req
            .sparse_choices
            .iter()
            .map(|&id| Prediction {
                token_id: id,
                // log_prob: rand::random(),
                // NOTE: The factor is to normalize accumulated random fake log_prob.
                // **It is not needed for real log_prob generated from language models.**
                log_prob: rand::random::<f64>() * f64::powi(0.5, decoded_len + 1),
            })
            .collect();

        let res = InferResponse {
            sampled,
            sparse_choices,
        };

        println!("response: {res:?}");

        Ok(res)
    }
}

fn parse_byte_repr<S: AsRef<str>>(s: S) -> Result<u8, S> {
    static BYTE_REPR: OnceLock<Regex> = OnceLock::new();
    let byte_repr = BYTE_REPR
        .get_or_init(|| Regex::new("^<0[xX][0-9a-fA-F]{2}>$").expect("invalid byte repr regex?"));
    const PRE_LEN: usize = "<0x".len();
    const SUF_LEN: usize = ">".len();

    if byte_repr.is_match(s.as_ref()) {
        if let Some(hex) = s
            .as_ref()
            .get(PRE_LEN..s.as_ref().len().saturating_sub(SUF_LEN).max(PRE_LEN))
        {
            if let Ok(b) = u8::from_str_radix(hex, 16) {
                return Ok(b);
            }
        }
    }

    Err(s)
}

fn build_vocab<T: AsRef<Tokenizer>>(tokenizer: T) -> Result<Vec<Vec<u8>>> {
    let mut tokenizer = tokenizer.as_ref().clone();
    let vocab_size = tokenizer.get_vocab_size(true);

    let dummy_special_token = AddedToken::from("<*dummy-surrounding*>", true);
    let add_token_res = tokenizer.add_special_tokens(&[dummy_special_token.clone()]);
    assert!(add_token_res == 1);
    let &dummy_token_id = tokenizer
        .get_added_vocabulary()
        .get_vocab()
        .get(&dummy_special_token.content)
        .expect("new dummy special token should be in the vocab");
    assert!((dummy_token_id as usize) >= vocab_size);

    let mut token_bytes = vec![Vec::new(); vocab_size];
    for (token, id) in tokenizer.get_vocab(true) {
        if id == dummy_token_id {
            continue;
        }
        assert!((id as usize) < vocab_size);
        match parse_byte_repr(token) {
            Ok(byte) => token_bytes[id as usize].push(byte),
            Err(_) => {
                if tokenizer
                    .get_added_vocabulary()
                    .get_added_tokens_decoder()
                    .contains_key(&id)
                {
                    // ignore special tokens
                    continue;
                }

                let decoded = tokenizer
                    .decode(&[dummy_token_id, id, dummy_token_id], false)
                    .map_err(|e| eyre!(e))?;

                assert!(decoded.starts_with(&dummy_special_token.content));
                assert!(decoded.ends_with(&dummy_special_token.content));

                let offset = dummy_special_token.content.len();
                token_bytes[id as usize].extend(decoded[offset..decoded.len() - offset].as_bytes())
            }
        }
    }

    Ok(token_bytes)
}

#[derive(Clone, Debug, Parser)]
struct Args {
    #[arg(short, long, env, default_value = "codellama/CodeLlama-7b-Instruct-hf")]
    tokenizer_path: String,
}

async fn main_body() -> Result<()> {
    let args = Args::try_parse()?;

    let tokenizer =
        Arc::new(Tokenizer::from_pretrained(&args.tokenizer_path, None).map_err(|e| eyre!(e))?);

    let vocab = build_vocab(tokenizer.clone())?;

    let automaton = Arc::new(VocabPrefixAutomaton::new(vocab));

    println!("waiting for text (in json format) from stdin...");
    let text: String = serde_json::from_reader(std::io::stdin())?;

    println!("prompt: {text:?}\n");
    let tokenized = tokenizer
        .encode(text.as_str(), true)
        .map_err(|e| eyre!(e))?;
    let prefilled_text = tokenizer
        .decode(tokenized.get_ids(), false)
        .map_err(|e| eyre!(e))?;

    let offset = tokenized
        .get_ids()
        .iter()
        .filter_map(|&id| {
            tokenizer
                .get_added_vocabulary()
                .get_added_tokens_decoder()
                .get(&id)
        })
        .last()
        .and_then(|special_token| {
            println!("{special_token:?}");
            text.rfind(&special_token.content)
                .map(|pos| pos + special_token.content.len())
        })
        .unwrap_or(0);
    println!("search from pos {offset}\n");

    let Some((tree, mut req)) = SearchTree::new(
        automaton.clone(),
        |end_pos| async {
            let mut res = Vec::new();
            for pos in end_pos {
                let tokenized = tokenizer.encode(&text[..pos], true)?;
                res.push((pos, tokenized.get_ids().to_vec()))
            }
            Ok::<_, tokenizers::Error>(res)
        },
        text.as_str(),
        offset,
    )
    .await
    .map_err(|e| eyre!(e))?
    else {
        println!("no token healing required");
        return Ok(());
    };

    let mut dummy_infer = DummyInfer::new(tree).await?;

    println!(
        "prefilled tokens:\n{:?}\n",
        Vec::from_iter(
            dummy_infer
                .tree
                .prefilled_token_ids()
                .iter()
                .map(|&id| tokenizer.id_to_token(id))
        ),
    );

    loop {
        let res = dummy_infer.handle_infer_req(req).await?;
        req = if let Some(req) = dummy_infer.tree.feed(res)? {
            req
        } else {
            break;
        };
    }

    println!(
        "\nbest choice:\n{:?}\n",
        dummy_infer.tree.get_best_choice()?,
    );

    let best_token_ids_to_decode = dummy_infer.tree.get_best_choice()?.extra_token_ids.clone();
    println!(
        "best choice tokens:\n{:?}\n",
        Vec::from_iter(
            best_token_ids_to_decode
                .iter()
                .map(|&id| tokenizer.id_to_token(id))
        ),
    );

    let full_token_ids: Vec<_> = dummy_infer
        .tree
        .prefilled_token_ids()
        .iter()
        .chain(best_token_ids_to_decode.iter())
        .copied()
        .collect();
    let full_text = tokenizer
        .decode(&full_token_ids, false)
        .map_err(|e| eyre!(e))?;

    println!(
        "decoded best choice:\n{:?}\n",
        &full_text[prefilled_text.len()..]
    );
    println!("complete best choice text:\n{:?}\n", full_text);

    Ok(())
}

fn main() -> Result<()> {
    let runtime = Runtime::new()?;
    runtime.block_on(main_body())
}
