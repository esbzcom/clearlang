# Chain Packages

Chain packages provide chain-scoped types and helpers on top of the runtime ABI.

## Purpose
- Keep chain-specific rules out of the core language.
- Allow evolution as chains introduce new types or services.

## Chain-Scoped Environment Types
- Chain identity types live in chain packages, not the core language.
- Examples: `std::eth::Address`, `std::solana::Pubkey`, `std::cosmos::Addr`.
- Avoid a global `Address` type; chain packages define encoding, validation, and helpers.

## Examples
- `std::eth::Address`
- `std::solana::Pubkey`
- `std::eth::io::*` for chain I/O helpers built on runtime capabilities.

## Business Logic Placement
- Business-logic utilities (e.g., token helpers, chain-specific encoding) belong in chain packages.
- Core language features remain chain-agnostic and minimal.

## Effect Gating
- Chain packages may wrap host capabilities only through `io` functions.
- Pure contract logic remains deterministic; host interaction is explicit.

## Versioning and Evolution
- Chain packages should be versioned and opt-in.
- New crypto types or storage services belong in chain packages first.
- Core language changes should be rare and backward compatible.
