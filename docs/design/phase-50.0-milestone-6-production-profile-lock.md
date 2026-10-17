# Phase 50.0 - Milestone 6 Production Profile and Threat-Model Lock

Date: 2026-10-17

Status: Locked for local planning only; it authorizes no target action.

Owner: product-and-runtime-owner

## Purpose

Lock the exact product claim, inherited technical boundary, actors, threat
model, failure rules, and evidence map for Milestone 6 before ClearLang selects
a mainnet, signer/governance policy, or target behavior. This is a promotion of
one existing configuration-registry profile, not a language or EVM expansion.

The lock allows only local documentation, fixture, policy, and tooling work.
It does not authorize a testnet or mainnet transaction. Phase 50.1 must first
reproduce the current full Milestone 5 v2 local acceptance report from a clean
checkout; Phase 50.2 then selects exactly one eligible mainnet.

## Product Decision

The only candidate production product is the existing **fixed-owner,
caller-authorized scalar Configuration Registry Profile**. Its legitimate
low-value use is a public configuration-revision beacon: the owner announces
which externally published, non-custodial configuration revision is current.

The profile stores only a fixed owner and one `U64` value. The owner may replace
the value through the one permitted transition, which emits a canonical event.
The profile does not store or validate an off-chain configuration document,
content digest, URI, or availability claim; it does not force any downstream
client to follow the announced revision.

The sole possible completion claim is:

> Production-ready for the named Configuration Registry Profile on the named
> chain, under the named operator policy and exposure limit only.

That statement is unavailable until every Milestone 6 gate closes. It must
never be shortened to “production-ready”, “mainnet-ready”, or “verified”
without its profile, chain, and evidence-layer qualifiers.

## Product Boundary and Exposure Limit

The production candidate retains all of these constraints:

| Area | Permitted boundary | Explicitly excluded |
| --- | --- | --- |
| State | One immutable owner address and one `U64` configuration revision. | Maps, lists, dynamic values, arbitrary state, content hashes, or configuration documents. |
| Transition | The owner alone may call `set_value(next)`; it writes one value and emits `ValueChanged(next)`. | Owner rotation, multiple owners, roles, delegation, multisig, recovery, pause, timelock, or upgrades. |
| Value and assets | Every target action has native transaction value `0`; the contract cannot receive, hold, account for, or transfer assets. | Tokens, custody, balances, exchanges, lending, settlement, or any financial decision. |
| Target behavior | Direct construction, reads, and the one straight-line guarded transition under an approved target profile. | Generic EVM, proxies, delegate calls, callbacks, external calls, or arbitrary ABI behavior. |
| Operator behavior | Explicit reviewed endpoint, chain ID, sender, signer policy, nonce, gas limit, fee fields, and constructor/invocation inputs. | Wallet discovery, remote signing, implicit network/account/nonce/fee selection, or transaction replacement outside policy. |

The maximum permitted on-chain asset exposure is therefore zero. A downstream
service's decision to read the announced revision is outside this profile's
proof and operational claim. If the announcement would control funds, custody,
critical safety operations, or a broad user population, it is outside the
low-value product boundary and requires a new product lock.

## Inherited Contract Inventory

The following inventory is authoritative for the Milestone 6 candidate. A
future task may update it only through an explicit compatibility decision; it
must not silently reinterpret an inherited artifact or substitute a newer
surface.

| Area | Locked inherited input | Authority |
| --- | --- | --- |
| Source profile | `clg.evm-stateful-scalar.v1` with the restricted caller authorization defined in Phase 41.0. | `docs/design/phase-41.0-caller-authorization-profile-lock.md` |
| Reference source | v1 `ConfigRegistry`: `owner: std::eth::Address`, `value: U64`, `init(owner, initial_value)`, and owner-guarded `set_value(next)`. | `examples/projects/reference-config-registry/v1/config_registry.clear` and its README |
| Proof and release model | Source/proof/schema/ABI/EVM/campaign/signature/assurance/receipt/release-bundle identities remain cross-bound and independently verifiable. | `docs/design/phase-30.6.1-contract-release-verification-lock.md` and `docs/contract-product-contract.md` |
| Bundle verifier | `clg.contract-release-bundle.v1`, schema `1`; result `clg.contract-release-verifier-result.v1`, schema `1`; independent verifier line `0.1.x`. | `docs/release/milestone_5_pilot_closure_decision.md` |
| Local acceptance | `clg.pilot-precheck-report.v2`, schema `2`, `scope: "full"`; release precheck and offline verifier workflow remain required inputs. | `docs/design/phase-40.1.1-pilot-precheck-cli-report-lock.md` and Milestone 5 closure decision |
| Diagnostic boundary | Bundle/evidence verification failure is `C140`; simulator execution failure is trace-backed `C142`; unsupported caller-profile behavior is `T834`. | `docs/contract-product-contract.md` and Phase 41.0 lock |
| Pilot target | `clg.evm-sepolia-pilot.v1` on chain ID `11155111` is historical pilot evidence only. | `docs/design/phase-42.0.2-sepolia-pilot-profile-lock.md` |

