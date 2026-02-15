// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title AttestationRegistry (production-hardening baseline)
/// @notice Registry for ClearLang proof attestations with signer authz,
///         key rotation controls, revocation, and schema versioning.
contract AttestationRegistry {
    uint256 public constant MAX_URI_BYTES = 512;

    struct Attestation {
        address signer;
        bytes32 payloadHash;
        string uri;
        uint32 schemaVersion;
        uint64 timestamp;
        bool revoked;
        uint64 revokedAt;
    }

    address public owner;
    mapping(address => bool) public authorizedSigners;
    mapping(uint32 => bool) public allowedSchemaVersions;
    mapping(bytes32 => Attestation) private attestations;

    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event SignerAuthorizationChanged(address indexed signer, bool authorized);
    event SchemaVersionStatusChanged(uint32 indexed schemaVersion, bool allowed);
    event AttestationRegistered(
        bytes32 indexed attestationId,
        address indexed signer,
        bytes32 payloadHash,
        string uri,
        uint32 schemaVersion,
        uint256 timestamp
    );
    event AttestationRevoked(bytes32 indexed attestationId, address indexed revokedBy, uint256 timestamp);

    modifier onlyOwner() {
        require(msg.sender == owner, "not owner");
        _;
    }

    constructor() {
        owner = msg.sender;
        emit OwnershipTransferred(address(0), msg.sender);

        authorizedSigners[msg.sender] = true;
        emit SignerAuthorizationChanged(msg.sender, true);

        allowedSchemaVersions[1] = true;
        emit SchemaVersionStatusChanged(1, true);
    }

    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "owner zero");
        address previousOwner = owner;
        owner = newOwner;
        if (!authorizedSigners[newOwner]) {
            authorizedSigners[newOwner] = true;
            emit SignerAuthorizationChanged(newOwner, true);
        }
        emit OwnershipTransferred(previousOwner, newOwner);
    }

    function setSignerAuthorization(address signer, bool authorized) external onlyOwner {
        require(signer != address(0), "signer zero");
        authorizedSigners[signer] = authorized;
        emit SignerAuthorizationChanged(signer, authorized);
    }

    function setSchemaVersion(uint32 schemaVersion, bool allowed) external onlyOwner {
        require(schemaVersion != 0, "schema zero");
        allowedSchemaVersions[schemaVersion] = allowed;
        emit SchemaVersionStatusChanged(schemaVersion, allowed);
    }

    /// @notice Computes the canonical attestation id.
    /// @dev Binds id to this registry, signer, payload hash, and schema version.
    function computeAttestationId(bytes32 payloadHash, address signer, uint32 schemaVersion)
        public
        view
        returns (bytes32)
    {
        return keccak256(abi.encode(address(this), payloadHash, signer, schemaVersion));
    }

    /// @notice Stores a new attestation under an authorized signer and supported schema.
    function register(bytes32 attestationId, bytes32 payloadHash, string calldata uri, uint32 schemaVersion)
        external
    {
        require(authorizedSigners[msg.sender], "unauthorized signer");
        require(allowedSchemaVersions[schemaVersion], "unsupported schema");
        require(payloadHash != bytes32(0), "payload hash zero");
        uint256 uriLen = bytes(uri).length;
        require(uriLen > 0, "uri empty");
        require(uriLen <= MAX_URI_BYTES, "uri too long");
        require(attestations[attestationId].timestamp == 0, "attestation exists");
        require(
            attestationId == computeAttestationId(payloadHash, msg.sender, schemaVersion),
            "attestation id mismatch"
        );

        attestations[attestationId] = Attestation({
            signer: msg.sender,
            payloadHash: payloadHash,
            uri: uri,
            schemaVersion: schemaVersion,
            timestamp: uint64(block.timestamp),
            revoked: false,
            revokedAt: 0
        });

        emit AttestationRegistered(
            attestationId,
            msg.sender,
            payloadHash,
            uri,
            schemaVersion,
            block.timestamp
        );
    }

    /// @notice Revokes an existing attestation.
    /// @dev Allowed for the original signer or the contract owner.
    function revoke(bytes32 attestationId) external {
        Attestation storage attestation = attestations[attestationId];
        require(attestation.timestamp != 0, "attestation missing");
        require(!attestation.revoked, "attestation revoked");
        require(msg.sender == owner || msg.sender == attestation.signer, "not revoker");

        attestation.revoked = true;
        attestation.revokedAt = uint64(block.timestamp);
        emit AttestationRevoked(attestationId, msg.sender, block.timestamp);
    }

    function get(bytes32 attestationId) external view returns (Attestation memory) {
        return attestations[attestationId];
    }
}
