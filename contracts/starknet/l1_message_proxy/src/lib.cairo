#[starknet::contract]
pub mod L1MessageProxy {
    use starknet::{ContractAddress, EthAddress};
    use store::{IStoreDispatcher, IStoreDispatcherTrait};

    #[storage]
    struct Storage {
        l1_messages_sender: EthAddress,
        store_dispatcher: IStoreDispatcher,
    }

    #[constructor]
    fn constructor(
        ref self: ContractState, l1_messages_sender: EthAddress, store_address: ContractAddress
    ) {
        self.l1_messages_sender.write(l1_messages_sender);
        self.store_dispatcher.write(IStoreDispatcher { contract_address: store_address });
    }

    #[l1_handler]
    fn receive_from_l1(
        ref self: ContractState,
        from_address: felt252,
        block_hash_low: felt252,
        block_hash_high: felt252,
        block_number_low: felt252,
        block_number_high: felt252
    ) {
        let block_hash = u256 {
            low: block_hash_high.try_into().unwrap(), high: block_hash_low.try_into().unwrap()
        };
        let block_number: u64 = block_number_low.try_into().unwrap();
        assert!(
            from_address == self.l1_messages_sender.read().into(),
            "L1MessagesProxy: unauthorized sender"
        );

        let store = self.store_dispatcher.read();
        println!("store address: {:?}", store.contract_address);
        store.store_latest_blockhash_from_l1(block_number, block_hash);
    }
}
