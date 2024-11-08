// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";

import {L1MessageSender} from "../src/L1MessageSender.sol";

address constant SN_CORE = 0xc662c410C0ECf747543f5bA90660f6ABeBD9C8c4;

contract LocalSetup is Script {
    L1MessageSender public l1MessageSender;

    function setUp() public {}

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("ACCOUNT_PRIVATE_KEY");
        string memory json = "local_testing";

        vm.startBroadcast(deployerPrivateKey);

        l1MessageSender = new L1MessageSender(SN_CORE);
        console.log("L1MessageSender deployed at", address(l1MessageSender));
        vm.serializeString(json, "L1MessageSender_address", vm.toString(address(l1MessageSender)));

        vm.stopBroadcast();

         string memory data = vm.serializeBool(json, "success", true);

        string memory localLogs = "./logs/";
        vm.createDir(localLogs, true);
        vm.writeJson(data, string.concat(localLogs, "local_setup.json"));
    }
}
