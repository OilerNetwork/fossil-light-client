// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "forge-std/Script.sol";

import {L1MessageSender} from "../src/L1MessageSender.sol";

/**
 * @notice A simple script to send a message to Starknet.
 */
contract FinalizedBlockHash is Script {
    uint256 _privateKey;
    address _l1MessageSenderAddress;
    uint256 _l2ContractAddress;

    function setUp() public {
        _privateKey = vm.envUint("ACCOUNT_PRIVATE_KEY");
        _l1MessageSenderAddress = vm.envAddress("L1_MESSAGE_SENDER");
        _l2ContractAddress = vm.envUint("L2_MSG_PROXY");
    }

    function run() public {
        vm.startBroadcast(_privateKey);
        L1MessageSender(_l1MessageSenderAddress).sendFinalizedBlockHashToL2{value: 30000}(_l2ContractAddress);

        vm.stopBroadcast();
    }
}
