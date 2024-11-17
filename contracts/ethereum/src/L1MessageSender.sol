
// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import {Uint256Splitter} from "lib/U256Splitter.sol";

import {IStarknetMessaging} from "../src/StarknetMessagingLocal.sol";

contract L1MessageSender {
    IStarknetMessaging private _snMessaging;
    // uint256 public immutable l2RecipientAddr;

    using Uint256Splitter for uint256;

    /// @dev starknetSelector(receive_from_l1)
    uint256 constant RECEIVE_FROM_L1_SELECTOR =
        598342674068027518481179578557554850038206119856216505601406522348670006916;

    // TODO - describe
    constructor(address snMessaging) {
        _snMessaging = IStarknetMessaging(snMessaging);
    }

    function sendFinalizedBlockHashToL2(uint256 l2RecipientAddr) external payable {
        uint256 finalizedBlockNumber = block.number - 192;
        bytes32 parentHash = blockhash(finalizedBlockNumber);
        uint256 blockNumber = uint256(finalizedBlockNumber);
        _sendBlockHashToL2(parentHash, blockNumber, l2RecipientAddr);
    }

    function _sendBlockHashToL2(bytes32 parentHash_, uint256 blockNumber_, uint256 _l2RecipientAddr) internal {
        uint256[] memory message = new uint256[](4);
        (uint256 parentHashLow, uint256 parentHashHigh) = uint256(parentHash_).split128();
        (uint256 blockNumberLow, uint256 blockNumberHigh) = blockNumber_.split128();
        message[0] = parentHashLow;
        message[1] = parentHashHigh;
        message[2] = blockNumberLow;
        message[3] = blockNumberHigh;

        _snMessaging.sendMessageToL2{value: 30000}(_l2RecipientAddr, RECEIVE_FROM_L1_SELECTOR, message);
    }
}
