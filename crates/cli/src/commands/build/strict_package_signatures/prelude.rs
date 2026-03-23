use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

use super::strict_package_contract::StrictPackageContractV0;
use super::strict_trust_policy::{StrictTrustPolicyV0, TrustedSignerV0};
use super::strict_validation::{
    parse_utc_timestamp_components, validate_exact_semver, validate_package_id,
    validate_sha256_digest,
};

const STRICT_PACKAGE_SIGNATURES_FILE: &str = "clg.package-signatures.json";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StrictPackageSignaturesError {
    code: &'static str,
    message: String,
}

impl StrictPackageSignaturesError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(super) fn code(&self) -> &'static str {
        self.code
    }

    pub(super) fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrictPackageSignatureEntry {
    name: String,
    version: String,
    digest: String,
    key_id: String,
    signed_at: String,
    signature: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSignatureRoot {
    schema_version: u32,
    signatures: Vec<RawSignatureEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSignatureEntry {
    name: String,
    version: String,
    digest: String,
    key_id: String,
    signed_at: String,
    signature_format: String,
    signature: String,
}

