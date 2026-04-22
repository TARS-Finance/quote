# Local Policy-Driven Strategy Generation

## Summary

This design replaces static `strategy_path` input for local single-solver development with in-process strategy generation from:

- `Settings.toml` for service runtime
- `policy.toml` for solver chain and route policy data
- local `chain.json` for concrete asset metadata

The target scope is a local smoke-test setup for:

- `base_sepolia`
- `arbitrum_sepolia`
- `bitcoin_testnet`

The system must support:

- EVM -> EVM quotes and order creation
- EVM -> Bitcoin quotes and order creation
- Bitcoin -> EVM quotes and order creation

Pricing remains live. Static prices are explicitly out of scope.

## Goals

- Remove the need for a pre-generated local strategies file in the single-solver local workflow.
- Reuse the same policy semantics already used by `solver-comms` and the shared `policy` crate.
- Generate concrete quoteable strategies in-process in the shape already consumed by `orderbook`.
- Preserve existing quote and create-order behavior above the strategy registry layer.
- Correct Bitcoin identity handling so liquidity lookup and order identity do not conflict.

## Non-Goals

- Multi-solver route aggregation.
- Running `solver-agg-v2` locally.
- Reworking quote math, order persistence, or the public API surface.
- Supporting all catalog chains in phase 1.

## Reference Flow

The current external flow is:

1. `solver-comms` parses solver chain config and `SolverPolicyConfig`
2. `solver-comms` POSTs chains and policy to `solver-agg-v2`
3. Quote-side infrastructure consumes concrete strategy records
4. Quote service indexes strategies and serves `/quote`

For local single-solver development, this design keeps the same logical flow but collapses it into one process:

1. `orderbook` parses trimmed local `policy.toml`
2. `orderbook` loads local `chain.json`
3. `orderbook` constructs the same effective chain and policy inputs in memory
4. `orderbook` uses the shared policy engine to validate and annotate routes
5. `orderbook` emits concrete runtime strategy records directly into `StrategyRegistry`

No remote solver aggregator is involved.

## Runtime Files

### Settings.toml

`Settings.toml` remains the root runtime config for the service. It keeps:

- bind address
- local Postgres URL
- chain metadata path
- pricing config
- quote config
- chain ID mapping

It adds a path to `policy.toml`.

### policy.toml

`policy.toml` is a trimmed local-only file derived from the `solver-comms` config model. It contains only:

- `solver_id`
- `solver_name`
- `[chains.*]`
- `[policy]`

Each chain section preserves `solver-comms` semantics:

- `rpc_url`
- `native_decimals`
- `native_asset_id`
- `address`
- optional `solver_account`
- `supported_assets`

The `[policy]` section preserves the `SolverPolicyConfig` shape:

- default policy
- isolation groups
- blacklist pairs
- whitelist overrides
- default fee
- default max slippage
- default confirmation target
- per-route overrides
- max limits

### chain.json

`chain.json` remains the source of concrete asset metadata:

- chain names
- asset IDs
- HTLC addresses
- token addresses
- decimals
- min/max amounts
- timelocks
- token identifiers for pricing

For phase 1 it should be minimal and contain only:

- `base_sepolia`
- `arbitrum_sepolia`
- `bitcoin_testnet`

## Reused Code And Ownership

### Reuse as-is

The following shared policy logic should be reused directly:

- `tars-rs/crates/policy/src/primitives.rs`
- `tars-rs/crates/policy/src/solver_policy.rs`
- `tars-rs/crates/policy/src/collections.rs`
- `tars-rs/crates/policy/src/common.rs`
- `tars-rs/crates/policy/src/errors.rs`

These files already define:

- `SolverPolicyConfig`
- `SolverPolicy`
- route validation
- fee lookup
- slippage lookup
- confirmation target lookup
- source amount lookup

### Copy or adapt from solver-comms

`orderbook` should introduce a local `policy.toml` parser derived from:

- `~/Desktop/catalog/solver-comms/src/settings.rs`

Only the local trimmed fields are needed, but chain semantics must stay aligned with `solver-comms`.

### Reference from quote

Quote-side behavior matters in two places:

- concrete strategy shape
- strategy cache/index behavior

The concrete output must remain compatible with the strategy schema consumed by the quote path. The indexing behavior should remain in `orderbook`’s own registry layer.

The quote service should not be modified to consume policy directly.

## Strategy Expansion Rules

### Inputs

The builder takes:

