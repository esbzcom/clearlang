// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "../AttestationRegistry.sol";

contract RegistryCaller {
    function register(
        AttestationRegistry registry,
        bytes32 attestationId,
        bytes32 payloadHash,
        string calldata uri,
        uint32 schemaVersion
    ) external {
        registry.register(attestationId, payloadHash, uri, schemaVersion);
    }

    function revoke(AttestationRegistry registry, bytes32 attestationId) external {
        registry.revoke(attestationId);
    }
}
