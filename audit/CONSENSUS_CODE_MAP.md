# Consensus-Critical Code Map (zkas-rusty @ fd86c69)

Delta vs fork base `33903e0` (upstream rusty-kaspa 2026-06-24). Line counts = fork diff.

## Fork-added / fork-modified consensus files (primary review scope)

| Area | Files (Δ lines) | Key functions |
|---|---|---|
| AuxPoW / merged mining | `consensus/core/src/auxpow.rs` (+390), `consensus/pow/src/auxpow.rs` (+370), `crypto/hashes/src/pow_hashers.rs` (+449), `consensus/pow/src/lib.rs` (+68) | `verify_aux_pow`, `calc_block_level_gated`, `check_pow_gated` |
| Bridge peg-in | `consensus/pow/src/pegin.rs` (+189), `crypto/merkle/src/lib.rs` (+186) | `KaspaBurnProof::verify`, `KaspaBurnProof::claim`, `create/verify_tx_merkle_witness` |
| Shielded state machine | `consensus/src/processes/shielded.rs` (+1046), `consensus/src/model/stores/shielded.rs` (+632), `consensus/core/src/zkas_state_binding.rs` (+204) | `apply/revert`, `shielded_state_root`, export/verify/seed (IBD), MuHash |
| Shielded core lib | `shielded-core/src/{bundle,verify,state,turnstile,nullifier,tree,commitment,burn,attestation,witness_chain,payment_check,coinbase}.rs` | `MAX_ACTIONS_PER_BUNDLE=512`, `BRIDGE_ENABLED=false`, `action_count_from_bytes` |
| Coinbase / emission | `consensus/src/processes/coinbase.rs` (+340), `consensus/core/src/coinbase.rs` | subsidy schedule, tail 0.6, 5% dev fee, `shielded_commitment` field |
| Mass | `consensus/core/src/mass/mod.rs` (+36) | `SHIELDED_MASS_PER_ACTION=1000`, shielded storage-mass zero path |
| Virtual processor | `consensus/src/pipeline/virtual_processor/processor.rs` (+277), `utxo_validation.rs` (+300) | `is_shielded_anchor_final`, dropped-spend fee handling, reorg nullifier WriteBatch |
| Header/body pipeline | `header_processor/pre_ghostdag_validation.rs` (+18), `header_processor/processor.rs` (+3), `body_processor/processor.rs` (+6), `body_processor/body_validation_in_context.rs` (+47) | aux-aware level, shielded coinbase checks |
| Difficulty | `consensus/src/processes/difficulty.rs` (+167), `window.rs` (+4) | DAA under merged mining |
| Genesis / params | `consensus/core/src/config/genesis.rs` (+113), `params.rs` (+246), `network.rs` (+59) | activation heights, ports, dev-fee params, genesis re-cut |
| IBD / sync | `protocol/flows/src/ibd/flow.rs` (+83), `ibd/streams.rs` (+108), `v10/request_pruning_point_shielded_state.rs` (new), `consensus/src/processes/sync/mod.rs` (+24), `pruning_proof/{apply,validate,mod}.rs` (+56), `model/stores/pruning_meta.rs` (+16) | `sync_new_shielded_state`, shielded PP export/seed |
| Tx validation | `tx_validation_in_isolation.rs` (+56 shielded/bridge guards), `consensus/core/src/tx.rs` (+29), `errors/*` (+21) | `check_shielded_in_isolation`, `BundleExtractError::BridgeDisabled` |
| Scripts | `crypto/txscript/src/{script_class,standard}.rs` (+37) | shielded address/script classes |
| Mempool (policy, consensus-adjacent) | `mining/src/mempool/{check_transaction_standard(+23),model/utxo_set(+191),model/transactions_pool(+13),replace_by_fee(+10)}` | shielded mass cap exemption, nullifier conflict tracking |
| RPC/p2p wire | `protocol/p2p/proto/*`, `convert/*`, `rpc/*` | new messages 64–67 (shielded state) |
| Core header/tx types | `consensus/core/src/header.rs` (+31) | auxpow fields, hash coverage |

## Inherited upstream core (secondary scope — spot-check at fork touch points)

- `consensus/src/processes/ghostdag/` — ordering
- `consensus/src/processes/past_median_time.rs`, `dagtraversalmanager`, `reachability`
- `consensus/src/pipeline/{header_processor,body_processor,virtual_processor,pruning_processor}` (unmodified parts)
- `consensus/src/processes/transaction_validator/` — sig/script checks
- `indexes/`, `database/`, `components/consensusmanager/` — storage, session lifecycle
- `notify/`, `rpc/` — non-consensus (out of scope except message validation)

## Out of scope per operator instruction

payment gateway (`gateway/`), pool, SDK (`sdk/`), wallets (`wallet/`, `zkas-walletd`,
`shielded-wallet`), relayer (`zkas-relayer`), explorer/indexer (`zkas-api`), miner app,
cli, wasm bindings, devops/CI/docs build scripts.
