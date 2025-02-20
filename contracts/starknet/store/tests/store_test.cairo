use fossil_store::{IFossilStoreDispatcher, IFossilStoreDispatcherTrait};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, declare, start_cheat_caller_address_global,
};
use super::fixtures::{test_avg_fees_1, test_avg_fees_2, test_journal};


fn deploy() -> IFossilStoreDispatcher {
    let contract = declare("Store").unwrap().contract_class();

    let (contract_address, _) = contract.deploy(@array![]).unwrap();

    // Create a Dispatcher object that will allow interacting with the deployed contract
    IFossilStoreDispatcher { contract_address }
}

#[test]
fn test_update_store_state() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    let avg_fees_1 = test_avg_fees_1();

    start_cheat_caller_address_global(starknet::contract_address_const::<0>());
    dispatcher.update_store_state(test_journal(), avg_fees_1, IPFS_HASH.clone());

    let timestamp_1 = avg_fees_1[0].timestamp;
    let timestamp_2 = avg_fees_1[1].timestamp;

    let avg_fee_1 = dispatcher.get_avg_fee(*timestamp_1);
    let avg_fee_2 = dispatcher.get_avg_fee(*timestamp_2);

    assert_eq!(avg_fee_1, *avg_fees_1[0].avg_fee);
    assert_eq!(avg_fee_2, *avg_fees_1[1].avg_fee);

    let avg_fees_2 = test_avg_fees_2();
    dispatcher.update_store_state(test_journal(), avg_fees_2, IPFS_HASH);

    let updated_fee = dispatcher.get_avg_fee(*timestamp_1);

    let expected_fee = (*avg_fees_2[0].avg_fee * *avg_fees_2[0].data_points
        + *avg_fees_1[0].avg_fee * *avg_fees_1[0].data_points)
        / (*avg_fees_2[0].data_points + *avg_fees_1[0].data_points);
    assert_eq!(updated_fee, expected_fee);
}
