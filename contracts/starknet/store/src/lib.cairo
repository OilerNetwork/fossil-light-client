#[starknet::interface]
pub trait IFossilStore<TContractState> {
    fn initialize(
        ref self: TContractState,
        verifier_address: starknet::ContractAddress,
        min_update_interval: u64
    );
    fn store_latest_blockhash_from_l1(ref self: TContractState, block_number: u64, blockhash: u256);
    fn update_mmr_state(ref self: TContractState, journal: verifier::Journal);
    fn get_latest_blockhash_from_l1(self: @TContractState) -> (u64, u256);
    fn get_mmr_state(self: @TContractState, batch_index: u64) -> Store::MMRSnapshot;
    fn get_latest_mmr_block(self: @TContractState) -> u64;
}

#[starknet::contract]
mod Store {
    use core::starknet::storage::{
        Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess
    };

    #[starknet::storage_node]
    pub(crate) struct MMRBatch {
        latest_mmr_block: u64,
        latest_mmr_block_hash: u256,
        leaves_count: u64,
        root_hash: u256,
    }

    #[derive(Copy, Drop, Serde, Debug)]
    pub struct MMRSnapshot {
        batch_index: u64,
        latest_mmr_block: u64,
        latest_mmr_block_hash: u256,
        root_hash: u256,
        leaves_count: u64,
    }

    #[storage]
    struct Storage {
        initialized: bool,
        verifier_address: starknet::ContractAddress,
        min_update_interval: u64,
        latest_blockhash_from_l1: (u64, u256),
        latest_mmr_block: u64,
        mmr_batches: Map<u64, MMRBatch>,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        LatestBlockhashFromL1Stored: LatestBlockhashFromL1Stored,
        MmrStateUpdated: MmrStateUpdated,
    }

    #[derive(Drop, starknet::Event)]
    struct LatestBlockhashFromL1Stored {
        block_number: u64,
        blockhash: u256,
    }

    #[derive(Drop, starknet::Event)]
    struct MmrStateUpdated {
        batch_index: u64,
        latest_mmr_block: u64,
        latest_mmr_block_hash: u256,
        leaves_count: u64,
        root_hash: u256,
    }

    #[abi(embed_v0)]
    impl FossilStoreImpl of super::IFossilStore<ContractState> {
        fn initialize(
            ref self: ContractState,
            verifier_address: starknet::ContractAddress,
            min_update_interval: u64
        ) {
            assert!(!self.initialized.read(), "Contract already initialized");
            self.initialized.write(true);
            self.verifier_address.write(verifier_address);
            self.min_update_interval.write(min_update_interval);
        }

        fn store_latest_blockhash_from_l1(
            ref self: ContractState, block_number: u64, blockhash: u256
        ) {
            self.latest_blockhash_from_l1.write((block_number, blockhash));
            self.emit(LatestBlockhashFromL1Stored { block_number, blockhash });
        }

        fn get_latest_blockhash_from_l1(self: @ContractState) -> (u64, u256) {
            self.latest_blockhash_from_l1.read()
        }

        fn update_mmr_state(ref self: ContractState, journal: verifier::Journal) {
            assert!(
                starknet::get_caller_address() == self.verifier_address.read(),
                "Only Fossil Verifier can update MMR state"
            );
            let global_latest_mmr_block = self.latest_mmr_block.read();

            if journal.latest_mmr_block > global_latest_mmr_block {
                let min_update_interval = self.min_update_interval.read();
                let actual_update_interval = journal.latest_mmr_block
                    - self.latest_mmr_block.read();
                assert!(
                    actual_update_interval >= min_update_interval,
                    "Update interval: {} must be greater than or equal to the minimum update interval: {}",
                    actual_update_interval,
                    min_update_interval
                );
                self.latest_mmr_block.write(journal.latest_mmr_block);
            }

            let mut curr_state = self.mmr_batches.entry(journal.batch_index);

            curr_state.latest_mmr_block.write(journal.latest_mmr_block);
            curr_state.latest_mmr_block_hash.write(journal.latest_mmr_block_hash);
            curr_state.leaves_count.write(journal.leaves_count);
            curr_state.root_hash.write(journal.root_hash);

            self
                .emit(
                    MmrStateUpdated {
                        batch_index: journal.batch_index,
                        latest_mmr_block: journal.latest_mmr_block,
                        latest_mmr_block_hash: journal.latest_mmr_block_hash,
                        leaves_count: journal.leaves_count,
                        root_hash: journal.root_hash
                    }
                );
        }

        fn get_mmr_state(self: @ContractState, batch_index: u64) -> MMRSnapshot {
            let curr_state = self.mmr_batches.entry(batch_index);
            MMRSnapshot {
                batch_index,
                latest_mmr_block: curr_state.latest_mmr_block.read(),
                latest_mmr_block_hash: curr_state.latest_mmr_block_hash.read(),
                leaves_count: curr_state.leaves_count.read(),
                root_hash: curr_state.root_hash.read(),
            }
        }

        fn get_latest_mmr_block(self: @ContractState) -> u64 {
            self.latest_mmr_block.read()
        }
    }
}
