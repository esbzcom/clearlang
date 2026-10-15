# Phase 43.0.3: Verifier lifecycle and emergency rollback lock

Date: 2026-10-15  
Status: Locked  
Owner: release-assurance-owner

## Scope

This lock defines how the independently distributed `clg-contract-verifier` is deprecated,
withdrawn, and replaced. It preserves the Phase 43.0.2 requirement that verification is wholly
offline and explicit. It does not add remote revocation, automatic updates, compatibility
translation, source builds, OCI images, or a new trust-policy format.

## Compatibility is declared, never inferred

The signed distribution manifest is the sole compatibility declaration for a verifier artifact.
A verifier may accept an input only when its own explicit capability set includes every exact
bundle format, schema version, target profile, execution profile, result schema, and minimum
verifier capability declared by that input. SemVer proximity, a similar name, an extension, or a
field that happens to be ignorable is not a compatibility rule.

When a requirement is absent or unsupported, the verifier must return its stable rejected result
(`status: "rejected"`) and must not parse as an older schema, select a nearest profile, discard an
unknown versioned field, or continue with weakened checks. Packaging validation likewise rejects
a missing or mismatched archive checksum, manifest, signature, distribution key, platform label,
or executable name. The final stable diagnostic code assignments are delivered with the shared
entrypoint in 43.1; their category must distinguish a package-integrity rejection from a
bundle-capability rejection without exposing local paths or secret material.

An unsupported host platform is a distribution rejection, not a reason to choose a similar
archive. The approved response is to use one of the exact supported artifacts in the Phase 43.0.2
table. This pilot does not authorize source compilation, an emulator, an OCI image, a hosted
service, or a wrapper as a replacement verifier.

## Deprecation policy

The initial supported line is the explicitly declared `0.1.x` capability set. A later release
must publish a new immutable archive, checksum listing, signed manifest, and release note; it
must never replace the bytes, checksum, or manifest of an existing version.

A deprecation notice must name all of the following:

1. The exact affected verifier version(s), platform artifact names, and manifest digest.
2. The replacement version and the exact capabilities it adds, retains, or removes.
3. Whether existing evidence may still be replayed with the deprecated artifact, and the exact
   last version that supports each declared format.
4. Whether the notice is routine deprecation or an emergency withdrawal, which follows the
   stricter rules below.

Deprecation is communication and release-selection policy; it does not rely on a clock or a
network response inside the verifier. A historical, valid artifact stays useful for reproducing
the evidence formats that its signed manifest declares, unless an emergency advisory marks that
exact artifact unsafe for operational use. A new verifier must still reject formats that it does
not explicitly declare, even if an earlier verifier accepted them.

## Emergency withdrawal and rollback

For a suspected verifier compromise, signing-key compromise, manifest/signature mismatch,
incorrect compatibility claim, or package-integrity failure, release owners must:

1. Freeze promotion and mark the exact affected version, artifact filename, checksum, manifest
   digest, and distribution key ID as suspect. Preserve the original artifact and all observed
   evidence; do not overwrite or silently delete it.
2. Publish a signed advisory through the trusted release channel that identifies affected
   artifacts, the reason and impact, a fixed version or explicitly named last-known-good version,
   and any key-rotation requirement. The advisory is an operator input; the offline verifier does
   not fetch or discover it.
3. If a key is suspected, retire it from future distribution signing and issue a separately
   authenticated replacement public key through the approved out-of-band channel. Never treat a
   release-bundle key as a distribution key or silently trust a newly seen key.
4. Build a new, unique fixed version and immutable distribution set. Validate its checksum,
   manifest signature, capability declaration, and a known-good release bundle locally before it
   is promoted. Never re-sign changed bytes under the withdrawn version.
5. Instruct operators to stop using the withdrawn artifact for new operational decisions. They
   may preserve and report prior results as historical evidence, clearly labelled with the
   affected artifact digest; remediation is a rerun with the named fixed or last-known-good
   verifier where the advisory permits it.

Rollback means selecting a specifically identified previously published verifier, not restoring
an unversioned "latest" file and not making a newer verifier emulate an old schema. Before using
a rollback candidate, the operator must locally verify its archive checksum and signed manifest,
confirm that its explicit capabilities cover the bundle being checked, and record the advisory
and verifier version with the result. If no non-affected artifact declares the needed
capabilities, the correct outcome is rejection and escalation, not a compatibility workaround.

## Implementation acceptance

Phase 43.1 must make the documented reject behavior observable from a clean directory. Local
black-box tests must cover an unknown schema/profile/capability, unsupported platform artifact,
checksum or manifest/signature mismatch, wrong distribution key, and a verifier-binary/version
mismatch. They must assert a stable rejected result, no nearest-version fallback, no ambient
path use, and no network attempt. A simulated withdrawal advisory must show that the fixed or
last-known-good named artifact is selected explicitly rather than by automatic downgrade.

## References

- `docs/design/phase-43.0.2-verifier-compatibility-lock.md`
- `docs/design/phase-43.0.1-verifier-delivery-form-lock.md`
- `docs/release/milestone_3-binary-incident-runbook.md`
- `docs/contract-operations.md`
