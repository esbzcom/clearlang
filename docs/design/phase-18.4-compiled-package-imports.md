# Phase 18.4: Compiled Package/Module Imports

## Goal
Allow `import pkg::module` and `import pkg::module::{item}` to resolve from compiled package metadata when no local source module exists.

## Metadata File
- Root file (legacy compatibility path): `clg-packages.json`
- Supported schema: `schema_version = 1`
- Migration note: canonical production metadata is tracked under Phase 22 (`clg.package-metadata.json` + `clg.package-abi.json`), with deterministic coexistence rejection during transition.
- Package constraints:
  - package `name` must be an identifier
  - `version` must follow `MAJOR.MINOR.PATCH`
  - `artifact.format` must be `wasm`
  - `artifact.path` must exist and point to a file
- Module constraints:
  - module path must be valid and start with the package name
  - `std::...` module paths are forbidden
- Export constraints:
  - export names must be unique per module
  - `type` exports require non-zero `{ bytes, align }` layout
  - `value` exports require `params`, `ret`, and an import target `{ module, name }`

## Resolver Behavior
- Import resolution order for non-`std` modules:
  - local source module file
  - compiled package module from metadata
- Conflict rule:
  - if both local source and package metadata provide the same module path, compilation fails (`C028`)
- Unknown module still fails with `C020`.

## Type/Loweing/Codegen Integration
- Package `type` exports are merged into the known external type-layout map.
- Package `value` exports are injected into typer as external callable signatures.
- Only *used* external functions are emitted as wasm imports.
- Lowered IR includes external function declarations so calls resolve deterministically.
- Codegen emits wrapper functions that forward arguments to imported host functions.

## Diagnostics
- `C027`: invalid compiled-package metadata (legacy schema/validation/collision issues, including migration coexistence conflicts)
- `C028`: module path conflict between local source and compiled package metadata

## Non-goals (Phase 18.4)
- No transitive package dependency resolution.
- No semver solver.
- No runtime package loader.
