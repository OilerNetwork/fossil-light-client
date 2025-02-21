use fossil_store::{IFossilStoreDispatcher, IFossilStoreDispatcherTrait};
use snforge_std::{
    ContractClassTrait, DeclareResultTrait, declare, start_cheat_caller_address_global,
};
use super::fixtures::{test_avg_fees_1, test_avg_fees_2, test_journal};

fn verifier_address() -> starknet::ContractAddress {
    starknet::contract_address_const::<'VERIFIER_ADDRESS'>()
}

fn l1_message_proxy_address() -> starknet::ContractAddress {
    starknet::contract_address_const::<'L1_MESSAGE_PROXY_ADDRESS'>()
}

const MIN_UPDATE_INTERVAL: u64 = 0;


fn deploy() -> IFossilStoreDispatcher {
    let contract = declare("Store").unwrap().contract_class();

    let (contract_address, _) = contract.deploy(@array![]).unwrap();

    // Create a Dispatcher object that will allow interacting with the deployed contract
    let dispatcher = IFossilStoreDispatcher { contract_address };
    dispatcher.initialize(verifier_address(), l1_message_proxy_address(), MIN_UPDATE_INTERVAL);
    dispatcher
}

#[test]
fn test_store_latest_blockhash_from_l1() {
    let dispatcher = deploy();

    let block_number = 100;
    let block_hash = 0x1234567890abcdef;

    start_cheat_caller_address_global(l1_message_proxy_address());
    dispatcher.store_latest_blockhash_from_l1(block_number, block_hash);

    let (stored_block_number, stored_block_hash) = dispatcher.get_latest_blockhash_from_l1();
    assert_eq!(stored_block_number, block_number);
    assert_eq!(stored_block_hash, block_hash);
}

#[test]
fn test_update_store_state_no_weighted_avg_fee() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address_global(verifier_address());
    dispatcher.update_store_state(test_journal(), test_avg_fees_1(), IPFS_HASH.clone());

    let timestamp_1 = test_avg_fees_1()[0].timestamp;
    let avg_fee_1 = dispatcher.get_avg_fee(*timestamp_1);

    assert_eq!(avg_fee_1, *test_avg_fees_1()[0].avg_fee);

    let mmr_state = dispatcher.get_mmr_state(test_journal().batch_index);
    assert_eq!(mmr_state.batch_index, test_journal().batch_index);
    assert_eq!(mmr_state.latest_mmr_block, test_journal().latest_mmr_block);
    assert_eq!(mmr_state.latest_mmr_block_hash, test_journal().latest_mmr_block_hash);
    assert_eq!(mmr_state.root_hash, test_journal().root_hash);
    assert_eq!(mmr_state.leaves_count, test_journal().leaves_count);
    assert_eq!(mmr_state.ipfs_hash, IPFS_HASH);
}

#[test]
fn test_update_store_state_weighted_avg_fee() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    let avg_fees_1 = test_avg_fees_1();

    start_cheat_caller_address_global(verifier_address());
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

#[test]
fn test_get_latest_mmr_block() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address_global(verifier_address());
    dispatcher.update_store_state(test_journal(), test_avg_fees_1(), IPFS_HASH.clone());

    let mmr_block = dispatcher.get_latest_mmr_block();
    assert_eq!(mmr_block, test_journal().latest_mmr_block);
}

#[test]
fn test_get_min_mmr_block() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address_global(verifier_address());
    dispatcher.update_store_state(test_journal(), test_avg_fees_1(), IPFS_HASH.clone());

    let expected_min_mmr_block = test_journal().latest_mmr_block - test_journal().leaves_count + 1;

    let min_mmr_block = dispatcher.get_min_mmr_block();
    assert_eq!(min_mmr_block, expected_min_mmr_block);
}

#[test]
fn test_get_batch_last_block_link() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address_global(verifier_address());
    let mut journal = test_journal();
    journal.batch_index += 1;
    dispatcher.update_store_state(journal, test_avg_fees_1(), IPFS_HASH.clone());

    let batch_last_block_link = dispatcher.get_batch_last_block_link(test_journal().batch_index);
    assert_eq!(batch_last_block_link, test_journal().first_block_parent_hash);
}

#[test]
fn test_get_batch_first_block_parent_hash() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address_global(verifier_address());
    let mut journal = test_journal();
    journal.batch_index -= 1;
    dispatcher.update_store_state(journal, test_avg_fees_1(), IPFS_HASH.clone());

    let batch_first_block_parent_hash = dispatcher
        .get_batch_first_block_parent_hash(test_journal().batch_index);
    assert_eq!(batch_first_block_parent_hash, test_journal().latest_mmr_block_hash);
}

#[test]
fn test_get_avg_fee() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address_global(verifier_address());
    dispatcher.update_store_state(test_journal(), test_avg_fees_1(), IPFS_HASH.clone());

    let timestamp_1 = test_avg_fees_1()[0].timestamp;
    let avg_fee = dispatcher.get_avg_fee(*timestamp_1);
    assert_eq!(avg_fee, *test_avg_fees_1()[0].avg_fee);
}

#[test]
fn test_get_avg_fees_in_range() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address_global(verifier_address());
    dispatcher.update_store_state(test_journal(), test_avg_fees_1(), IPFS_HASH.clone());

    let start_timestamp = test_avg_fees_1()[0].timestamp;
    let end_timestamp = test_avg_fees_1()[3].timestamp;

    let expected_avg_fees = array![
        *test_avg_fees_1()[0].avg_fee,
        *test_avg_fees_1()[1].avg_fee,
        *test_avg_fees_1()[2].avg_fee,
        *test_avg_fees_1()[3].avg_fee,
    ];

    let avg_fees = dispatcher.get_avg_fees_in_range(*start_timestamp, *end_timestamp);
    assert_eq!(avg_fees, expected_avg_fees);
}
