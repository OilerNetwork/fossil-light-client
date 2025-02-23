use fossil_store::{IFossilStoreDispatcher, IFossilStoreDispatcherTrait};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, declare, start_cheat_caller_address,
    stop_cheat_caller_address,
};
use super::fixtures::{calldata_default, invalid_proof, test_avg_fees, test_journal};

use verifier::{
    decode_journal, fossil_verifier::{IFossilVerifierDispatcher, IFossilVerifierDispatcherTrait},
    groth16_verifier::{
        IRisc0Groth16VerifierBN254Dispatcher, IRisc0Groth16VerifierBN254DispatcherTrait,
    },
};

fn l1_message_proxy_address() -> starknet::ContractAddress {
    starknet::contract_address_const::<'L1_MSG_SENDER'>()
}
fn OWNER() -> starknet::ContractAddress {
    starknet::contract_address_const::<'OWNER'>()
}

fn deploy() -> (IRisc0Groth16VerifierBN254Dispatcher, IFossilVerifierDispatcher) {
    let ecip_class = declare("UniversalECIP").unwrap().contract_class();
    let contract = declare("Risc0Groth16VerifierBN254").unwrap().contract_class();
    // Alternatively we could use `deploy_syscall` here
    let (groth16_verifier_address, _) = contract
        .deploy(@array![(*ecip_class.class_hash).into()])
        .unwrap();

    let (fossil_store_address, _) = declare("Store")
        .unwrap()
        .contract_class()
        .deploy(@array![OWNER().into()])
        .unwrap();

    let (verifier_address, _) = declare("FossilVerifier")
        .unwrap()
        .contract_class()
        .deploy(
            @array![groth16_verifier_address.into(), fossil_store_address.into(), OWNER().into()],
        )
        .unwrap();

    // Create a Dispatcher object that will allow interacting with the deployed contract
    let store_dispatcher = IFossilStoreDispatcher { contract_address: fossil_store_address };
    start_cheat_caller_address(store_dispatcher.contract_address, OWNER());
    store_dispatcher.initialize(verifier_address, l1_message_proxy_address(), 0);
    stop_cheat_caller_address(store_dispatcher.contract_address);
    (
        IRisc0Groth16VerifierBN254Dispatcher { contract_address: groth16_verifier_address },
        IFossilVerifierDispatcher { contract_address: verifier_address },
    )
}

#[test]
fn test_verify_groth16_proof_bn254() {
    let (groth16_verifier_dispatcher, _) = deploy();
    let mut calldata = calldata_default();
    let _ = calldata.pop_front();
    let (journal, fees) = decode_journal(
        groth16_verifier_dispatcher.verify_groth16_proof_bn254(calldata).unwrap(),
    );
    assert_eq!(journal, test_journal());
    assert_eq!(fees, test_avg_fees());
}

#[test]
fn test_verify_mmr_proof_first_batch() {
    let (_, verifier) = deploy();
    let IPFS_HASH: ByteArray = "IPFS_HASH_CID";
    start_cheat_caller_address(verifier.contract_address, OWNER());
    // First batch should succeed without checking batch link
    let result = verifier.verify_mmr_proof(calldata_default(), IPFS_HASH.into(), true);
    assert!(result);
}

#[test]
fn test_verify_mmr_proof_subsequent_batch() {
    let (_, verifier) = deploy();
    let IPFS_HASH: ByteArray = "IPFS_HASH_CID";
    start_cheat_caller_address(verifier.contract_address, OWNER());
    // Submit first batch
    verifier.verify_mmr_proof(calldata_default(), IPFS_HASH.clone(), true);

    // Submit second batch with correct linking
    let result = verifier.verify_mmr_proof(calldata_default(), IPFS_HASH, true);
    assert!(result);
}

#[test]
#[should_panic(expected: "Batch link mismatch")]
fn test_verify_mmr_proof_batch_link_mismatch() {
    let (_, verifier) = deploy();
    let IPFS_HASH: ByteArray = "IPFS_HASH_CID";
    start_cheat_caller_address(verifier.contract_address, OWNER());
    // First submit in build mode
    verifier.verify_mmr_proof(calldata_default(), IPFS_HASH.clone(), true);

    // Then update existing batch
    let result = verifier.verify_mmr_proof(calldata_default(), IPFS_HASH, false);
    assert!(result);
}

#[test]
#[should_panic(expected: 'not zero l0')]
fn test_verify_mmr_proof_invalid_proof() {
    let (_, verifier) = deploy();
    let IPFS_HASH: ByteArray = "IPFS_HASH_CID";
    start_cheat_caller_address(verifier.contract_address, OWNER());

    verifier.verify_mmr_proof(invalid_proof(), IPFS_HASH, true);
}

#[test]
fn test_get_verifier_address() {
    let (groth16_verifier_dispatcher, verifier) = deploy();

    assert_eq!(verifier.get_verifier_address(), groth16_verifier_dispatcher.contract_address);
}

#[test]
fn test_get_fossil_store_address() {
    let (_, verifier) = deploy();
    // We need to get the store address from deployment
    let store_address = verifier.get_fossil_store_address();
    assert!(store_address != starknet::contract_address_const::<0>());
}

#[test]
fn test_verify_mmr_proof_emits_event() {
    let (_, verifier) = deploy();
    let IPFS_HASH: ByteArray = "IPFS_HASH_CID";
    start_cheat_caller_address(verifier.contract_address, OWNER());
    // TODO: Set up event tracking
    let result = verifier.verify_mmr_proof(calldata_default(), IPFS_HASH, true);

    // TODO: Assert event was emitted with correct parameters
    assert!(result);
}
