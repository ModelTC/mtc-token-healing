from typing import Generic, Optional, Sequence, Tuple, TypeVar, overload

TokenId = int
SortedTokenId = int

class SortedTokenRange:
    lower: SortedTokenId
    upper: SortedTokenId

    def __init__(self, lower: SortedTokenId = 0, upper: SortedTokenId = 0) -> None: ...

class VocabPrefixAutomaton:
    def __init__(self, vocab: Sequence[bytes]) -> None: ...
    @property
    def vocab_size(self) -> int: ...
    def get_order(self) -> Sequence[TokenId]: ...
    def get_rank(self) -> Sequence[SortedTokenId]: ...
    def parse_bytes(self, inputs: bytes) -> Sequence[Tuple[int, SortedTokenRange]]: ...
    def parse_tokens(
        self, token_ids: Sequence[TokenId]
    ) -> Sequence[Tuple[bytes, SortedTokenRange]]: ...
    def parse_tokens_str_suffix(
        self, token_ids: Sequence[TokenId]
    ) -> Sequence[Tuple[str, SortedTokenRange]]: ...
    @overload
    def get_original_token_ids(self, sorted_token_id: SortedTokenId) -> TokenId: ...
    @overload
    def get_original_token_ids(
        self, sorted_token_ids: Sequence[SortedTokenId]
    ) -> Sequence[TokenId]: ...
    @overload
    def get_sorted_token_ids(self, token_id: TokenId) -> SortedTokenId: ...
    @overload
    def get_sorted_token_ids(
        self, token_ids: Sequence[TokenId]
    ) -> Sequence[SortedTokenId]: ...

Value = TypeVar("Value")

class TokenSeqTrieNode(Generic[Value]):
    token: int
    parent: int
    subtree_lower: int
    subtree_upper: int
    depth: int
    num_children: int
    value: Optional[Value]

class TokenSeqTrie(Generic[Value]):
    tokens: Sequence[int]
    parents: Sequence[int]
    subtree_lower_seq: Sequence[int]
    subtree_upper_seq: Sequence[int]
    depths: Sequence[int]
    num_children_seq: Sequence[int]
    values: Sequence[Optional[Value]]

    def __len__(self) -> int: ...

def dfs_token_seq_trie(
    sequences: Sequence[Sequence[int]],
    values: Sequence[Value],
) -> Tuple[TokenSeqTrie, int]: ...
def dfs_token_seq_trie_as_nodes(
    sequences_and_values: Sequence[Tuple[Sequence[int], Value]],
) -> Tuple[Sequence[TokenSeqTrieNode[Value]], int]: ...
