---
id: block-headers-validation
title: Block Header Validation (Keccak Hash Verification and Parent-Child Relation)
---

Validation ensures each block header is both internally consistent and part of a valid chain:

- **Integrity Check**: The Keccak hash of relevant fields in each block header is recomputed. The result must match the `block_hash` field. Any discrepancy indicates corruption or tampering of the blocks’ data. This step also ensures that the parent hash field in the block header is valid providing a guarantee of block_hash - parent_hash relationship.
- **Chain Validity Check**: The `parent_hash` field of each block is compared with the `block_hash` of the preceding block. This ensures the headers form a valid, unbroken chain within a batch.

```rust
pub fn are_blocks_and_chain_valid(block_headers: &[VerifiableBlockHeader], chain_id: u64) -> bool {
    for (i, block) in block_headers.iter().enumerate() {
        let block_hash = block.block_hash.clone();
        let parent_hash = block.parent_hash.clone().unwrap_or_default();
        let block_number = block.number;

        let is_valid = verify_block(block_number as u64, block.clone(), &block_hash, chain_id);

        if !is_valid {
            return false;
        }

        if i != 0 {
            let previous_block = &block_headers[i - 1];
            let previous_block_hash = previous_block.block_hash.clone();

            if parent_hash != previous_block_hash {
                return false;
            }
        }
    }

    true
}
```
