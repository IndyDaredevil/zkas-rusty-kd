//! Built-in consensus-divergence diagnostics.
//!
//! # Why this exists
//!
//! When a node rejects a block the chain accepted, the error it raises
//! (`BadCoinbaseTransaction`) is a single tx-hash comparison. It says *that* the node
//! disagreed, never *why*. Every disagreement of this shape has so far been diagnosed by
//! adding temporary log lines, rebuilding, resyncing 68,000 blocks, reading the log, and
//! guessing again — a loop that burned eleven wrong theories on one bug.
//!
//! The information needed to end that loop in a single pass is all present at the moment
//! of failure and is thrown away. This module keeps it: on a coinbase mismatch the node
//! writes a self-contained JSON report naming every input to the decision, plus a
//! `verdict_hint` section that states the likely cause in words.
//!
//! # Enabling
//!
//! `kaspad --consensus-diag[=<dir>]`, or `ZKAS_CONSENSUS_DIAG=<dir>` in the environment.
//! Default dir is `<appdir>/consensus-diag`. When unset, every entry point here is a
//! single relaxed atomic load and nothing is allocated — this must stay free enough to
//! leave compiled into release binaries, because the bugs it catches only appear on
//! mainnet, on someone else's node, once.
//!
//! # Design rule
//!
//! **Diagnostics must never change consensus.** Nothing here is read back by validation.
//! Reports are written after a block has already been judged, on the error path only.

use kaspa_hashes::Hash;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static DIAG_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Point diagnostics at `dir`. Call once at startup, before consensus starts. A `None`
/// leaves the environment variable in charge.
pub fn init(dir: Option<PathBuf>) {
    let _ = DIAG_DIR.set(dir.or_else(env_dir));
}

fn env_dir() -> Option<PathBuf> {
    std::env::var("ZKAS_CONSENSUS_DIAG").ok().filter(|s| !s.is_empty()).map(PathBuf::from)
}

fn dir() -> Option<&'static Path> {
    DIAG_DIR.get_or_init(env_dir).as_deref()
}

/// Whether to collect the per-decision detail a report needs. Checked on the hot path.
pub fn enabled() -> bool {
    dir().is_some()
}

/// A sibling of the anchor's source block — a block with the same selected parent, and so a
/// candidate to have minted the same notes and carried the same shielded tree root.
///
/// Siblings matter because `anchor_block` is a single-valued, last-write-wins map. When
/// several blocks share a root, *which* one a node has indexed depends on the order it
/// happened to validate them. A node that indexed a non-canonical producer fails the
/// chain-ancestor test and drops the spend; a node that indexed the canonical one keeps it.
///
/// `root_matches_anchor` is `None` when this node never persisted the sibling's shielded
/// state — which is the normal case for an orphan on a freshly synced node, since only
/// chain candidates are persisted. `None` is therefore not "ruled out", it is "unknown", and
/// the distinction is the whole point: the node that *accepted* the block is precisely the
/// one that would have persisted it.
#[derive(Serialize, Clone, Debug)]
pub struct SiblingBlock {
    pub block: String,
    pub blue_score: u64,
    /// On the selected chain of the block being validated.
    pub is_chain_ancestor: bool,
    /// Whether this node ever computed and stored a shielded tree for this block.
    pub shielded_state_persisted: bool,
    /// `Some(true)` ⇒ confirmed to carry the same root. `None` ⇒ unknown, see above.
    pub root_matches_anchor: Option<bool>,
    /// This is the block `anchor_block[root]` currently points at on this node.
    pub indexed_as_source: bool,
}

/// How confident this node is that the anchor→source mapping is unique.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Ambiguity {
    /// The source block has no siblings: the mapping cannot be order-dependent.
    None,
    /// A sibling is confirmed to carry the identical root. The mapping IS order-dependent.
    Confirmed,
    /// Siblings exist whose shielded state this node never computed, so they can neither be
    /// confirmed nor excluded from having produced this root — from here.
    Unverifiable,
}

