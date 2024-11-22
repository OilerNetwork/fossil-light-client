#[starknet::interface]
pub trait IStore<TContractState> {
    fn store_latest_blockhash_from_l1(ref self: TContractState, block_number: u64, blockhash: u256);
    fn update_mmr_state(
        ref self: TContractState,
        latest_mmr_block: u64,
        mmr_root: felt252,
        elements_count: u64,
        leaves_count: u64,
        peaks: Array<felt252>
    );
    fn get_latest_blockhash_from_l1(self: @TContractState) -> (u64, u256);
    fn get_mmr_state(self: @TContractState) -> (u64, felt252, u64, u64, Array<felt252>);
}

#[starknet::contract]
mod Store {
    use core::starknet::storage::{
        StoragePointerReadAccess, StoragePointerWriteAccess, Vec, VecTrait, MutableVecTrait
    };

    #[starknet::storage_node]
    pub(crate) struct MMRState {
        root_hash: felt252,
        elements_count: u64,
        leaves_count: u64,
    }

    #[storage]
    struct Storage {
        latest_blockhash_from_l1: (u64, u256),
        latest_mmr_block: u64,
        mmr_state: MMRState,
        peaks: Vec<felt252>,
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
        root_hash: felt252,
        elements_count: u64,
        leaves_count: u64,
        peaks: Array<felt252>,
    }

    #[abi(embed_v0)]
    impl StoreImpl of super::IStore<ContractState> {
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
            mmr_root: felt252,
            elements_count: u64,
            leaves_count: u64,
            peaks: Array<felt252>
        ) {
            self.latest_mmr_block.write(latest_mmr_block);

            let mut curr_state = self.mmr_state;
            curr_state.root_hash.write(mmr_root);
            curr_state.elements_count.write(elements_count);
            curr_state.leaves_count.write(leaves_count);

            for peak in peaks.clone() {
                self.peaks.append().write(peak);
            };

            self
                .emit(
                    MmrStateUpdated {
                        latest_mmr_block, root_hash: mmr_root, elements_count, leaves_count, peaks
                    }
                );
        }

        fn get_mmr_state(self: @ContractState) -> (u64, felt252, u64, u64, Array<felt252>) {
            let latest_mmr_block = self.latest_mmr_block.read();
            
            let curr_state = self.mmr_state;
            let (mmr_root, elements_count, leaves_count) = (
                curr_state.root_hash.read(),
                curr_state.elements_count.read(),
                curr_state.leaves_count.read(),
            );

            let mut peaks = array![];
            for i in 0..self.peaks.len() {
                peaks.append(self.peaks.at(i).read());
            };

            (latest_mmr_block, mmr_root, elements_count, leaves_count, peaks)
        }
    }
}
