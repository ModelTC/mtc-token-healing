from typing import Optional, Sequence, Tuple

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

class TokenSeqTrieNode:
    token: TokenId
    pred_range: Optional[SortedTokenRange]
    parent: int
    subtree_lower: int
    subtree_upper: int

def dfs_token_seq_trie(
    token_ids_seq: Sequence[Sequence[TokenId]],
    pred_rank_ranges: Sequence[SortedTokenRange],
) -> Sequence[TokenSeqTrieNode]: ...
