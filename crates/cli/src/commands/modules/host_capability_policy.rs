use std::collections::BTreeSet;

pub(crate) const KNOWN_HOST_CAPABILITIES: [&str; 18] = [
    "std::crypto::hash",
    "std::crypto::hmac",
    "std::crypto::verify",
    "std::env::chain_id",
    "std::env::random",
    "std::env::time",
    "std::host::env::block_height",
    "std::host::env::caller",
    "std::host::env::chain_id",
    "std::host::env::timestamp",
    "std::host::log::error",
    "std::host::log::info",
    "std::host::log::warn",
    "std::host::storage::contains",
    "std::host::storage::delete",
    "std::host::storage::get",
    "std::host::storage::set",
    "std::wasi::print",
];

pub(crate) fn is_known_host_capability(capability: &str) -> bool {
    KNOWN_HOST_CAPABILITIES.binary_search(&capability).is_ok()
}

pub(crate) fn runtime_import_capability(module: &str, name: &str) -> Option<&'static str> {
    match (module, name) {
        ("clearlang_crypto", "crypto_hash") => Some("std::crypto::hash"),
        ("clearlang_crypto", "crypto_hmac") => Some("std::crypto::hmac"),
        ("clearlang_crypto", "crypto_verify") => Some("std::crypto::verify"),
        ("clearlang_env", "env_time") => Some("std::env::time"),
        ("clearlang_env", "env_random") => Some("std::env::random"),
        ("clearlang_env", "env_chain_id") => Some("std::env::chain_id"),
        ("clearlang_host", "host_storage_contains") => Some("std::host::storage::contains"),
        ("clearlang_host", "host_storage_get") => Some("std::host::storage::get"),
        ("clearlang_host", "host_storage_set") => Some("std::host::storage::set"),
        ("clearlang_host", "host_storage_delete") => Some("std::host::storage::delete"),
        ("clearlang_host", "host_log_info") => Some("std::host::log::info"),
        ("clearlang_host", "host_log_warn") => Some("std::host::log::warn"),
        ("clearlang_host", "host_log_error") => Some("std::host::log::error"),
        ("clearlang_host", "host_env_chain_id") => Some("std::host::env::chain_id"),
        ("clearlang_host", "host_env_caller") => Some("std::host::env::caller"),
        ("clearlang_host", "host_env_block_height") => Some("std::host::env::block_height"),
        ("clearlang_host", "host_env_timestamp") => Some("std::host::env::timestamp"),
        ("wasi_snapshot_preview1", "fd_write") => Some("std::wasi::print"),
        _ => None,
    }
}

pub(crate) fn collect_required_host_capabilities<'a, I>(imports: I) -> BTreeSet<String>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    let mut required = BTreeSet::new();
    for (module, name) in imports {
        if let Some(capability) = runtime_import_capability(module, name) {
            required.insert(capability.to_string());
        }
    }
    required
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_host_capabilities_are_sorted_and_unique() {
        for pair in KNOWN_HOST_CAPABILITIES.windows(2) {
            assert!(
                pair[0] < pair[1],
                "host capability list must stay strictly sorted and unique"
            );
        }
    }

    #[test]
    fn runtime_import_mapping_covers_locked_host_interfaces() {
        assert_eq!(
            runtime_import_capability("clearlang_crypto", "crypto_hash"),
            Some("std::crypto::hash")
        );
        assert_eq!(
            runtime_import_capability("clearlang_crypto", "crypto_hmac"),
            Some("std::crypto::hmac")
        );
        assert_eq!(
            runtime_import_capability("clearlang_crypto", "crypto_verify"),
            Some("std::crypto::verify")
        );
        assert_eq!(
            runtime_import_capability("clearlang_env", "env_time"),
            Some("std::env::time")
        );
        assert_eq!(
            runtime_import_capability("clearlang_env", "env_random"),
            Some("std::env::random")
        );
        assert_eq!(
            runtime_import_capability("clearlang_env", "env_chain_id"),
            Some("std::env::chain_id")
        );
        assert_eq!(
            runtime_import_capability("clearlang_host", "host_storage_contains"),
            Some("std::host::storage::contains")
        );
        assert_eq!(
            runtime_import_capability("clearlang_host", "host_storage_get"),
            Some("std::host::storage::get")
        );
        assert_eq!(
            runtime_import_capability("clearlang_host", "host_storage_set"),
            Some("std::host::storage::set")
        );
        assert_eq!(
            runtime_import_capability("clearlang_host", "host_storage_delete"),
            Some("std::host::storage::delete")
        );
        assert_eq!(
            runtime_import_capability("clearlang_host", "host_log_info"),
            Some("std::host::log::info")
        );
        assert_eq!(
            runtime_import_capability("clearlang_host", "host_log_warn"),
            Some("std::host::log::warn")
        );
        assert_eq!(
            runtime_import_capability("clearlang_host", "host_log_error"),
            Some("std::host::log::error")
        );
        assert_eq!(
            runtime_import_capability("clearlang_host", "host_env_chain_id"),
            Some("std::host::env::chain_id")
        );
        assert_eq!(
            runtime_import_capability("clearlang_host", "host_env_caller"),
            Some("std::host::env::caller")
        );
        assert_eq!(
            runtime_import_capability("clearlang_host", "host_env_block_height"),
            Some("std::host::env::block_height")
        );
        assert_eq!(
            runtime_import_capability("clearlang_host", "host_env_timestamp"),
            Some("std::host::env::timestamp")
        );
        assert_eq!(
            runtime_import_capability("wasi_snapshot_preview1", "fd_write"),
            Some("std::wasi::print")
        );
        assert_eq!(runtime_import_capability("unknown", "symbol"), None);
    }
}
