//! 翻转后的 tokens 构成的后缀自动机的 link 树，
//! 是与 tokens 的前缀树同构的。
//!
//! 因此查询某一字符串是哪些 tokens 的前缀，
//! 等同于查询翻转后的字符串在后缀自动机上走到的状态所对应的 link 树的子树。
//!
//! The link tree of a suffix automaton of reversed tokens
//! is isomorphic to the prefix tree of tokens.
//!
//! Thus finding tokens prefixed with a string
//! is the same as walking to the state on the suffix automaton
//! and gathering information among the subtree of the link tree.
pub mod choice;
pub mod search_tree;
pub mod utils;
pub mod vocab;

pub use crate::{
    choice::BestChoice,
    search_tree::{InferRequest, InferResponse, Prediction, SearchTree, SearchTreeError},
    utils::{CountInfo, ReorderedTokenId, TokenId},
    vocab::VocabPrefixAutomaton,
};

#[cfg(test)]
mod tests;
