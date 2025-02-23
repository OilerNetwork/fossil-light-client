use fossil_store::{IFossilStoreDispatcher, IFossilStoreDispatcherTrait};
use snforge_std::{ContractClassTrait, DeclareResultTrait, declare, start_cheat_caller_address};
use super::fixtures::{test_avg_fees_1, test_avg_fees_2, test_journal};

fn verifier_address() -> starknet::ContractAddress {
    starknet::contract_address_const::<'VERIFIER_ADDRESS'>()
}

fn l1_message_proxy_address() -> starknet::ContractAddress {
    starknet::contract_address_const::<'L1_MESSAGE_PROXY_ADDRESS'>()
}

const MIN_UPDATE_INTERVAL: u64 = 10;
fn OWNER() -> starknet::ContractAddress {
    starknet::contract_address_const::<'OWNER'>()
}


fn deploy() -> IFossilStoreDispatcher {
    let contract = declare("Store").unwrap().contract_class();

    let (contract_address, _) = contract.deploy(@array![OWNER().into()]).unwrap();

    // Create a Dispatcher object that will allow interacting with the deployed contract
    let dispatcher = IFossilStoreDispatcher { contract_address };
    start_cheat_caller_address(contract_address, OWNER());
    dispatcher.initialize(verifier_address(), l1_message_proxy_address(), MIN_UPDATE_INTERVAL);
    dispatcher
}

#[test]
fn test_store_latest_blockhash_from_l1() {
    let dispatcher = deploy();

    let block_number = 100;
    let block_hash = 0x1234567890abcdef;

    start_cheat_caller_address(dispatcher.contract_address, l1_message_proxy_address());
    dispatcher.store_latest_blockhash_from_l1(block_number, block_hash);

    let (stored_block_number, stored_block_hash) = dispatcher.get_latest_blockhash_from_l1();
    assert_eq!(stored_block_number, block_number);
    assert_eq!(stored_block_hash, block_hash);
}

#[test]
fn test_update_store_state_no_weighted_avg_fee() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());
    dispatcher.update_store_state(OWNER(), test_journal(), test_avg_fees_1(), IPFS_HASH.clone());

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

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());
    dispatcher.update_store_state(OWNER(), test_journal(), avg_fees_1, IPFS_HASH.clone());

    let timestamp_1 = avg_fees_1[0].timestamp;
    let timestamp_2 = avg_fees_1[1].timestamp;

    let avg_fee_1 = dispatcher.get_avg_fee(*timestamp_1);
    let avg_fee_2 = dispatcher.get_avg_fee(*timestamp_2);

    assert_eq!(avg_fee_1, *avg_fees_1[0].avg_fee);
    assert_eq!(avg_fee_2, *avg_fees_1[1].avg_fee);

    let avg_fees_2 = test_avg_fees_2();
    dispatcher.update_store_state(OWNER(), test_journal(), avg_fees_2, IPFS_HASH);

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

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());
    dispatcher.update_store_state(OWNER(), test_journal(), test_avg_fees_1(), IPFS_HASH.clone());

    let mmr_block = dispatcher.get_latest_mmr_block();
    assert_eq!(mmr_block, test_journal().latest_mmr_block);
}

#[test]
fn test_get_min_mmr_block() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());
    dispatcher.update_store_state(OWNER(), test_journal(), test_avg_fees_1(), IPFS_HASH.clone());

    let expected_min_mmr_block = test_journal().latest_mmr_block - test_journal().leaves_count + 1;

    let min_mmr_block = dispatcher.get_min_mmr_block();
    assert_eq!(min_mmr_block, expected_min_mmr_block);
}

#[test]
fn test_get_batch_last_block_link() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());
    let mut journal = test_journal();
    journal.batch_index += 1;
    dispatcher.update_store_state(OWNER(), journal, test_avg_fees_1(), IPFS_HASH.clone());

    let batch_last_block_link = dispatcher.get_batch_last_block_link(test_journal().batch_index);
    assert_eq!(batch_last_block_link, test_journal().first_block_parent_hash);
}

#[test]
fn test_get_batch_first_block_parent_hash() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());
    let mut journal = test_journal();
    journal.batch_index -= 1;
    dispatcher.update_store_state(OWNER(), journal, test_avg_fees_1(), IPFS_HASH.clone());

    let batch_first_block_parent_hash = dispatcher
        .get_batch_first_block_parent_hash(test_journal().batch_index);
    assert_eq!(batch_first_block_parent_hash, test_journal().latest_mmr_block_hash);
}

#[test]
fn test_get_avg_fee() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());
    dispatcher.update_store_state(OWNER(), test_journal(), test_avg_fees_1(), IPFS_HASH.clone());

    let timestamp_1 = test_avg_fees_1()[0].timestamp;
    let avg_fee = dispatcher.get_avg_fee(*timestamp_1);
    assert_eq!(avg_fee, *test_avg_fees_1()[0].avg_fee);
}

#[test]
fn test_get_avg_fees_in_range() {
    let dispatcher = deploy();

    let IPFS_HASH = "IPFS_HASH_CID";

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());
    dispatcher.update_store_state(OWNER(), test_journal(), test_avg_fees_1(), IPFS_HASH.clone());

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

#[test]
#[should_panic(expected: "Contract already initialized")]
fn test_double_initialization() {
    let dispatcher = deploy();

    // Attempt to initialize again
    dispatcher.initialize(verifier_address(), l1_message_proxy_address(), MIN_UPDATE_INTERVAL);
}

