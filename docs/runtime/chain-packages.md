# Chain Packages

Chain packages provide chain-scoped types and helpers on top of the runtime ABI.

## Purpose
- Keep chain-specific rules out of the core language.
- Allow evolution as chains introduce new types or services.

## Examples
- `std::eth::Address`
- `std::solana::Pubkey`
- `std::eth::io::*` for chain I/O helpers built on runtime capabilities.

## Versioning and Evolution
- Chain packages should be versioned and opt-in.
- New crypto types or storage services belong in chain packages first.
- Core language changes should be rare and backward compatible.
