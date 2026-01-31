// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "../AttestationRegistry.sol";

contract AttestationRegistryTest {
    function test_register_and_get_roundtrip() public {
        AttestationRegistry registry = new AttestationRegistry();
        bytes32 attestationId = keccak256("attestation");
        bytes32 payloadHash = keccak256("payload");
        string memory uri = "ipfs://example";

        registry.register(attestationId, payloadHash, uri);

        AttestationRegistry.Attestation memory stored = registry.get(attestationId);
        assert(stored.signer == address(this));
        assert(stored.payloadHash == payloadHash);
        assert(keccak256(bytes(stored.uri)) == keccak256(bytes(uri)));
        assert(stored.timestamp != 0);
    }

    function test_register_rejects_duplicate() public {
        AttestationRegistry registry = new AttestationRegistry();
        bytes32 attestationId = keccak256("duplicate");
        registry.register(attestationId, keccak256("first"), "ipfs://first");

        bool reverted = false;
        try registry.register(attestationId, keccak256("second"), "ipfs://second") {
        } catch {
            reverted = true;
        }
        assert(reverted);
    }
}
