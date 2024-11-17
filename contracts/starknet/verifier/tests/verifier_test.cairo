use snforge_std::{declare, ContractClassTrait, DeclareResultTrait};
use super::fixtures::calldata_default;

use verifier::groth16_verifier::{
    IRisc0Groth16VerifierBN254Dispatcher, IRisc0Groth16VerifierBN254DispatcherTrait
};

fn deploy() -> IRisc0Groth16VerifierBN254Dispatcher {
    let contract = declare("Risc0Groth16VerifierBN254").unwrap().contract_class();
    // Alternatively we could use `deploy_syscall` here
    let (contract_address, _) = contract.deploy(@array![]).unwrap();

    let msm_class = declare("UniversalECIP").unwrap().contract_class();
    println!("msm_class: {:?}", msm_class.class_hash);

    // Create a Dispatcher object that will allow interacting with the deployed contract
    let dispatcher = IRisc0Groth16VerifierBN254Dispatcher { contract_address };
    dispatcher
}

#[test]
fn test_verify_groth16_proof_bn254() {
    let dispatcher = deploy();
    let result = dispatcher.verify_groth16_proof_bn254(calldata_default().span());
    println!("result: {:?}", result);
}
