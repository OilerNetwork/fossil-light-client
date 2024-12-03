#[starknet::interface]
pub trait IFossilStore<TContractState> {
    fn store_latest_blockhash_from_l1(ref self: TContractState, block_number: u64, blockhash: u256);
    fn update_mmr_state(
        ref self: TContractState,
        batch_index: u64,
        latest_mmr_block: u64,
        leaves_count: u64,
        mmr_root: u256,
    );
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
        leaves_count: u64,
        root_hash: u256,
    }

    #[derive(Copy, Drop, Serde, Debug)]
    pub struct MMRSnapshot {
        batch_index: u64,
        root_hash: u256,
        leaves_count: u64,
    }

    #[storage]
    struct Storage {
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
        leaves_count: u64,
        root_hash: u256,
    }

    #[abi(embed_v0)]
    impl FossilStoreImpl of super::IFossilStore<ContractState> {
        fn store_latest_blockhash_from_l1(
            ref self: ContractState, block_number: u64, blockhash: u256
        ) {
            self.latest_blockhash_from_l1.write((block_number, blockhash));
            self.emit(LatestBlockhashFromL1Stored { block_number, blockhash });
        }

        fn get_latest_blockhash_from_l1(self: @ContractState) -> (u64, u256) {
            self.latest_blockhash_from_l1.read()
        }

        fn update_mmr_state(
            ref self: ContractState,
            batch_index: u64,
            latest_mmr_block: u64,
            leaves_count: u64,
            mmr_root: u256,
        ) {
            let mut curr_state = self.mmr_batches.entry(batch_index);

            curr_state.leaves_count.write(leaves_count);
            curr_state.root_hash.write(mmr_root);

            self.latest_mmr_block.write(latest_mmr_block);

            self.emit(MmrStateUpdated { batch_index, leaves_count, root_hash: mmr_root });
        }

        fn get_mmr_state(self: @ContractState, batch_index: u64) -> MMRSnapshot {
            let curr_state = self.mmr_batches.entry(batch_index);
            MMRSnapshot {
                batch_index,
                leaves_count: curr_state.leaves_count.read(),
                root_hash: curr_state.root_hash.read(),
            }
        }

        fn get_latest_mmr_block(self: @ContractState) -> u64 {
            self.latest_mmr_block.read()
        }
    }
}
