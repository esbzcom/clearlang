# Runtime ABI Overview

This document defines the stable host interface that ClearLang programs target when compiled to Wasm.

## Goals
- Deterministic and portable across chains and non-chain runtimes.
- Minimal surface area that can be re-implemented by different hosts.
- Clear separation from chain-specific libraries and business logic.

## Host Capabilities (Baseline)
- Storage: read/write raw bytes by key.
- Crypto syscalls: hash and signature verification hooks (host-provided).
- Logging/events: append structured logs for the host to consume.
- Metering: gas/step accounting and deterministic limits.
- ABI entrypoints: `init`, `handle`, and `query` with canonical serialization.

## Notes
- Chain packages should wrap these primitives with chain-specific types and rules.
- The runtime must document limits and determinism guarantees.
