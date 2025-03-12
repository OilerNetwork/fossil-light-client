// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";

import {L1MessageSender} from "../src/L1MessageSender.sol";

contract ExternalMessagingSetup is Script {
    function setUp() public {}

    function run() public {
        string memory envType = vm.envString("ENV_TYPE");
        console.log("ENV_TYPE: %s", envType);

        uint256 deployerPrivateKey = vm.envUint("ACCOUNT_PRIVATE_KEY");
        address snMessagingAddress = vm.envAddress("SN_MESSAGING");
        
        console.log("Using SN_MESSAGING: %s", snMessagingAddress);

        vm.startBroadcast(deployerPrivateKey);

        // Deploy L1MessageSender contract with the provided StarknetMessaging address
        address l1MessageSenderAddress = address(new L1MessageSender(snMessagingAddress));
        console.log("L1MessageSender deployed at: %s", l1MessageSenderAddress);
        
        // Save the addresses to the JSON file
        vm.writeJson(
            vm.toString(snMessagingAddress),
            string.concat("logs/", "external_setup.json"),
            ".snMessaging_address"
        );
        
        vm.writeJson(
            vm.toString(l1MessageSenderAddress),
            string.concat("logs/", "external_setup.json"),
            ".l1MessageSender_address"
        );

        vm.stopBroadcast();
    }
} 