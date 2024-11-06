// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {L1MessagesSender} from "../src/L1MessagesSender.sol";

address constant SN_CORE = 0xc662c410C0ECf747543f5bA90660f6ABeBD9C8c4;

contract L1MessagesSenderScript is Script {
    L1MessagesSender public l1MessagesSender;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        l1MessagesSender = new L1MessagesSender(SN_CORE, vm.envUint("L2_CONTRACT_ADDRESS"));

        vm.stopBroadcast();
    }
}
