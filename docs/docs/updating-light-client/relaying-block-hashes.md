---
id: relaying-block-hashes
title: Relaying Finalized Block Hashes
---

Every several hours, a finalized block hash is sent from L1 to L2 on Starknet via the Fossil Relayer.

Finalized blocks are considered "safe" as they are not subject to chain reorganizations.

## Relayer Configuration

The relayer requires several key configurations:
- An Ethereum private key for transaction signing
- The L2 message proxy contract address on Starknet (66 characters: '0x' + 64 hex chars)
- The Ethereum RPC URL for L1 connectivity
- The L1 message sender contract address

## L1 Message Sender Contract

The L1MessageSender contract handles the block hash relay:

1. **Block Finalization**:
   - Uses `block.number - 96` to select a finalized block
   - With Ethereum's ~12 second block time, this represents ~19 minutes
   - This ensures the block hash is from a finalized block, preventing any reorg issues

2. **Message Formatting**:
   - Splits both block hash and block number into high/low 128-bit components
   - Creates a message array with these four components
   - Sends the message to L2 with a value of 30,000 Wei

```solidity
function sendFinalizedBlockHashToL2(uint256 l2RecipientAddr) external payable {
    uint256 finalizedBlockNumber = block.number - 96;
    bytes32 parentHash = blockhash(finalizedBlockNumber);
    uint256 blockNumber = uint256(finalizedBlockNumber);
    _sendBlockHashToL2(parentHash, blockNumber, l2RecipientAddr);
}
```

## L2 Message Reception

The L1MessageProxy contract on Starknet handles incoming messages:

1. **Message Validation**:
   - Verifies the message sender is the authorized L1MessageSender contract
   - Reconstructs the block hash and number from the low/high components
  
  ```rust
    #[l1_handler]
    fn receive_from_l1(
        ref self: ContractState,
        from_address: felt252,
        block_hash_low: felt252,
        block_hash_high: felt252,
        block_number_low: felt252,
        block_number_high: felt252,
    ) {
        let block_hash = u256 {
            low: block_hash_high.try_into().unwrap(), high: block_hash_low.try_into().unwrap(),
        };
        let block_number: u64 = block_number_low.try_into().unwrap();
        assert!(
            from_address == self.l1_messages_sender.read().into(),
            "L1MessagesProxy: unauthorized sender",
        );

        let store = self.store_dispatcher.read();
        store.store_latest_blockhash_from_l1(block_number, block_hash);
    }
    ```

2. **State Update**:
   - Forwards the validated block hash and number to the Fossil Store contract
   - The store contract updates its state and emits a `LatestBlockhashFromL1Stored` event

   ```rust
    fn store_latest_blockhash_from_l1(
            ref self: ContractState, block_number: u64, blockhash: u256,
    ) {
        assert!(
            starknet::get_caller_address() == self.l1_message_proxy_address.read(),
            "Only L1 Message Proxy can store latest blockhash from L1",
        );
        self.latest_blockhash_from_l1.write((block_number, blockhash));
        self.emit(LatestBlockhashFromL1Stored { block_number, blockhash });
    }
   ```

This process ensures a secure and verifiable relay of finalized block hashes from Ethereum to Starknet, maintaining the integrity of the cross-chain communication.
