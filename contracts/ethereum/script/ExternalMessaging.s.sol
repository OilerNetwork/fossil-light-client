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
        
        // Create a JSON object in memory first
        string memory jsonObj = '{"snMessaging_address":"';
        jsonObj = string.concat(jsonObj, vm.toString(snMessagingAddress));
        jsonObj = string.concat(jsonObj, '","l1MessageSender_address":"');
        jsonObj = string.concat(jsonObj, vm.toString(l1MessageSenderAddress));
        jsonObj = string.concat(jsonObj, '"}');
        
        // Write the entire JSON object at once
        vm.writeFile(
            string.concat("logs/", "external_setup.json"),
            jsonObj
        );

        vm.stopBroadcast();
    }
} 