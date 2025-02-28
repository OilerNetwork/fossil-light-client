#[starknet::interface]
pub trait IFossilVerifier<TContractState> {
    fn verify_mmr_proof(
        ref self: TContractState, proof: Span<felt252>, ipfs_hash: ByteArray, is_build: bool,
    ) -> bool;
    fn update_verifier_address(
        ref self: TContractState, new_verifier_address: starknet::ContractAddress,
    );
    fn get_verifier_address(self: @TContractState) -> starknet::ContractAddress;
    fn get_fossil_store_address(self: @TContractState) -> starknet::ContractAddress;
    fn upgrade(ref self: TContractState, new_class_hash: starknet::ClassHash);
}

#[starknet::contract]
mod FossilVerifier {
    use core::num::traits::Zero;
    use fossil_store::{IFossilStoreDispatcher, IFossilStoreDispatcherTrait};
    use openzeppelin_access::ownable::OwnableComponent;
    use openzeppelin_upgrades::UpgradeableComponent;
    use verifier::decode_journal;
    use verifier::groth16_verifier::{
        IRisc0Groth16VerifierBN254Dispatcher, IRisc0Groth16VerifierBN254DispatcherTrait,
    };

    component!(path: OwnableComponent, storage: ownable, event: OwnableEvent);
    component!(path: UpgradeableComponent, storage: upgradeable, event: UpgradeableEvent);

    // Ownable Mixin
    #[abi(embed_v0)]
    impl OwnableMixinImpl = OwnableComponent::OwnableMixinImpl<ContractState>;
    impl OwnableInternalImpl = OwnableComponent::InternalImpl<ContractState>;

    // Upgradeable
    impl UpgradeableInternalImpl = UpgradeableComponent::InternalImpl<ContractState>;


    #[storage]
    struct Storage {
        bn254_verifier: IRisc0Groth16VerifierBN254Dispatcher,
        fossil_store: IFossilStoreDispatcher,
        #[substorage(v0)]
        ownable: OwnableComponent::Storage,
        #[substorage(v0)]
        upgradeable: UpgradeableComponent::Storage,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        MmrProofVerified: MmrProofVerified,
        #[flat]
        OwnableEvent: OwnableComponent::Event,
        #[flat]
        UpgradeableEvent: UpgradeableComponent::Event,
    }

    #[derive(Drop, starknet::Event)]
    struct MmrProofVerified {
        batch_index: u64,
        latest_mmr_block: u64,
        new_leaves_count: u64,
        new_mmr_root: u256,
    }

    #[constructor]
    fn constructor(
        ref self: ContractState,
        verifier_address: starknet::ContractAddress,
        fossil_store_address: starknet::ContractAddress,
        owner: starknet::ContractAddress,
    ) {
        self
            .bn254_verifier
            .write(IRisc0Groth16VerifierBN254Dispatcher { contract_address: verifier_address });
        self.fossil_store.write(IFossilStoreDispatcher { contract_address: fossil_store_address });
        self.ownable.initializer(owner);
    }

    #[abi(embed_v0)]
    impl FossilVerifierImpl of super::IFossilVerifier<ContractState> {
        fn verify_mmr_proof(
            ref self: ContractState, mut proof: Span<felt252>, ipfs_hash: ByteArray, is_build: bool,
        ) -> bool {
            let _ = proof.pop_front();
            let journal = self
                .bn254_verifier
                .read()
                .verify_groth16_proof_bn254(proof)
                .expect('Failed to verify proof');

            let (journal, avg_fees) = decode_journal(journal);

            let fossil_store = self.fossil_store.read();

            if is_build {
                let batch_link = fossil_store.get_batch_last_block_link(journal.batch_index);
                // If the batch link is zero, it means that the batch is the first batch, and we
                // don't need to check the batch link
                if !batch_link.is_zero() {
                    assert!(batch_link == journal.latest_mmr_block_hash, "Batch link mismatch");
                }
            } else {
                let current_batch_state = fossil_store.get_mmr_state(journal.batch_index);
                let mut batch_link = 0;

                if current_batch_state.leaves_count > 0 {
                    batch_link = current_batch_state.latest_mmr_block_hash;
                } else {
                    batch_link = fossil_store
                        .get_batch_first_block_parent_hash(journal.batch_index);
                }

                assert!(batch_link == journal.first_block_parent_hash, "Batch link mismatch");
            }

            let verifier_caller = starknet::get_caller_address();
            fossil_store.update_store_state(verifier_caller, journal, avg_fees.span(), ipfs_hash);

            self
                .emit(
                    MmrProofVerified {
                        batch_index: journal.batch_index,
                        latest_mmr_block: journal.latest_mmr_block,
                        new_leaves_count: journal.leaves_count,
                        new_mmr_root: journal.root_hash,
                    },
                );

            true
        }

        fn update_verifier_address(
            ref self: ContractState, new_verifier_address: starknet::ContractAddress,
        ) {
            self.ownable.assert_only_owner();
            self
                .bn254_verifier
                .write(
                    IRisc0Groth16VerifierBN254Dispatcher { contract_address: new_verifier_address },
                );
        }

        fn get_verifier_address(self: @ContractState) -> starknet::ContractAddress {
            self.bn254_verifier.read().contract_address
        }

        fn get_fossil_store_address(self: @ContractState) -> starknet::ContractAddress {
            self.fossil_store.read().contract_address
        }

        fn upgrade(ref self: ContractState, new_class_hash: starknet::ClassHash) {
            self.ownable.assert_only_owner();
            self.upgradeable.upgrade(new_class_hash);
        }
    }
}
