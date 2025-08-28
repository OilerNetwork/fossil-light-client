use starknet::{ClassHash, EthAddress};

#[starknet::interface]
trait IL1MessageProxy<T> {
    fn update_l1_messages_sender(ref self: T, new_sender: EthAddress);
    fn upgrade(ref self: T, new_implementation: ClassHash);
    fn get_l1_messages_sender(ref self: T) -> EthAddress;
}

#[starknet::contract]
pub mod L1MessageProxy {
    use fossil_store::{IFossilStoreDispatcher, IFossilStoreDispatcherTrait};
    use openzeppelin_access::ownable::OwnableComponent;
    use openzeppelin_upgrades::UpgradeableComponent;
    use starknet::{ContractAddress, EthAddress};

    component!(path: OwnableComponent, storage: ownable, event: OwnableEvent);
    component!(path: UpgradeableComponent, storage: upgradeable, event: UpgradeableEvent);

    #[abi(embed_v0)]
    impl OwnableMixinImpl = OwnableComponent::OwnableMixinImpl<ContractState>;
    impl OwnableInternalImpl = OwnableComponent::InternalImpl<ContractState>;

    // Upgradeable
    impl UpgradeableInternalImpl = UpgradeableComponent::InternalImpl<ContractState>;

    #[storage]
    struct Storage {
        l1_messages_sender: EthAddress,
        store_dispatcher: IFossilStoreDispatcher,
        #[substorage(v0)]
        ownable: OwnableComponent::Storage,
        #[substorage(v0)]
        upgradeable: UpgradeableComponent::Storage,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        #[flat]
        OwnableEvent: OwnableComponent::Event,
        #[flat]
        UpgradeableEvent: UpgradeableComponent::Event,
    }

    #[constructor]
    fn constructor(
        ref self: ContractState,
        l1_messages_sender: EthAddress,
        store_address: ContractAddress,
        owner: starknet::ContractAddress,
    ) {
        self.ownable.initializer(owner);
        self.l1_messages_sender.write(l1_messages_sender);
        self.store_dispatcher.write(IFossilStoreDispatcher { contract_address: store_address });
    }

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
            low: block_hash_low.try_into().unwrap(), high: block_hash_high.try_into().unwrap(),
        };
        let block_number: u64 = block_number_low.try_into().unwrap();
        assert!(
            from_address == self.l1_messages_sender.read().into(),
            "L1MessagesProxy: unauthorized sender",
        );
        let store = self.store_dispatcher.read();
        store.store_latest_blockhash_from_l1(block_number, block_hash);
    }

    #[abi(embed_v0)]
    impl IL1MessageProxyImpl of super::IL1MessageProxy<ContractState> {
        fn update_l1_messages_sender(ref self: ContractState, new_sender: EthAddress) {
            self.ownable.assert_only_owner();
            self.l1_messages_sender.write(new_sender);
        }

        fn upgrade(ref self: ContractState, new_implementation: starknet::ClassHash) {
            self.ownable.assert_only_owner();
            self.upgradeable.upgrade(new_implementation);
        }

        fn get_l1_messages_sender(ref self: ContractState) -> EthAddress {
            self.l1_messages_sender.read()
        }
    }
}
