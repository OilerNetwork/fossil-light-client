#[starknet::interface]
pub trait IFossilStore<TContractState> {
    fn initialize(
        ref self: TContractState,
        verifier_address: starknet::ContractAddress,
        l1_message_proxy_address: starknet::ContractAddress,
        min_update_interval: u64,
    );
    fn store_latest_blockhash_from_l1(ref self: TContractState, block_number: u64, blockhash: u256);
    fn update_store_state(
        ref self: TContractState,
        verifier_caller: starknet::ContractAddress,
        journal: verifier::Journal,
        avg_fees: Span<verifier::AvgFees>,
        ipfs_hash: ByteArray,
    );
    fn get_latest_blockhash_from_l1(self: @TContractState) -> (u64, u256);
    fn get_mmr_state(self: @TContractState, batch_index: u64) -> Store::MMRSnapshot;
    fn get_latest_mmr_block(self: @TContractState) -> u64;
    fn get_min_mmr_block(self: @TContractState) -> u64;
    fn get_batch_last_block_link(self: @TContractState, batch_index: u64) -> u256;
    fn get_batch_first_block_parent_hash(self: @TContractState, batch_index: u64) -> u256;
    fn get_avg_fee(self: @TContractState, timestamp: u64) -> felt252;
    fn get_avg_fees_in_range(
        self: @TContractState, start_timestamp: u64, end_timestamp: u64,
    ) -> Array<felt252>;
    fn upgrade(ref self: TContractState, new_class_hash: starknet::ClassHash);
}

#[starknet::contract]
pub mod Store {
    use core::starknet::storage::{
        Map, StoragePathEntry, StoragePointerReadAccess, StoragePointerWriteAccess,
    };
    use fp::{UFixedPoint123x128, UFixedPoint123x128Impl, UFixedPoint123x128StorePacking};
    use openzeppelin_access::ownable::OwnableComponent;
    use openzeppelin_upgrades::UpgradeableComponent;

    component!(path: OwnableComponent, storage: ownable, event: OwnableEvent);
    component!(path: UpgradeableComponent, storage: upgradeable, event: UpgradeableEvent);

    #[abi(embed_v0)]
    impl OwnableMixinImpl = OwnableComponent::OwnableMixinImpl<ContractState>;
    impl OwnableInternalImpl = OwnableComponent::InternalImpl<ContractState>;

    // Upgradeable
    impl UpgradeableInternalImpl = UpgradeableComponent::InternalImpl<ContractState>;

    const HOUR_IN_SECONDS: u64 = 3600;

    #[starknet::storage_node]
    pub(crate) struct MMRBatch {
        latest_mmr_block: u64,
        latest_mmr_block_hash: u256,
        leaves_count: u64,
        root_hash: u256,
        first_block_parent_hash: u256,
        ipfs_hash: ByteArray,
    }

    #[starknet::storage_node]
    pub struct AvgFees {
        data_points: u64,
        avg_fee: felt252,
    }

    #[derive(Drop, Serde, Debug)]
    pub struct MMRSnapshot {
        pub batch_index: u64,
        pub latest_mmr_block: u64,
        pub latest_mmr_block_hash: u256,
        pub root_hash: u256,
        pub leaves_count: u64,
        pub ipfs_hash: ByteArray,
    }

    #[storage]
    struct Storage {
        initialized: bool,
        verifier_address: starknet::ContractAddress,
        l1_message_proxy_address: starknet::ContractAddress,
        latest_blockhash_from_l1: (u64, u256),
        latest_mmr_block: u64,
        mmr_batches: Map<u64, MMRBatch>,
        min_mmr_block: u64,
        min_update_interval: u64,
        avg_fees: Map<u64, AvgFees>,
        #[substorage(v0)]
        ownable: OwnableComponent::Storage,
        #[substorage(v0)]
        upgradeable: UpgradeableComponent::Storage,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        LatestBlockhashFromL1Stored: LatestBlockhashFromL1Stored,
        MmrStateUpdated: MmrStateUpdated,
        IPFSHashUpdated: IPFSHashUpdated,
        #[flat]
        OwnableEvent: OwnableComponent::Event,
        #[flat]
        UpgradeableEvent: UpgradeableComponent::Event,
    }

    #[derive(Drop, starknet::Event)]
    struct LatestBlockhashFromL1Stored {
        block_number: u64,
        blockhash: u256,
    }

    #[derive(Drop, starknet::Event)]
    struct IPFSHashUpdated {
        batch_index: u64,
        ipfs_hash: ByteArray,
    }

    #[derive(Drop, starknet::Event)]
    struct MmrStateUpdated {
        batch_index: u64,
        latest_mmr_block: u64,
        latest_mmr_block_hash: u256,
        leaves_count: u64,
        root_hash: u256,
    }

