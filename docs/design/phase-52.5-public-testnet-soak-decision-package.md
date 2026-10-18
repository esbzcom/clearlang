# Phase 52.5 - Public-Testnet Soak Decision Package

Date: 2026-10-17

Status: Preparation only. This package authorizes no endpoint contact, signing,
Safe operation, deployment, invocation, or mainnet claim.

## Scope

The soak exercises the unchanged scalar ConfigRegistry semantics under a
separate staging security domain. It uses a staging-only Safe, deployer,
owners, nonces, approvals, credentials, and request identifiers; nothing from
the production domain may be reused. The public-testnet target, endpoint, and
all transaction values remain explicit future operator inputs. This document
does not select them.

## Required Exercises

The approved execution package must contain one explicit record for each:

1. preflight wrong-chain prevention with no target transaction;
2. one zero-value registry deployment;
3. one Safe-authorized zero-value `CALL` to `set_value(next)`;
4. one separately approved expected unauthorized invocation with no successful
   transition or `ValueChanged` event; and
5. one interrupted-operation observation that preserves the request and stops
   without automatic retry, replacement, fee bump, or resubmission.

Every target action needs separate pre-execution authorization. The package
does not turn an approval for one exercise into approval for another.

## Evidence and Decision

Each completed exercise retains canonical secret-free capture input, target
evidence, separately retained identity anchor, offline replay report, and
primary/independent observation records. It must satisfy the Phase 52.4
matrix. Endpoint URLs, credentials, keys, seed phrases, raw signatures,
absolute paths, and device locations are prohibited.

The final soak decision is `not_started`, `incomplete`, `eligible_for_gate_d`,
or `not_eligible`. `eligible_for_gate_d` means only that this bounded testnet
package has the required evidence; it is not a production, mainnet, Safe, or
launch authorization claim. Any anomaly, missing replay, disagreement,
reorganization, unexpected unauthorized success, or interrupted action without
a preserved stop record produces `not_eligible`.

## External Inputs Required Before Execution

An operator must separately supply and approve the staging target profile and
chain ID, endpoint selection and approved secret boundary, staging Safe and
owner evidence, deployer and executor addresses/nonces, release/identity
anchor, explicit request records, fee/value limits, and intended duration.
Until then, this package remains `not_started`.
