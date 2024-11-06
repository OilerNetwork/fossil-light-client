
// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "lib/U256Splitter.sol";

import {IStarknetCore} from "./interfaces/IStarknetCore.sol";

contract L1MessagesSender {
    IStarknetCore private _snCore;
    uint256 public immutable l2RecipientAddr;

    using Uint256Splitter for uint256;

    /// @dev starknetSelector(receive_from_l1)
    uint256 constant SUBMIT_L1_BLOCKHASH_SELECTOR =
        598342674068027518481179578557554850038206119856216505601406522348670006916;

    // TODO - describe
    constructor(address snCore, uint256 l2RecipientAddr_) {
        _snCore = IStarknetCore(snCore);
        l2RecipientAddr = l2RecipientAddr_;
    }

    // TODO - natspec
    function sendExactParentHashToL2(uint256 blockNumber_) external payable {
        bytes32 parentHash = blockhash(blockNumber_ - 1);
        require(parentHash != bytes32(0), "ERR_INVALID_BLOCK_NUMBER");
        _sendBlockHashToL2(parentHash, blockNumber_);
    }

    function sendLatestParentHashToL2() external payable {
        bytes32 parentHash = blockhash(block.number - 1);
        _sendBlockHashToL2(parentHash, block.number);
    }

    function _sendBlockHashToL2(bytes32 parentHash_, uint256 blockNumber_) internal {
        uint256[] memory message = new uint256[](4);
        (uint256 parentHashLow, uint256 parentHashHigh) = uint256(parentHash_).split128();
        (uint256 blockNumberLow, uint256 blockNumberHigh) = blockNumber_.split128();
        message[0] = parentHashLow;
        message[1] = parentHashHigh;
        message[2] = blockNumberLow;
        message[3] = blockNumberHigh;

        _snCore.sendMessageToL2{value: msg.value}(l2RecipientAddr, SUBMIT_L1_BLOCKHASH_SELECTOR, message);
    }
}
