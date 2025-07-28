from typing import Optional, Sequence, Tuple, overload

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

class TokenSeqTrieNode:
    token: int
    pred_range: Optional[SortedTokenRange]
    parent: int
    subtree_lower: int
    subtree_upper: int

def dfs_token_seq_trie(
    token_ids_seq: Sequence[Sequence[int]],
    pred_rank_ranges: Sequence[SortedTokenRange],
) -> Tuple[Sequence[TokenSeqTrieNode], int]: ...
