use snforge_std::{ContractClassTrait, DeclareResultTrait, declare};
use super::fixtures::{calldata_default, test_avg_fees, test_journal};

use verifier::{
    decode_journal,
    groth16_verifier::{
        IRisc0Groth16VerifierBN254Dispatcher, IRisc0Groth16VerifierBN254DispatcherTrait,
    },
};

fn deploy() -> IRisc0Groth16VerifierBN254Dispatcher {
    let ecip_class = declare("UniversalECIP").unwrap().contract_class();
    let contract = declare("Risc0Groth16VerifierBN254").unwrap().contract_class();
    // Alternatively we could use `deploy_syscall` here
    let (contract_address, _) = contract.deploy(@array![(*ecip_class.class_hash).into()]).unwrap();

    // Create a Dispatcher object that will allow interacting with the deployed contract
    IRisc0Groth16VerifierBN254Dispatcher { contract_address }
}

#[test]
fn test_verify_groth16_proof_bn254() {
    let dispatcher = deploy();
    let mut calldata = calldata_default();
    let _ = calldata.pop_front();
    let (journal, fees) = decode_journal(dispatcher.verify_groth16_proof_bn254(calldata).unwrap());
    assert_eq!(journal, test_journal());
    assert_eq!(fees, test_avg_fees());
}
