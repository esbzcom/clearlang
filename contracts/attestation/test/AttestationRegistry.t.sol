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

contract AttestationRegistryTest {
    function test_register_and_get_roundtrip() public {
        AttestationRegistry registry = new AttestationRegistry();
        bytes32 payloadHash = keccak256("payload");
        string memory uri = "ipfs://example";
        uint32 schemaVersion = 1;
        bytes32 attestationId = registry.computeAttestationId(payloadHash, address(this), schemaVersion);

        registry.register(attestationId, payloadHash, uri, schemaVersion);

        AttestationRegistry.Attestation memory stored = registry.get(attestationId);
        assert(stored.signer == address(this));
        assert(stored.payloadHash == payloadHash);
        assert(keccak256(bytes(stored.uri)) == keccak256(bytes(uri)));
        assert(stored.schemaVersion == schemaVersion);
        assert(stored.timestamp != 0);
        assert(!stored.revoked);
        assert(stored.revokedAt == 0);
    }

    function test_register_rejects_duplicate() public {
        AttestationRegistry registry = new AttestationRegistry();
        bytes32 payloadHash = keccak256("first");
        bytes32 attestationId = registry.computeAttestationId(payloadHash, address(this), 1);
        registry.register(attestationId, payloadHash, "ipfs://first", 1);

        bool reverted = false;
        try registry.register(attestationId, keccak256("second"), "ipfs://second", 1) {
        } catch {
            reverted = true;
        }
        assert(reverted);
    }

    function test_register_rejects_unauthorized_signer() public {
        AttestationRegistry registry = new AttestationRegistry();
        RegistryCaller unauthorized = new RegistryCaller();
        bytes32 payloadHash = keccak256("payload");
        bytes32 attestationId = registry.computeAttestationId(payloadHash, address(unauthorized), 1);

        bool reverted = false;
        try unauthorized.register(registry, attestationId, payloadHash, "ipfs://unauthorized", 1) {
        } catch {
            reverted = true;
        }
        assert(reverted);
    }

    function test_owner_can_authorize_and_rotate_signer() public {
        AttestationRegistry registry = new AttestationRegistry();
        RegistryCaller signer = new RegistryCaller();
        registry.setSignerAuthorization(address(signer), true);

        bytes32 payloadHash1 = keccak256("payload1");
        bytes32 id1 = registry.computeAttestationId(payloadHash1, address(signer), 1);
        signer.register(registry, id1, payloadHash1, "ipfs://one", 1);

        registry.setSignerAuthorization(address(signer), false);

        bool reverted = false;
        bytes32 payloadHash2 = keccak256("payload2");
        bytes32 id2 = registry.computeAttestationId(payloadHash2, address(signer), 1);
        try signer.register(registry, id2, payloadHash2, "ipfs://two", 1) {
        } catch {
            reverted = true;
        }
        assert(reverted);
    }

    function test_register_rejects_unsupported_schema_until_enabled() public {
        AttestationRegistry registry = new AttestationRegistry();
        uint32 schemaVersion = 2;
        bytes32 payloadHash = keccak256("payload");
        bytes32 attestationId = registry.computeAttestationId(payloadHash, address(this), schemaVersion);

        bool reverted = false;
        try registry.register(attestationId, payloadHash, "ipfs://schema2", schemaVersion) {
        } catch {
            reverted = true;
        }
        assert(reverted);

        registry.setSchemaVersion(schemaVersion, true);
        registry.register(attestationId, payloadHash, "ipfs://schema2", schemaVersion);
    }

    function test_register_rejects_attestation_id_mismatch() public {
        AttestationRegistry registry = new AttestationRegistry();
        bytes32 payloadHash = keccak256("payload");
        bytes32 wrongId = keccak256("wrong");

        bool reverted = false;
        try registry.register(wrongId, payloadHash, "ipfs://bad-id", 1) {
        } catch {
            reverted = true;
        }
        assert(reverted);
    }

    function test_signer_can_revoke() public {
        AttestationRegistry registry = new AttestationRegistry();
        RegistryCaller signer = new RegistryCaller();
        registry.setSignerAuthorization(address(signer), true);

        bytes32 payloadHash = keccak256("payload");
        bytes32 attestationId = registry.computeAttestationId(payloadHash, address(signer), 1);
        signer.register(registry, attestationId, payloadHash, "ipfs://revokable", 1);
        signer.revoke(registry, attestationId);

        AttestationRegistry.Attestation memory stored = registry.get(attestationId);
        assert(stored.revoked);
        assert(stored.revokedAt != 0);
    }

    function test_owner_can_revoke_foreign_attestation() public {
        AttestationRegistry registry = new AttestationRegistry();
        RegistryCaller signer = new RegistryCaller();
        registry.setSignerAuthorization(address(signer), true);

        bytes32 payloadHash = keccak256("payload");
        bytes32 attestationId = registry.computeAttestationId(payloadHash, address(signer), 1);
        signer.register(registry, attestationId, payloadHash, "ipfs://owner-revokes", 1);
        registry.revoke(attestationId);

        AttestationRegistry.Attestation memory stored = registry.get(attestationId);
        assert(stored.revoked);
    }

    function test_revoke_rejects_non_owner_non_signer() public {
        AttestationRegistry registry = new AttestationRegistry();
        RegistryCaller signer = new RegistryCaller();
        RegistryCaller stranger = new RegistryCaller();
        registry.setSignerAuthorization(address(signer), true);

        bytes32 payloadHash = keccak256("payload");
        bytes32 attestationId = registry.computeAttestationId(payloadHash, address(signer), 1);
        signer.register(registry, attestationId, payloadHash, "ipfs://target", 1);

        bool reverted = false;
        try stranger.revoke(registry, attestationId) {
        } catch {
            reverted = true;
        }
        assert(reverted);
    }
}