- parsed `policy.toml`
- loaded `chain.json`

The builder does not fetch policy, chains, or strategies from remote services.

### Candidate assets

Candidate assets are the intersection of:

- chains present in `policy.toml`
- asset IDs listed in each `[chains.*].supported_assets`
- assets present in `chain.json`

Anything outside that intersection is ignored.

### Candidate routes

For phase 1, the builder generates every directed cross-chain route across the selected scope:

- Base -> Arbitrum
- Arbitrum -> Base
- Base -> Bitcoin
- Bitcoin -> Base
- Arbitrum -> Bitcoin
- Bitcoin -> Arbitrum

Same-chain routes are excluded in phase 1.

### Route validation

Each candidate source/destination pair is validated using `SolverPolicy`:

1. both assets must be supported
2. isolation rules must allow the pair
3. blacklist rules must not block the pair
4. whitelist overrides must unblock explicitly allowed pairs

Only validated routes become concrete strategies.

### Route annotations

For each valid route, the builder derives:

- `min_amount` and `max_amount` from `SolverPolicy::get_source_amount(...)`
- `fee` and `fixed_fee` from `SolverPolicy::get_fee(...)`
- `max_slippage` from `SolverPolicy::get_max_slippage(...)`
- `min_source_confirmations` from `SolverPolicy::get_confirmation_target(...)`

### Timelocks

Timelock values come from metadata, not from policy:

- `min_source_timelock` comes from the source asset metadata
- `destination_timelock` comes from the destination asset metadata

This preserves the existing orderbook expectation that concrete strategies already carry resolved timelock values.

## Concrete Strategy Shape

The builder must emit concrete strategy records in the same effective shape already consumed by `orderbook`:

- `id`
- `source_chain_address`
- `dest_chain_address`
- `source_chain`
- `dest_chain`
- `source_asset`
- `dest_asset`
- `makers`
- `min_amount`
- `max_amount`
- `min_source_timelock`
- `destination_timelock`
- `min_source_confirmations`
- `fee`
- `fixed_fee`
- `max_slippage`

The registry then indexes those strategies exactly as it does today.

## Bitcoin Identity Split

Bitcoin requires two different identities:

- `address`: bech32 address used for on-chain balance lookup
- `solver_account`: x-only pubkey used for order identity and committed-funds identity

For Bitcoin chains:

- liquidity watchers must use `address` for balance fetches
- generated strategies must use `solver_account` as the solver-side chain identity
- committed-funds lookups must use `solver_account`

For EVM chains:

- `address` serves both roles

This matches the intended `solver-comms` behavior while allowing quoteability and correct order creation at the same time.

## Service Wiring

Startup becomes:

1. load `Settings.toml`
2. load `chain.json`
3. load `policy.toml`
4. build in-memory concrete strategies
5. create `StrategyRegistry`
6. start pricing refresh
7. start liquidity watcher
8. serve HTTP

File-based strategy loading can remain as a compatibility fallback, but the local smoke-test path should use `policy.toml`.

## Pricing

Pricing remains live and continues to use the existing market-data provider path.

Requirements:

- no static prices
- BTC and USDC prices resolve from the existing provider configuration
- `/quote` fails clearly if required prices are missing

This keeps local smoke tests representative of real quote behavior.

## Testing And Verification

### Unit tests

- parse trimmed `policy.toml`
- expand concrete strategies from `policy.toml + chain.json`
- verify all valid directed routes across the three-chain scope
- verify policy overrides affect generated strategies correctly
- verify Bitcoin bech32 vs x-only split

### Integration tests

- startup succeeds with local Postgres and local files
- strategy registry contains EVM -> EVM, EVM -> Bitcoin, and Bitcoin -> EVM routes
- `/quote` returns valid routes with live prices
- `/orders` persists matched orders using generated strategies

### Runtime smoke tests

Required flows:

- `base_sepolia:usdc -> arbitrum_sepolia:usdc`
- `base_sepolia:usdc -> bitcoin_testnet:btc`
- `bitcoin_testnet:btc -> base_sepolia:usdc`

For each:

- `GET /quote` succeeds
- `POST /orders` succeeds
- persisted order is readable from the local store

## Success Criteria

Phase 1 is complete when:

- local startup works with local Postgres
- strategies are generated from local `policy.toml`
- live pricing works
- quote requests succeed for the three route classes
- create-order succeeds for the three route classes
- Bitcoin liquidity lookup and order identity both work correctly
