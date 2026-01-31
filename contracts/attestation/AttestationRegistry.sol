// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title AttestationRegistry (reference implementation)
/// @notice Minimal registry for ClearLang proof attestations.
/// @dev This contract is intentionally minimal and not production-hardened.
contract AttestationRegistry {
    struct Attestation {
        address signer;
        bytes32 payloadHash;
        string uri;
        uint256 timestamp;
    }

    mapping(bytes32 => Attestation) private attestations;

    event AttestationRegistered(
        bytes32 indexed attestationId,
        address indexed signer,
        bytes32 payloadHash,
        string uri,
        uint256 timestamp
    );

    function register(bytes32 attestationId, bytes32 payloadHash, string calldata uri) external {
        require(attestations[attestationId].timestamp == 0, "attestation exists");
        attestations[attestationId] = Attestation({
            signer: msg.sender,
            payloadHash: payloadHash,
            uri: uri,
            timestamp: block.timestamp
        });
        emit AttestationRegistered(attestationId, msg.sender, payloadHash, uri, block.timestamp);
    }

    function get(bytes32 attestationId) external view returns (Attestation memory) {
        return attestations[attestationId];
    }
}