The eventual production target is deliberately **not** in this table. Phase
50.2 must select exactly one named mainnet and define its target policy. The
Sepolia profile is neither a production default nor an authorization to infer a
mainnet endpoint, client, finality rule, transaction type, or signer policy.

### Baseline Identity Rule

Milestone 5 retained the following historical identity values:

| Identity | Historical accepted value |
| --- | --- |
| Clean-clone evidence revisions | `118fd2c3922b0ac71272c3559c23f59c0df6254d`, `928e9ab3822562493abe9291ee00fdad92c0e166` |
| Rust/Cargo | `1.89.0` |
| Solver | Checked-in Z3 `4.16.0`, external CLI backend; solver profile `25.1.4`; supply-chain policy `25.1.16` |
| Dependency lock SHA-256 | `6a50f62920ff103f68e25f08923e932686ce188c5e8ed63d58e80a00315c9bf2` |
| Historical v1 full-report SHA-256 | `fed40ee8b5054747c6416815a8324f44c585cba18096aef7635a978312d18d87` |

These values are inventory inputs, not a replacement for current acceptance.
Historical v1 reports cannot be relabeled as v2 evidence. The current working
tree is also not a qualifying baseline because it contains uncommitted planning
changes. Phase 50.1 has recorded a clean committed revision, tool identities,
lockfile digest, solver/trust inputs, and a freshly generated full v2 report in
`docs/evidence/phase-50.1-clean-v2-reproduction.md`; only that record is the
Milestone 6 baseline. A subsequent input change requires a new approved
compatibility decision and reproduction.

## Claim Layers

The following vocabulary is mandatory in every Milestone 6 document and
artifact. No layer inherits the guarantee of another layer.

| Layer | Permitted statement | Required evidence | Not established |
| --- | --- | --- | --- |
| Source/proof | The exact supported source obligations were checked under the named compiler, solver, and proof profile. | Canonical proof artifact and bound source/compiler/profile identities. | Correctness of unmodeled semantics, cryptographic hardness, host behavior, or chain behavior. |
| Release verification | The independent verifier accepted the bundle-local evidence under the approved key and policy. | Verifier package/trust inputs and canonical verifier result. | Safety of altered sources/artifacts or a live deployment. |
| Local simulation/replay | The locked simulator or replay reproduced the recorded input and result. | Canonical trace, request/response fixture, and replay result. | RPC honesty, chain consensus, finality, or a live action. |
| Target observation | The named observation client returned records matching the locked request and identity chain. | Sanitized evidence record, receipt/bytecode/storage observations, and offline replay. | Honesty, availability, consensus, finality, or correctness of any client or chain. |
| Production-profile decision | The named profile met every specified local, operational, review, and launch-evidence gate. | Canonical production-precheck, review disposition, launch decision, and final correspondence record. | Generic EVM support, all applications, custody safety, or multi-chain readiness. |

## Actors, Authority, and Handoffs

No actor obtains implicit authority from possession of a project checkout,
verifier, test report, or target endpoint.

| Actor | May do | Trusted inputs | Must hand off / may not do |
| --- | --- | --- | --- |
| Release owner | Approve the exact source, release policy, public verification key, and candidate bundle. | Reviewed source and generated local evidence. | Hands bundle identity to policy approver and verifier; may not treat a signature as deployment or chain proof. |
| Policy approver | Approve the named profile, exposure limit, signer policy, and launch decision inputs. | Published Phase 50/51 policy records and evidence matrix. | Hands approved policy identity to deploy operator; may not silently widen profile or override a failed gate. |
| Deploy operator | Prepare explicit target requests and, only after Gate E approval, submit the one allowed action. | Approved request, endpoint, chain ID, signer authority, nonce, fee/gas, and constructor/invocation inputs. | Hands sanitized observations to evidence/replay process; may not discover or substitute inputs. |
| Signer custodian | Operate the signer according to the later approved policy. | Approved request digest and policy identity. | Hands non-secret approval evidence only; may not expose keys, seed material, or credentials. |
| Independent verifier | Validate verifier distribution and bundle-local evidence offline. | Approved verifier package, public keys, trust policy, and bundle. | Hands canonical verifier result to reviewer; may not rely on a source checkout, wallet, endpoint, or network. |
| Independent reviewer | Review the locked artifact/evidence set and record findings. | Frozen identity set, non-secret evidence, scope, and exclusions. | Hands a findings disposition to the policy approver; may not replace local proof/replay gates. |
| Incident owner | Declare stop, preserve non-secret evidence, coordinate disclosure and remediation. | Incident policy and retained evidence. | Hands a closure/remediation decision to policy approver; may not erase or rewrite failed evidence. |

The release owner, policy approver, and deploy operator must be distinct named
roles in the policy even if a small organization assigns multiple roles to one
person. Phase 51 determines whether the production policy permits such a
combination; this lock does not approve it.

## Threat Model and Mandatory Failure Response

### Assets protected

1. The integrity of the bounded source-to-production-profile decision.
2. The identity chain across source, proof, schema, ABI, target artifact,
   release bundle, operator request, observation, and final record.
