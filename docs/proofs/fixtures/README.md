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

## linear-collections-branch.vc.json
Source:
```
resource File { drop {} }
pure function choose(flag: Bool, consume files: List<File>) -> List<File> {
  if flag {
    std::list::remove_take(files, 0)[0]
  } else {
    std::list::remove_take(files, 0)[0]
  }
}
```

## linear-collections-branch-inline.vc.json
Source:
```
resource File { drop {} }
pure function id(consume files: List<File>) -> List<File> { files }
pure function choose(flag: Bool, consume files: List<File>) -> List<File> {
  if flag {
    std::list::remove_take(id(files), 0)[0]
  } else {
    std::list::remove_take(id(files), 0)[0]
  }
}
```

## linear-collections-loop.vc.json
Source:
```
resource File { drop {} }
pure function loop_step(consume files: List<File>, n: Int) -> List<File> {
  while n > 0 invariant { n >= 0 } variant { n } {
    let out = std::list::remove_take(files, 0);
    let files = out[0];
  }
  files
}
```
