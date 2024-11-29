#[starknet::interface]
pub trait IFossilStore<TContractState> {
    fn store_latest_blockhash_from_l1(ref self: TContractState, block_number: u64, blockhash: u256);
    fn update_mmr_state(
        ref self: TContractState,
        latest_mmr_block: u64,
        mmr_root: u256,
        elements_count: u64,
        leaves_count: u64,
    );
    fn get_latest_blockhash_from_l1(self: @TContractState) -> (u64, u256);
    fn get_mmr_state(self: @TContractState) -> Store::MMRSnapshot;
}

#[starknet::contract]
mod Store {
    use core::starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess,};

    #[starknet::storage_node]
    pub(crate) struct MMRState {
        latest_block_number: u64,
        root_hash: u256,
        elements_count: u64,
        leaves_count: u64,
    }

    #[derive(Copy, Drop, Serde, Debug)]
    pub struct MMRSnapshot {
        latest_block_number: u64,
        root_hash: u256,
        elements_count: u64,
        leaves_count: u64,
    }

    #[storage]
    struct Storage {
        latest_blockhash_from_l1: (u64, u256),
        mmr_state: MMRState,
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
        latest_mmr_block: u64,
        root_hash: u256,
        elements_count: u64,
        leaves_count: u64,
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
            latest_mmr_block: u64,
            mmr_root: u256,
            elements_count: u64,
            leaves_count: u64,
        ) {
            let mut curr_state = self.mmr_state;
            curr_state.latest_block_number.write(latest_mmr_block);
            curr_state.root_hash.write(mmr_root);
            curr_state.elements_count.write(elements_count);
            curr_state.leaves_count.write(leaves_count);

            self
                .emit(
                    MmrStateUpdated {
                        latest_mmr_block, root_hash: mmr_root, elements_count, leaves_count
                    }
                );
        }

        fn get_mmr_state(self: @ContractState) -> MMRSnapshot {
            let curr_state = self.mmr_state;
            MMRSnapshot {
                latest_block_number: curr_state.latest_block_number.read(),
                root_hash: curr_state.root_hash.read(),
                elements_count: curr_state.elements_count.read(),
                leaves_count: curr_state.leaves_count.read(),
            }
        }
    }
}