3. The fixed-owner authorization guard and the integrity of the announced
   scalar configuration revision.
4. The ability of operators and independent reviewers to detect a wrong chain,
   wrong artifact, wrong policy, wrong caller, malformed evidence, or target
   disagreement before accepting an action.
5. Confidential signer/key, endpoint-credential, and environment data, which
   must never enter source, artifacts, reports, fixtures, logs, or retained
   evidence.

### Threats to reject or expose

1. Source, proof, schema, ABI, bytecode, constructor, bundle, policy, request,
   receipt, event, storage, or evidence substitution.
2. Unsafe paths, omitted/duplicated/reordered evidence, stale evidence, or
   cross-chain/profile replay.
3. Unauthorized, malformed, substituted, expired, replacement, nonzero-value,
   or over-exposure target requests.
4. Wrong-chain, endpoint/profile mismatch, malformed client response,
   reverted/pending/replaced receipt, client disagreement, or reorganization.
5. Tampered verifier, public key, trust policy, schema, compatibility rule, or
   reviewer evidence set.
6. Private-key, credential, absolute-path, or environment-data leakage.

### Fail-closed response table

| Condition | Required response |
| --- | --- |
| Source/proof/bundle/verifier/policy identity mismatch | Stop; preserve non-secret failing evidence; identify the changed input; regenerate evidence only under an approved compatibility decision. |
| Missing v2 clean reproduction or unsupported tool identity | Stop before any target action; do not infer acceptance from historical v1 reports or CI. |
| Wrong chain, malformed request, invalid approval, or policy violation | Reject locally before signing/submission and emit a stable, secret-free diagnostic. |
| Revert, receipt mismatch, client disagreement, or reorganization | Do not accept a deployment/invocation result; preserve evidence, stop the workflow, and escalate to the incident owner. |
| Suspected signer/key or endpoint-credential compromise | Stop all target operations, revoke operator use under the approved policy, preserve non-secret evidence, and require a new reviewed policy decision. |
| Unexpected target behavior or disclosure | Preserve identities and observations, publish no compatibility or production claim, and open an incident/review process. |

## Acceptance-to-Evidence Matrix

Every completion statement must name the evidence that supports it. A report may
be an input to multiple later checks, but no row may be satisfied by an
informal assertion or a network observation alone.

| Completion statement | Required canonical evidence | Owning task | Independent check |
| --- | --- | --- | --- |
| The inherited registry candidate is unambiguous. | This lock's inventory plus the referenced v1 source and release/proof contracts. | 50.0.1 | Code/release review; terminology fixture is added in 50.2. |
| The current pilot baseline is eligible for promotion work. | Fresh clean `clg.pilot-precheck-report.v2` full report, digest projection, and compatibility record. | 50.1 | Report validator and clean-directory reproduction. |
| One mainnet profile has defined operational semantics. | Mainnet profile/target-policy lock and terminology fixtures. | 50.2 | Policy review and negative configuration fixtures. |
| An operator request is authorized and safe to submit. | Canonical request, approval/policy record, and validator result. | 51.1-51.3 | Adversarial request/secret-leakage matrix. |
| A target observation corresponds to the approved candidate. | Bound deployment/invocation evidence and offline replay report. | 52.1-52.3 | Tamper and independent-observation fixtures. |
| The candidate satisfies all local production gates. | Canonical `production-precheck` candidate report and support-policy record. | 53.0-53.3 | Report self-validation and clean-environment reproduction. |
| The launch candidate has sufficient independent assurance. | Findings disposition tied to the frozen identity set. | 54.0-54.1 | Independent reviewer and policy-approver review. |
| A production-profile claim is valid. | Approved pre-launch decision, final correspondence record, and closure decision. | 54.3-54.5 | Independent environment reproduction and final evidence review. |

## Exit Criteria

Phase 50.0 is complete only when:

1. this lock is published and referenced from the Milestone 6 roadmap and
   design-lock index;
2. the inherited source, proof, verifier, target, diagnostics, release bundle,
   identity inputs, and exclusions are inventoried without claiming that
   historical v1 evidence is current v2 acceptance;
3. the permitted product, zero asset exposure limit, actors, authority limits,
   threats, non-claims, and fail-closed responses are explicit;
4. every Milestone 6 completion claim maps to a canonical evidence artifact and
   independent check; and
5. the document authorizes no target action and leaves mainnet selection,
   signer policy, and target semantics to their named later phases.

## References

- `README.md`
- `docs/todo/milestone_6_roadmap.md`
- `docs/todo/general_production_readiness_path.md`
- `docs/release/milestone_5_pilot_closure_decision.md`
- `docs/release/milestone_5_pilot_product_contract.md`
- `docs/design/phase-40.0-milestone-5-pilot-product-lock.md`
- `docs/design/phase-41.0-caller-authorization-profile-lock.md`
- `docs/design/phase-30.6.1-contract-release-verification-lock.md`
- `docs/design/verified-profile-packs.md`
- `examples/projects/reference-config-registry/v1/README.md`