    #[constructor]
    fn constructor(ref self: ContractState, owner: starknet::ContractAddress) {
        self.ownable.initializer(owner);
    }

    #[abi(embed_v0)]
    impl FossilStoreImpl of super::IFossilStore<ContractState> {
        fn initialize(
            ref self: ContractState,
            verifier_address: starknet::ContractAddress,
            l1_message_proxy_address: starknet::ContractAddress,
            min_update_interval: u64,
        ) {
            self.ownable.assert_only_owner();
            assert!(!self.initialized.read(), "Contract already initialized");
            self.initialized.write(true);
            self.verifier_address.write(verifier_address);
            self.l1_message_proxy_address.write(l1_message_proxy_address);
            self.min_update_interval.write(min_update_interval);
        }

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

        fn get_latest_blockhash_from_l1(self: @ContractState) -> (u64, u256) {
            self.latest_blockhash_from_l1.read()
        }

        fn update_store_state(
            ref self: ContractState,
            verifier_caller: starknet::ContractAddress,
            journal: verifier::Journal,
            avg_fees: Span<verifier::AvgFees>,
            ipfs_hash: ByteArray,
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
            curr_state.first_block_parent_hash.write(journal.first_block_parent_hash);

            for avg_fee in avg_fees {
                let mut curr_avg_fee = self.avg_fees.entry(*avg_fee.timestamp);
                if curr_avg_fee.data_points.read() == 0 {
                    curr_avg_fee.data_points.write(*avg_fee.data_points);
                    let avg_fee_fixed_point: UFixedPoint123x128 = (*avg_fee.avg_fee).into();
                    curr_avg_fee
                        .avg_fee
                        .write(UFixedPoint123x128StorePacking::pack(avg_fee_fixed_point));
                } else {
                    let existing_points_fixed: UFixedPoint123x128 = curr_avg_fee
                        .data_points
                        .read()
                        .into();
                    let existing_fee_fixed: UFixedPoint123x128 =
                        UFixedPoint123x128StorePacking::unpack(
                        curr_avg_fee.avg_fee.read(),
                    );

                    let avg_fee_data_points_fixed: UFixedPoint123x128 = (*avg_fee.data_points)
                        .into();
                    let new_data_points_fixed: UFixedPoint123x128 = existing_points_fixed
                        + avg_fee_data_points_fixed;

                    let avg_fee_fixed: UFixedPoint123x128 = (*avg_fee.avg_fee).into();
                    let new_avg_fee_fixed: UFixedPoint123x128 = (existing_fee_fixed
                        * existing_points_fixed
                        + avg_fee_fixed * avg_fee_data_points_fixed)
                        / new_data_points_fixed;

                    curr_avg_fee
                        .avg_fee
                        .write(UFixedPoint123x128StorePacking::pack(new_avg_fee_fixed));
                    curr_avg_fee
                        .data_points
                        .write(
                            new_data_points_fixed
                                .get_integer()
                                .try_into()
                                .expect('Failed to convert u128 to u64'),
                        );
                }
            };

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

            if verifier_caller == self.ownable.Ownable_owner.read() {
                curr_state.ipfs_hash.write(ipfs_hash.clone());
                self.emit(IPFSHashUpdated { batch_index: journal.batch_index, ipfs_hash });
            }
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
            if batch_index == 0 {
                return 0;
            }
            let curr_state = self.mmr_batches.entry(batch_index - 1);
            curr_state.latest_mmr_block_hash.read()
        }

        fn get_avg_fee(self: @ContractState, timestamp: u64) -> felt252 {
            assert!(timestamp % HOUR_IN_SECONDS == 0, "Timestamp must be a multiple of 3600");
            let curr_state = self.avg_fees.entry(timestamp);
            curr_state.avg_fee.read()
        }

        fn get_avg_fees_in_range(
            self: @ContractState, start_timestamp: u64, end_timestamp: u64,
        ) -> Array<felt252> {
            assert!(
                start_timestamp <= end_timestamp,
                "Start timestamp must be less than or equal to end timestamp",
            );
            assert!(
                start_timestamp % HOUR_IN_SECONDS == 0,
                "Start timestamp must be a multiple of 3600",
            );
            assert!(
                end_timestamp % HOUR_IN_SECONDS == 0, "End timestamp must be a multiple of 3600",
            );

            let mut fees: Array<felt252> = array![];

            let mut i = start_timestamp;
            while i <= end_timestamp {
                fees.append(self.get_avg_fee(i));
                i += HOUR_IN_SECONDS;
            };
            fees
        }

        fn upgrade(ref self: ContractState, new_class_hash: starknet::ClassHash) {
            self.ownable.assert_only_owner();
            self.upgradeable.upgrade(new_class_hash);
        }
    }
}
