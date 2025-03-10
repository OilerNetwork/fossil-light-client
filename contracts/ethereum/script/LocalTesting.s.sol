// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";

import {L1MessageSender} from "../src/L1MessageSender.sol";

import {StarknetMessagingLocal} from "../src/StarknetMessagingLocal.sol";

contract LocalSetup is Script {
    function setUp() public {}

    function run() public{
        string memory envType = vm.envString("ENV_TYPE");
        console.log("ENV_TYPE: %s", envType);

        uint256 deployerPrivateKey = vm.envUint("ACCOUNT_PRIVATE_KEY");
        
        string memory json = "local_testing";

        vm.startBroadcast(deployerPrivateKey);

        address snMessagingAddress = vm.envAddress("SN_MESSAGING");

        if (keccak256(bytes(envType)) == keccak256(bytes("docker")) || keccak256(bytes(envType)) == keccak256(bytes("local"))) {
            snMessagingAddress = address(new StarknetMessagingLocal());
        } else {
            snMessagingAddress = vm.envAddress("SN_MESSAGING");
        }

        console.log("SN_MESSAGING: %s", snMessagingAddress);


        vm.serializeString(json, "snMessaging_address", vm.toString(snMessagingAddress));

        address l1MessageSenderAddress = address(new L1MessageSender(snMessagingAddress));
        vm.serializeString(json, "l1MessageSender_address", vm.toString(l1MessageSenderAddress));

        vm.stopBroadcast();
    }
}
