// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";

import {L1MessageSender} from "../src/L1MessageSender.sol";

import {StarknetMessagingLocal} from "../src/StarknetMessagingLocal.sol";

contract LocalSetup is Script {
    function setUp() public {}

    function run() public {
        string memory envType = vm.envString("ENV_TYPE");
        console.log("ENV_TYPE: %s", envType);

        uint256 deployerPrivateKey = vm.envUint("ACCOUNT_PRIVATE_KEY");

        vm.startBroadcast(deployerPrivateKey);

        // Deploy StarknetMessagingLocal contract
        address snMessagingAddress = address(new StarknetMessagingLocal());

        console.log("SN_MESSAGING: %s", snMessagingAddress);

        // Save the address to the JSON file
        vm.writeJson(
            vm.toString(snMessagingAddress),
            string.concat("logs/", "local_setup.json"),
            ".snMessaging_address"
        );

        // Deploy L1MessageSender contract
        address l1MessageSenderAddress = address(
            new L1MessageSender(snMessagingAddress)
        );

        // Save the address to the JSON file
        vm.writeJson(
            vm.toString(l1MessageSenderAddress),
            string.concat("logs/", "local_setup.json"),
            ".l1MessageSender_address"
        );

        vm.stopBroadcast();
    }
}