#[test]
#[should_panic(expected: "Only L1 Message Proxy can store latest blockhash from L1")]
fn test_unauthorized_blockhash_update() {
    let dispatcher = deploy();
    // Don't start cheating caller address, attempt update with default caller
    dispatcher.store_latest_blockhash_from_l1(100, 0x1234567890abcdef);
}

#[test]
#[should_panic(expected: "Only Fossil Verifier can update MMR state")]
fn test_unauthorized_mmr_state_update() {
    let dispatcher = deploy();
    // Don't start cheating caller address, attempt update with default caller
    dispatcher.update_store_state(OWNER(), test_journal(), test_avg_fees_1(), "IPFS_HASH_CID");
}

#[test]
#[should_panic(
    expected: "Update interval: 9 must be greater than or equal to the minimum update interval: 10",
)]
fn test_min_update_interval_violation() {
    let dispatcher = deploy();

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());

    // First update
    let mut journal = test_journal();
    dispatcher.update_store_state(OWNER(), journal, test_avg_fees_1(), "IPFS_HASH_CID");

    // Second update too soon
    journal.latest_mmr_block += MIN_UPDATE_INTERVAL - 1;
    dispatcher.update_store_state(OWNER(), journal, test_avg_fees_1(), "IPFS_HASH_CID");
}

#[test]
#[should_panic(expected: "Timestamp must be a multiple of 3600")]
fn test_invalid_timestamp_get_avg_fee() {
    let dispatcher = deploy();

    // Try to get fee with non-hourly timestamp
    dispatcher.get_avg_fee(1234); // Not multiple of 3600
}

#[test]
#[should_panic(expected: "Start timestamp must be less than or equal to end timestamp")]
fn test_invalid_timestamp_range() {
    let dispatcher = deploy();

    // Try to get fees with end before start
    let end_timestamp = 3600; // 1 hour
    let start_timestamp = 7200; // 2 hours
    dispatcher.get_avg_fees_in_range(start_timestamp, end_timestamp);
}

#[test]
fn test_empty_batch_first_block_parent_hash() {
    let dispatcher = deploy();

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());

    // Get parent hash for non-existent batch should return 0
    let parent_hash = dispatcher.get_batch_first_block_parent_hash(0);
    assert_eq!(parent_hash, 0, "Empty batch should return zero hash");
}

#[test]
fn test_empty_batch_mmr_state() {
    let dispatcher = deploy();

    // Get MMR state for non-existent batch
    let mmr_state = dispatcher.get_mmr_state(999);

    assert_eq!(mmr_state.latest_mmr_block, 0);
    assert_eq!(mmr_state.latest_mmr_block_hash, 0);
    assert_eq!(mmr_state.leaves_count, 0);
    assert_eq!(mmr_state.root_hash, 0);
    assert_eq!(mmr_state.ipfs_hash, "");
}

#[test]
fn test_weighted_average_fee_calculation() {
    let dispatcher = deploy();

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());

    // Create test data with known weighted average result
    let timestamp: u64 = 3600; // 1 hour
    let mut avg_fees = array![
        verifier::AvgFees { timestamp, avg_fee: 100, data_points: 10 },
        verifier::AvgFees { timestamp, avg_fee: 200, data_points: 20 },
    ];

    // First update
    dispatcher.update_store_state(OWNER(), test_journal(), avg_fees.span(), "IPFS_HASH_CID");

    // Expected weighted average: (100 * 10 + 200 * 20) / (10 + 20) = 166.67 ≈ 166
    let fee = dispatcher.get_avg_fee(timestamp);
    assert_eq!(fee, 166, "Incorrect weighted average calculation");
}

#[test]
fn test_min_mmr_block_updates() {
    let dispatcher = deploy();

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());

    // First update
    let mut journal = test_journal();
    journal.latest_mmr_block = 1000;
    journal.leaves_count = 100;
    dispatcher.update_store_state(OWNER(), journal, test_avg_fees_1(), "IPFS_HASH_CID");

    // Expected min block: 1000 - 100 + 1 = 901
    assert_eq!(dispatcher.get_min_mmr_block(), 901);

    // Second update with lower min block
    journal.latest_mmr_block = 1500;
    journal.leaves_count = 700;
    dispatcher.update_store_state(OWNER(), journal, test_avg_fees_1(), "IPFS_HASH_CID");

    // Expected min block: 1500 - 700 + 1 = 801
    assert_eq!(dispatcher.get_min_mmr_block(), 801);
}

#[test]
fn test_batch_linking_sequence() {
    let dispatcher = deploy();

    start_cheat_caller_address(dispatcher.contract_address, verifier_address());

    // First batch
    let mut journal = test_journal();
    journal.batch_index = 1;
    journal.latest_mmr_block_hash = 0x1111;
    dispatcher.update_store_state(OWNER(), journal, test_avg_fees_1(), "IPFS_HASH_CID");

    // Second batch
    journal.batch_index = 2;
    journal.first_block_parent_hash = 0x1111;
    journal.latest_mmr_block_hash = 0x2222;
    dispatcher.update_store_state(OWNER(), journal, test_avg_fees_1(), "IPFS_HASH_CID");

    // Verify links
    assert_eq!(dispatcher.get_batch_first_block_parent_hash(2), 0x1111);
    assert_eq!(dispatcher.get_batch_last_block_link(1), 0x1111);
}
