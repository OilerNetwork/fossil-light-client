// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";

import {L1MessageSender} from "../src/L1MessageSender.sol";

import {StarknetMessagingLocal} from "../src/StarknetMessagingLocal.sol";

// address constant SN_CORE = 0xc662c410C0ECf747543f5bA90660f6ABeBD9C8c4;

contract LocalSetup is Script {
    function setUp() public {}

    function run() public{
        // uint256 deployerPrivateKey = vm.envUint("ACCOUNT_PRIVATE_KEY");
        uint256 deployerPrivateKey = 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80;
        
        string memory json = "local_testing";

        vm.startBroadcast(deployerPrivateKey);

        address snLocalAddress = address(new StarknetMessagingLocal());
        vm.serializeString(json, "snMessaging_address", vm.toString(snLocalAddress));

        address l1MessageSenderAddress = address(new L1MessageSender(snLocalAddress));
        vm.serializeString(json, "l1MessageSender_address", vm.toString(l1MessageSenderAddress));

        vm.stopBroadcast();

        string memory data = vm.serializeBool(json, "success", true);

        string memory localLogs = "logs/";
        vm.createDir(localLogs, true);
        vm.writeJson(data, string.concat(localLogs, "local_setup.json"));
    }
}
