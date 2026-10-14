# Reference Configuration Registry — v2 migration fixture

This is the append-only successor to [v1](../v1/README.md). It retains `owner` and `value` in
their v1 declaration order and appends `revision: U64`. The checked-in `migrate` declaration is
bound to the exact v1 schema digest, `sha256:e86a79dc3c132066b1e750df8f55b0777e2b0b3da58647050ac9ebd68d961dcc`.

Migration initializes only the appended `revision` field to zero; it does not change the v1
owner or configuration value. `set_value` still requires the canonical caller to equal `owner`,
then writes the new value, emits `ValueChanged`, and returns the value. `revision` remains the
zero-valued migration marker in this deliberately narrow pilot profile.

`migration-fixture.json` specifies the input and expected migrated state. It is local evidence,
not a live upgrade or deployment record. The matching integration test releases v1 and v2 in
temporary directories, verifies the v2 release from a separate copy, and checks the release and
receipt identity bindings through the mocked locked target.
