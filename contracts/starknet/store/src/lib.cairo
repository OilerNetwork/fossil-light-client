#[starknet::interface]
pub trait IStore<TContractState> {
    fn store_latest_blockhash_from_l1(ref self: TContractState, block_number: u64, blockhash: u256);
    fn update_mmr_state(ref self: TContractState, latest_block_number: u64, mmr_root: felt252);
    fn get_latest_blockhash_from_l1(self: @TContractState) -> (u64, u256);
    fn get_mmr_state(self: @TContractState) -> (u64, felt252);
}

#[starknet::contract]
mod Store {
    #[storage]
    struct Storage {
        latest_blockhash_from_l1: (u64, u256),
        mmr_state: (u64, felt252),
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        LatestBlockhashFromL1Stored: LatestBlockhashFromL1Stored,
        MmrRootStored: MmrRootStored,
    }

    #[derive(Drop, starknet::Event)]
    struct LatestBlockhashFromL1Stored {
        block_number: u64,
        blockhash: u256,
    }

    #[derive(Drop, starknet::Event)]
    struct MmrRootStored {
        latest_block_number: u64,
        mmr_root: felt252,
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

        fn update_mmr_state(ref self: ContractState, latest_block_number: u64, mmr_root: felt252) {
            self.mmr_state.write((latest_block_number, mmr_root));
            self.emit(MmrRootStored { latest_block_number, mmr_root });
        }

        fn get_mmr_state(self: @ContractState) -> (u64, felt252) {
            self.mmr_state.read()
        }
    }
}
