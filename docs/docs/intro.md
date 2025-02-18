---
id: introduction
title: Introduction
---

# Introduction

This document serves as an initial draft of the architecture for the Fossil Light Client (FLC). Its primary purpose is to provide stakeholders with an overview of the current implementation, gather feedback, and establish the next steps for development and potential design adjustments.
The FLC is designed to offer trustless guarantees of Ethereum block data integrity, a foundation for Fossil’s computational processes. It achieves this by leveraging advanced cryptographic techniques, direct access to Ethereum block hashes via Solidity opcodes, and L1-to-L2 messaging. All Ethereum block hashes are verified for integrity and stored in on-chain Merkle Mountain Range (MMR) states on Starknet to ensure that when a request for Ethereum data is made to Fossil, each block hash in the requested range can be validated as part of the on-chain MMR states.
The intended audience for this document includes stakeholders, product managers, and developers at Nethermind.
