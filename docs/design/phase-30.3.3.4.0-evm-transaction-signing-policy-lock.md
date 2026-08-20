# Phase 30.3.3.4.0 - EVM Transaction Signing Policy Lock

Date: 2026-08-20
Status: Locked
Owner: target-runtime-owner

## Purpose

Define explicit signing and submission rules for `clg target deploy` and `clg target invoke`.
The policy prevents ambient wallets, inferred fees, or an unsigned simulation from being confused
with an on-chain transaction.

## Policy

- The only supported signer input is a local JSON `clg.evm-signing-key.v1` file containing a
  secp256k1 private key and its derived lowercase `0x` EVM address. The supplied `--sender` must
  equal the derived address.
- The first envelope is legacy EIP-155. Every deploy/invoke requires explicit `--nonce`,
  `--gas-limit`, and `--gas-price`; no value is discovered from an endpoint or environment.
- The transaction chain ID is the explicit command value, confirmed by `eth_chainId` before any
  submission. The command submits only an RLP-encoded signed transaction with
  `eth_sendRawTransaction`.
- A command polls `eth_getTransactionReceipt` by the returned transaction hash (at most ten
  explicit polls, 100 ms apart). It emits a
  successful `clg.target-receipt.v1` only when receipt status is `0x1` and block number/hash are
  present and well formed. RPC errors, malformed responses, timeouts, status `0x0`, and chain-ID
  mismatch fail without a success receipt.

## Non-Goals

EIP-1559 envelopes, account/wallet discovery, remote signing, fee estimation, automatic nonce
selection, transaction replacement, and confirmation-depth policies are deferred.
