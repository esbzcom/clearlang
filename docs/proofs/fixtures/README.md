# VC Refinement Fixtures

These fixtures mirror `--emit-vcs` output for refined aliases.

## Runnability notes
`clg build --emit-vcs` requires a `main` entrypoint. The snippets below omit `main` for clarity.
To regenerate a fixture, add a minimal entrypoint and emit VCs, for example:
```
function main() -> Int { 0 }
```
```
clg build sample.clear --emit-vcs docs/proofs/fixtures/refinement-basic.vc.json -o out.wasm
```

## refinement-basic.vc.json
Source:
```
type Nat = Int where n >= 0;
pure function inc(a: Nat) -> Nat { a + 1 }
```

## refinement-contracts-loops.vc.json
Source:
```
type Nat = Int where n >= 0;
pure function countdown(n: Nat) -> Int
  require { n > 1 }
  ensure { result >= 0 }
{
  while n > 0 invariant { n >= 0 } variant { n } { n; }
  n + 0
}
```

## refinement-call-site.vc.json
Source:
```
type Nat = Int where n >= 0;
pure function takes(n: Nat) -> Nat { n }
pure function caller(x: Int) -> Nat
  require { x >= 0 }
  ensure { result >= 0 }
{ takes(x + 1) }
```
