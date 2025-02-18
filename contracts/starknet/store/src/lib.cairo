#[starknet::interface]
pub trait IFossilStore<TContractState> {
    fn initialize(
        ref self: TContractState,
        verifier_address: starknet::ContractAddress,
        min_update_interval: u64,
    );
    fn store_latest_blockhash_from_l1(ref self: TContractState, block_number: u64, blockhash: u256);
    fn update_store_state(
        ref self: TContractState, journal: verifier::Journal, ipfs_hash: ByteArray,
    );
    fn get_latest_blockhash_from_l1(self: @TContractState) -> (u64, u256);
    fn get_mmr_state(self: @TContractState, batch_index: u64) -> Store::MMRSnapshot;
    fn get_latest_mmr_block(self: @TContractState) -> u64;
    fn get_min_mmr_block(self: @TContractState) -> u64;
    fn get_batch_last_block_link(self: @TContractState, batch_index: u64) -> u256;
    fn get_batch_first_block_parent_hash(self: @TContractState, batch_index: u64) -> u256;
    fn get_avg_fee(self: @TContractState, batch_index: u64) -> u64;
    fn get_avg_fees_in_range(
        self: @TContractState, start_batch_index: u64, end_batch_index: u64,
    ) -> Array<u64>;
}

#[starknet::contract]
mod Store {
    use core::starknet::storage::{
        Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess,
    };

    #[starknet::storage_node]
    pub(crate) struct MMRBatch {
        latest_mmr_block: u64,
        latest_mmr_block_hash: u256,
        leaves_count: u64,
        root_hash: u256,
        first_block_parent_hash: u256,
        ipfs_hash: ByteArray,
    }

    #[derive(Drop, Serde, Debug)]
    pub struct MMRSnapshot {
        batch_index: u64,
        latest_mmr_block: u64,
        latest_mmr_block_hash: u256,
        root_hash: u256,
        leaves_count: u64,
        ipfs_hash: ByteArray,
    }

    #[storage]
    struct Storage {
        initialized: bool,
        verifier_address: starknet::ContractAddress,
        latest_blockhash_from_l1: (u64, u256),
        latest_mmr_block: u64,
        mmr_batches: Map<u64, MMRBatch>,
        min_mmr_block: u64,
        min_update_interval: u64,
        avg_fees: Map<u64, u64>,
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
            min_update_interval: u64,
        ) {
            assert!(!self.initialized.read(), "Contract already initialized");
            self.initialized.write(true);
            self.verifier_address.write(verifier_address);
            self.min_update_interval.write(min_update_interval);
        }

        fn store_latest_blockhash_from_l1(
            ref self: ContractState, block_number: u64, blockhash: u256,
        ) {
            self.latest_blockhash_from_l1.write((block_number, blockhash));
            self.emit(LatestBlockhashFromL1Stored { block_number, blockhash });
        }

        fn get_latest_blockhash_from_l1(self: @ContractState) -> (u64, u256) {
            self.latest_blockhash_from_l1.read()
        }

        fn update_store_state(
            ref self: ContractState, journal: verifier::Journal, ipfs_hash: ByteArray,
        ) {
            assert!(
                starknet::get_caller_address() == self.verifier_address.read(),
                "Only Fossil Verifier can update MMR state",
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
                    min_update_interval,
                );
                self.latest_mmr_block.write(journal.latest_mmr_block);
            }

            let mut curr_state = self.mmr_batches.entry(journal.batch_index);

            curr_state.latest_mmr_block.write(journal.latest_mmr_block);

            let min_mmr_block = self.min_mmr_block.read();
            let lowest_batch_block = journal.latest_mmr_block - journal.leaves_count + 1;
            if min_mmr_block != 0 {
                if lowest_batch_block < min_mmr_block {
                    self.min_mmr_block.write(lowest_batch_block);
                }
            } else {
                self.min_mmr_block.write(lowest_batch_block);
            }

            curr_state.latest_mmr_block_hash.write(journal.latest_mmr_block_hash);
            curr_state.leaves_count.write(journal.leaves_count);
            curr_state.root_hash.write(journal.root_hash);
            curr_state.ipfs_hash.write(ipfs_hash);
            curr_state.first_block_parent_hash.write(journal.first_block_parent_hash);

            let [(i_0, fees_0), (i_1, fees_1), (i_2, fees_2), (i_3, fees_3)] = journal.avg_fees;

            if fees_0 != 0 {
                self.avg_fees.write(i_0, fees_0);
            }
            if fees_1 != 0 {
                self.avg_fees.write(i_1, fees_1);
            }
            if fees_2 != 0 {
                self.avg_fees.write(i_2, fees_2);
            }
            if fees_3 != 0 {
                self.avg_fees.write(i_3, fees_3);
            }

            self
                .emit(
                    MmrStateUpdated {
                        batch_index: journal.batch_index,
                        latest_mmr_block: journal.latest_mmr_block,
                        latest_mmr_block_hash: journal.latest_mmr_block_hash,
                        leaves_count: journal.leaves_count,
                        root_hash: journal.root_hash,
                    },
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
                ipfs_hash: curr_state.ipfs_hash.read(),
            }
        }

        fn get_latest_mmr_block(self: @ContractState) -> u64 {
            self.latest_mmr_block.read()
        }

        fn get_min_mmr_block(self: @ContractState) -> u64 {
            self.min_mmr_block.read()
        }

        fn get_batch_last_block_link(self: @ContractState, batch_index: u64) -> u256 {
            let curr_state = self.mmr_batches.entry(batch_index + 1);
            curr_state.first_block_parent_hash.read()
        }

        fn get_batch_first_block_parent_hash(self: @ContractState, batch_index: u64) -> u256 {
            let curr_state = self.mmr_batches.entry(batch_index - 1);
            curr_state.latest_mmr_block_hash.read()
        }

        fn get_avg_fee(self: @ContractState, batch_index: u64) -> u64 {
            self.avg_fees.read(batch_index)
        }

        fn get_avg_fees_in_range(
            self: @ContractState, start_batch_index: u64, end_batch_index: u64,
        ) -> Array<u64> {
            let mut fees = array![];
            for i in start_batch_index..end_batch_index {
                fees.append(self.get_avg_fee(i));
            };
            fees
        }
    }
}
