pub(super) use crate::commands::validation::{
    parse_utc_timestamp_components, validate_exact_semver, validate_package_id,
    validate_semver_requirement, validate_sha256_digest, validate_utc_rfc3339,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_id_validation_accepts_namespaced_identifiers() {
        assert!(validate_package_id("std::core").is_ok());
    }

    #[test]
    fn exact_semver_rejects_range() {
        assert!(validate_exact_semver("^1.0.0").is_err());
    }

    #[test]
    fn digest_validation_rejects_uppercase_hex() {
        assert!(validate_sha256_digest(
            "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        )
        .is_err());
    }

    #[test]
    fn utc_rfc3339_validation_rejects_invalid_day() {
        assert!(validate_utc_rfc3339("signed_at", "2026-02-31T00:00:00Z").is_err());
    }

    #[test]
    fn semver_requirement_validation_rejects_invalid_requirement() {
        assert!(validate_semver_requirement("latest").is_err());
    }
}
