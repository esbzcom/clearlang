// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "../AttestationRegistry.sol";
import "./RegistryCaller.sol";

contract AttestationRegistryFuzzTest {
    function testFuzz_compute_attestation_id_is_deterministic(
        bytes32 payloadHash,
        address signer,
        uint32 schemaVersion
    ) public {
        AttestationRegistry registry = new AttestationRegistry();
        bytes32 id1 = registry.computeAttestationId(payloadHash, signer, schemaVersion);
        bytes32 id2 = registry.computeAttestationId(payloadHash, signer, schemaVersion);
        assert(id1 == id2);
    }

    function testFuzz_register_rejects_unauthorized_signer(bytes32 payloadHash, uint32 schemaVersion) public {
        AttestationRegistry registry = new AttestationRegistry();
        RegistryCaller unauthorized = new RegistryCaller();
        uint32 effectiveSchema = schemaVersion == 0 ? 1 : schemaVersion;
        if (effectiveSchema != 1) {
            registry.setSchemaVersion(effectiveSchema, true);
        }
        bytes32 attestationId =
            registry.computeAttestationId(payloadHash, address(unauthorized), effectiveSchema);

        bool reverted = false;
        try unauthorized.register(registry, attestationId, payloadHash, "ipfs://fuzz", effectiveSchema) {
        } catch {
            reverted = true;
        }
        assert(reverted);
    }

    function testFuzz_register_rejects_mismatched_id(
        bytes32 payloadHash,
        bytes32 otherPayloadHash,
        uint32 schemaVersion
    ) public {
        AttestationRegistry registry = new AttestationRegistry();
        uint32 effectiveSchema = schemaVersion == 0 ? 1 : schemaVersion;
        if (effectiveSchema != 1) {
            registry.setSchemaVersion(effectiveSchema, true);
        }

        bytes32 wrongId = registry.computeAttestationId(otherPayloadHash, address(this), effectiveSchema);
        bytes32 expectedId = registry.computeAttestationId(payloadHash, address(this), effectiveSchema);
        if (wrongId == expectedId) {
            wrongId = bytes32(uint256(wrongId) ^ 1);
        }

        bool reverted = false;
        try registry.register(wrongId, payloadHash, "ipfs://fuzz-bad-id", effectiveSchema) {
        } catch {
            reverted = true;
        }
        assert(reverted);
    }

    function testFuzz_schema_toggle_controls_registration(bytes32 payloadHash, uint32 schemaVersion) public {
        AttestationRegistry registry = new AttestationRegistry();
        if (payloadHash == bytes32(0)) {
            payloadHash = keccak256("fuzz-nonzero");
        }
        uint32 effectiveSchema = schemaVersion == 0 ? 2 : schemaVersion;
        if (effectiveSchema == 1) {
            effectiveSchema = 2;
        }

        bytes32 attestationId = registry.computeAttestationId(payloadHash, address(this), effectiveSchema);

        bool reverted = false;
        try registry.register(attestationId, payloadHash, "ipfs://schema-gated", effectiveSchema) {
        } catch {
            reverted = true;
        }
        assert(reverted);

        registry.setSchemaVersion(effectiveSchema, true);
        registry.register(attestationId, payloadHash, "ipfs://schema-gated", effectiveSchema);
    }

    function testFuzz_revoke_rejects_non_owner_non_signer(bytes32 payloadHash, uint32 schemaVersion) public {
        AttestationRegistry registry = new AttestationRegistry();
        RegistryCaller signer = new RegistryCaller();
        RegistryCaller stranger = new RegistryCaller();
        if (payloadHash == bytes32(0)) {
            payloadHash = keccak256("fuzz-nonzero");
        }

        uint32 effectiveSchema = schemaVersion == 0 ? 1 : schemaVersion;
        if (effectiveSchema != 1) {
            registry.setSchemaVersion(effectiveSchema, true);
        }
        registry.setSignerAuthorization(address(signer), true);

        bytes32 attestationId = registry.computeAttestationId(payloadHash, address(signer), effectiveSchema);
        signer.register(registry, attestationId, payloadHash, "ipfs://rev-target", effectiveSchema);

        bool reverted = false;
        try stranger.revoke(registry, attestationId) {
        } catch {
            reverted = true;
        }
        assert(reverted);
    }
}
