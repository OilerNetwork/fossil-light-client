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
    fn update_min_update_interval(ref self: TContractState, min_update_interval: u64);
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
    fn restore(ref self: TContractState, block_number: u64);
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
        AvgFeesUpdated: AvgFeesUpdated,
        MinUpdateIntervalUpdated: MinUpdateIntervalUpdated,
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
    struct AvgFeesUpdated {
        timestamp: u64,
        avg_fee_fixed: felt252,
    }

    #[derive(Drop, starknet::Event)]
    struct MinUpdateIntervalUpdated {
        new_min_update_interval: u64,
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
            let min_update_interval = self.min_update_interval.read();
            let (latest_block_number, _) = self.latest_blockhash_from_l1.read();
            let actual_update_interval = block_number - latest_block_number;
            assert!(
                actual_update_interval >= min_update_interval,
                "Update interval: {} must be greater than or equal to the minimum update interval: {}",
                actual_update_interval,
                min_update_interval,
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

            let mut batch_state = self.mmr_batches.entry(journal.batch_index);

            // Update latest MMR block if the new one is more recent
            let current_latest_block = self.latest_mmr_block.read();
            if current_latest_block < journal.latest_mmr_block {
                self.latest_mmr_block.write(journal.latest_mmr_block);
            }

            // Update minimum MMR block tracking
            let current_min_block = self.min_mmr_block.read();
            let batch_first_block = journal.latest_mmr_block - journal.leaves_count + 1;

            if current_min_block != 0 {
                if batch_first_block < current_min_block {
                    self.min_mmr_block.write(batch_first_block);
                }
            } else {
                self.min_mmr_block.write(batch_first_block);
            }

            // Update batch state with journal data
            batch_state.latest_mmr_block_hash.write(journal.latest_mmr_block_hash);
            batch_state.leaves_count.write(journal.leaves_count);
            batch_state.root_hash.write(journal.root_hash);
            batch_state.first_block_parent_hash.write(journal.first_block_parent_hash);
            batch_state.latest_mmr_block.write(journal.latest_mmr_block);

            // Process average fees data
            for fee_entry in avg_fees {
                let mut stored_fee_entry = self.avg_fees.entry(*fee_entry.timestamp);

                if stored_fee_entry.data_points.read() == 0 {
                    // First entry for this timestamp - store directly
                    stored_fee_entry.data_points.write(*fee_entry.data_points);
                    stored_fee_entry.avg_fee.write(*fee_entry.avg_fee);

                    self
                        .emit(
                            AvgFeesUpdated {
                                timestamp: *fee_entry.timestamp, avg_fee_fixed: *fee_entry.avg_fee,
                            },
                        );
                } else {
                    // Merge with existing fee data using weighted average
                    let existing_points: UFixedPoint123x128 = (stored_fee_entry.data_points.read())
                        .into();
                    let existing_fee = UFixedPoint123x128StorePacking::unpack(
                        stored_fee_entry.avg_fee.read(),
                    );

                    let new_points: UFixedPoint123x128 = (*fee_entry.data_points).into();
                    let new_fee: UFixedPoint123x128 = UFixedPoint123x128StorePacking::unpack(
                        *fee_entry.avg_fee,
                    );

                    let total_points = existing_points + new_points;

                    // Calculate weighted average of fees
                    let weighted_avg_fee = (existing_fee * existing_points + new_fee * new_points)
                        / total_points;
                    let packed_weighted_fee = UFixedPoint123x128StorePacking::pack(
                        weighted_avg_fee,
                    );

                    // Update storage with merged data
                    stored_fee_entry.avg_fee.write(packed_weighted_fee);
                    stored_fee_entry
                        .data_points
                        .write(
                            total_points
                                .get_integer()
                                .try_into()
                                .expect('Failed to convert u128 to u64'),
                        );

                    self
                        .emit(
                            AvgFeesUpdated {
                                timestamp: *fee_entry.timestamp, avg_fee_fixed: packed_weighted_fee,
                            },
                        );
                }
            };

            // Emit MMR state update event
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

            // Only contract owner can update IPFS hash
            if verifier_caller == self.ownable.Ownable_owner.read() {
                batch_state.ipfs_hash.write(ipfs_hash.clone());
                self.emit(IPFSHashUpdated { batch_index: journal.batch_index, ipfs_hash });
            }
        }

        fn update_min_update_interval(ref self: ContractState, min_update_interval: u64) {
            self.ownable.assert_only_owner();
            self.min_update_interval.write(min_update_interval);
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

            // Return the packed value directly - the caller will unpack it
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

        fn restore(ref self: ContractState, block_number: u64) {
            self.latest_mmr_block.write(block_number);
        }
    }
}