/// How this node resolved one shielded spend's anchor, and how close the call was.
#[derive(Serialize, Clone, Debug)]
pub struct AnchorResolution {
    pub anchor: String,
    pub source: Option<String>,
    pub source_blue_score: Option<u64>,
    pub is_chain_ancestor: Option<bool>,
    pub age: Option<u64>,
    pub window_min: u64,
    pub window_max: u64,
    pub verdict: bool,
    pub reject_reason: Option<String>,
    /// Blocks sharing `source`'s selected parent — candidates to have produced the same root.
    pub source_siblings: Vec<SiblingBlock>,
    pub ambiguity: Ambiguity,
}

/// The fate of one shielded transaction in this block's mergeset, with the two drop
/// reasons kept apart — they point at completely different bugs.
#[derive(Serialize, Clone, Debug)]
pub struct ShieldedDecision {
    pub txid: String,
    pub source_block: String,
    pub is_selected_parent: bool,
    pub fee: u64,
    pub nullifier_count: usize,
    pub anchor: String,
    pub anchor_ok: bool,
    pub conflict_ok: bool,
    pub kept: bool,
    pub drop_reason: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct CoinbaseOutputDiff {
    pub index: usize,
    pub expected_value: Option<u64>,
    pub actual_value: Option<u64>,
    pub expected_script_prefix: Option<String>,
    pub actual_script_prefix: Option<String>,
    pub matches: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct MergesetReward {
    pub block: String,
    pub subsidy: u64,
    pub total_fees: u64,
    pub is_selected_parent: bool,
}

/// Everything that fed one rejected block's coinbase decision.
#[derive(Serialize, Clone, Debug)]
pub struct BlockDivergenceReport {
    pub schema: &'static str,
    pub kind: &'static str,
    pub block: String,
    pub daa_score: u64,
    pub blue_score: u64,
    pub selected_parent: String,
    pub mergeset: Vec<String>,

    /// False ⇒ the shielded state root itself diverged. True ⇒ state agrees and the
    /// disagreement is purely about which fees were re-minted.
    pub payload_equal: bool,
    pub shielded_commitment_at_selected_parent: Option<String>,
    pub shielded_tree_size_at_selected_parent: Option<u64>,
    pub global_nullifier_count: Option<u64>,
    pub global_set_matches_snapshot: Option<bool>,

    pub coinbase_outputs: Vec<CoinbaseOutputDiff>,
    /// `expected - actual`, summed over outputs. Matching this against a fee below
    /// identifies the responsible transaction immediately.
    pub value_delta: i128,
    pub mergeset_rewards: Vec<MergesetReward>,
    pub shielded_decisions: Vec<ShieldedDecision>,
    pub anchors: Vec<AnchorResolution>,

    /// Plain-language reading of the evidence above. See [`derive_hints`].
    pub verdict_hint: Vec<String>,
}

impl BlockDivergenceReport {
    pub const SCHEMA: &'static str = "zkas.consensus-divergence/1";
}

/// Turn the collected evidence into statements about what went wrong.
///
/// These are ordered most-specific first and are deliberately blunt: the point is that a
/// person reading one file learns the cause without a rebuild. Each rule fires only on a
/// pattern that has actually been observed and confirmed on mainnet.
pub fn derive_hints(r: &BlockDivergenceReport) -> Vec<String> {
    let mut hints = Vec::new();

    // Rule 1: an ambiguous anchor. This is the DAA 371,851 bug. Sibling blocks sharing a
    // selected parent can mint identical reward notes and therefore carry an identical tree
    // root; the anchor index keeps only the last one written.
    for a in r.anchors.iter().filter(|a| a.ambiguity == Ambiguity::Confirmed) {
        let non_canon: Vec<&SiblingBlock> =
            a.source_siblings.iter().filter(|b| b.root_matches_anchor == Some(true) && !b.is_chain_ancestor).collect();
        hints.push(format!(
            "AMBIGUOUS ANCHOR {}: a sibling of the source block carries this exact shielded root, {} of them NOT on the selected chain. \
             `anchor_block` is last-write-wins, so which block this node resolved ({}) depends on the order it validated them — \
             including orphans a differently-connected node never saw. A node that indexed a non-canonical producer fails the \
             chain-ancestor test and DROPS the spend; this node kept it. This is a consensus non-determinism, not a state divergence.",
            short(&a.anchor),
            non_canon.len(),
            a.source.as_deref().map(short).unwrap_or_else(|| "<unresolved>".into()),
        ));
        for b in non_canon {
            hints.push(format!(
                "  ↳ non-canonical producer {} (blue {}) resolves this anchor to a block that is not a chain ancestor ⇒ fail-closed ⇒ spend dropped.",
                short(&b.block),
                b.blue_score
            ));
        }
    }

    // Rule 1b: siblings exist but this node never computed their shielded state, so it cannot
    // confirm or exclude them from here. A freshly synced node persists only chain candidates,
    // so this is the EXPECTED shape of the ambiguity bug as seen from the rejecting side — the
    // evidence lives on the node that accepted the block. Say exactly how to go get it.
    for a in r.anchors.iter().filter(|a| a.ambiguity == Ambiguity::Unverifiable) {
        let unknown: Vec<&SiblingBlock> = a.source_siblings.iter().filter(|b| !b.shielded_state_persisted).collect();
        hints.push(format!(
            "POSSIBLE ANCHOR AMBIGUITY {}: the source block {} has {} sibling(s) whose shielded state this node never computed \
             (it only persists chain candidates, so an orphan is invisible here). If a node that ACCEPTED this block indexed this \
             anchor to one of them, that node fails the chain-ancestor test and drops the spend — which is exactly the disagreement \
             seen here. To confirm, run on a node that followed the chain live: {}. If any returns the same tree root as this \
             anchor, the anchor→source mapping is order-dependent and this is a consensus non-determinism, not a state divergence.",
            short(&a.anchor),
            a.source.as_deref().map(short).unwrap_or_else(|| "<unresolved>".into()),
            unknown.len(),
            unknown.iter().map(|b| format!("getShieldedTreeState({})", b.block)).collect::<Vec<_>>().join(" ; "),
        ));
    }

    // Rule 2: the value gap equals a specific transaction's fee.
    if r.value_delta != 0 {
        let mag = r.value_delta.unsigned_abs();
        for d in &r.shielded_decisions {
            if u128::from(d.fee) == mag {
                hints.push(format!(
                    "The coinbase differs by exactly {} sompi, which is the fee of shielded tx {} (kept={} on this node). \
                     The chain made the OPPOSITE keep/drop call on that transaction.",
                    d.fee,
                    short(&d.txid),
                    d.kept
                ));
            }
        }
    }

    // Rule 3: state agrees, only the fee decision differs. Rules out a whole class.
    if r.payload_equal && r.value_delta != 0 {
        hints.push(
            "payload_equal=true ⇒ this node's shielded state root at the selected parent MATCHES the chain's. \
             The note-commitment tree, nullifier set and supply are all in agreement; only the applied-spend set differs. \
             Do NOT go looking for tree or nullifier corruption."
                .to_string(),
        );
    }
    if !r.payload_equal {
        hints.push(
            "payload_equal=false ⇒ the shielded state root itself diverged at the selected parent. \
             Compare `getShieldedTreeState` against a node that followed the chain live, walking back to the first block where they differ."
                .to_string(),
        );
    }

    // Rule 4: a near-threshold anchor. Wallets deliberately anchor just past the maturity
    // depth, so this predicate runs permanently close to its own boundary.
    for a in r.anchors.iter() {
        if let Some(age) = a.age {
            let margin = age.saturating_sub(a.window_min);
            if a.verdict && margin < 64 {
                hints.push(format!(
                    "Anchor {} passed maturity by only {} blue score ({} vs minimum {}). Any difference in which \
                     block a node resolves this root to would flip the verdict.",
                    short(&a.anchor),
                    margin,
                    age,
                    a.window_min
                ));
            }
        }
    }

    // Rule 5: nullifier bookkeeping drifted from what the state root commits to.
    if r.global_set_matches_snapshot == Some(false) {
        hints.push(
            "global_set_matches_snapshot=false ⇒ the live nullifier set has drifted from the snapshot the state root commits to. \
             Conflict resolution is deciding against a set the chain never had — suspect reorg revert/re-apply ordering."
                .to_string(),
        );
    }

    if hints.is_empty() {
        hints.push(
            "No known pattern matched. The per-decision detail above is complete: compare this file against the same block's \
             report from a node that accepted it, field by field."
                .to_string(),
        );
    }
    hints
}

fn short(h: &str) -> String {
    h.chars().take(12).collect()
}

/// Write a report and log where it went. Never panics and never propagates an error:
/// a diagnostic that can take the node down is worse than no diagnostic.
pub fn write_report(mut report: BlockDivergenceReport) {
    let Some(dir) = dir() else { return };
    report.verdict_hint = derive_hints(&report);

    if let Err(e) = std::fs::create_dir_all(dir) {
        kaspa_core::warn!("consensus-diag: cannot create {}: {e}", dir.display());
        return;
    }
    let path = dir.join(format!("{}-{}.json", report.daa_score, short(&report.block)));
    let json = match serde_json::to_string_pretty(&report) {
        Ok(j) => j,
        Err(e) => {
            kaspa_core::warn!("consensus-diag: cannot serialize report: {e}");
            return;
        }
    };
    match std::fs::write(&path, json) {
        Ok(()) => {
            kaspa_core::warn!("consensus-diag: wrote divergence report to {}", path.display());
            for h in &report.verdict_hint {
                kaspa_core::warn!("consensus-diag: {h}");
            }
        }
        Err(e) => kaspa_core::warn!("consensus-diag: cannot write {}: {e}", path.display()),
    }
}

pub fn hex32(b: &[u8; 32]) -> String {
    faster_hex::hex_string(b)
}

pub fn hash_str(h: Hash) -> String {
    h.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> BlockDivergenceReport {
        BlockDivergenceReport {
            schema: BlockDivergenceReport::SCHEMA,
            kind: "coinbase_mismatch",
            block: "f44b2f15aaaa".into(),
            daa_score: 371851,
            blue_score: 370225,
            selected_parent: "a998250c86db".into(),
            mergeset: vec![],
            payload_equal: true,
            shielded_commitment_at_selected_parent: None,
            shielded_tree_size_at_selected_parent: None,
            global_nullifier_count: None,
            global_set_matches_snapshot: None,
            coinbase_outputs: vec![],
            value_delta: 0,
            mergeset_rewards: vec![],
            shielded_decisions: vec![],
            anchors: vec![],
            verdict_hint: vec![],
        }
    }

    /// The DAA 371,851 signature: one root, two producers, one of them an orphan.
    #[test]
    fn names_an_ambiguous_anchor_as_the_cause() {
        let mut r = base();
        r.anchors.push(AnchorResolution {
            anchor: "671ab9ec3271".into(),
            source: Some("da8dfb9d7ad4".into()),
            source_blue_score: Some(368939),
            is_chain_ancestor: Some(true),
            age: Some(1286),
            window_min: 600,
            window_max: 27000,
            verdict: true,
            reject_reason: None,
            source_siblings: vec![SiblingBlock {
                block: "e6f50b473e5f".into(),
                blue_score: 368939,
                is_chain_ancestor: false,
                shielded_state_persisted: true,
                root_matches_anchor: Some(true),
                indexed_as_source: false,
            }],
            ambiguity: Ambiguity::Confirmed,
        });
        let hints = derive_hints(&r);
        assert!(hints[0].contains("AMBIGUOUS ANCHOR"), "{hints:?}");
        assert!(hints.iter().any(|h| h.contains("e6f50b473e5f")), "names the orphan producer: {hints:?}");
    }

    /// The shape the REJECTING node actually sees: the orphan is in its DAG but its shielded
    /// state was never computed, so the root cannot be compared from here. The report must still
    /// raise the ambiguity and say exactly which blocks to query elsewhere — silence here is what
    /// sent the investigation chasing state corruption for days.
    #[test]
    fn raises_unverifiable_ambiguity_and_says_how_to_confirm_it() {
        let mut r = base();
        r.anchors.push(AnchorResolution {
            anchor: "671ab9ec3271".into(),
            source: Some("da8dfb9d7ad4".into()),
            source_blue_score: Some(368939),
            is_chain_ancestor: Some(true),
            age: Some(1286),
            window_min: 600,
            window_max: 27000,
            verdict: true,
            reject_reason: None,
            source_siblings: vec![SiblingBlock {
                block: "e6f50b473e5f2f186a23025d687f94db9f798f028c4eb35db85356299f1781d5".into(),
                blue_score: 368939,
                is_chain_ancestor: false,
                shielded_state_persisted: false,
                root_matches_anchor: None,
                indexed_as_source: false,
            }],
            ambiguity: Ambiguity::Unverifiable,
        });
        let hints = derive_hints(&r);
        assert!(hints.iter().any(|h| h.contains("POSSIBLE ANCHOR AMBIGUITY")), "{hints:?}");
        assert!(
            hints.iter().any(|h| h.contains("getShieldedTreeState(e6f50b473e5f2f186a23025d687f94db9f798f028c4eb35db85356299f1781d5)")),
            "must name the exact command and full block hash to run elsewhere: {hints:?}"
        );
    }

    #[test]
    fn matches_the_value_gap_to_a_transaction_fee() {
        let mut r = base();
        r.value_delta = 24_578_600;
        r.shielded_decisions.push(ShieldedDecision {
            txid: "10351eded2ab".into(),
            source_block: "a998250c86db".into(),
            is_selected_parent: true,
            fee: 24_578_600,
            nullifier_count: 21,
            anchor: "671ab9ec3271".into(),
            anchor_ok: true,
            conflict_ok: true,
            kept: true,
            drop_reason: None,
        });
        let hints = derive_hints(&r);
        assert!(hints.iter().any(|h| h.contains("10351eded2ab") && h.contains("24578600")), "{hints:?}");
    }

    /// A matching payload must steer the reader away from tree/nullifier corruption —
    /// that wrong turn cost several days.
    #[test]
    fn rules_out_state_divergence_when_the_payload_matches() {
        let mut r = base();
        r.value_delta = 1;
        let hints = derive_hints(&r);
        assert!(hints.iter().any(|h| h.contains("Do NOT go looking for tree or nullifier corruption")), "{hints:?}");
    }

    #[test]
    fn flags_an_anchor_that_only_just_cleared_maturity() {
        let mut r = base();
        r.anchors.push(AnchorResolution {
            anchor: "aabbccdd0011".into(),
            source: Some("1111111111".into()),
            source_blue_score: Some(1000),
            is_chain_ancestor: Some(true),
            age: Some(635),
            window_min: 600,
            window_max: 27000,
            verdict: true,
            reject_reason: None,
            source_siblings: vec![],
            ambiguity: Ambiguity::None,
        });
        let hints = derive_hints(&r);
        assert!(hints.iter().any(|h| h.contains("passed maturity by only 35")), "{hints:?}");
    }

    #[test]
    fn always_says_something() {
        assert_eq!(derive_hints(&base()).len(), 1);
        assert!(derive_hints(&base())[0].contains("No known pattern matched"));
    }
}
