//! Merged-mining (AuxPoW) support for the stratum bridge — the pool-side engine that
//! lets ASICs merge-mine ZKas.
//!
//! It plugs in as a decorator around [`crate::kaspaapi::KaspaApi`] so the entire
//! stratum/job/share-validation core stays untouched:
//!
//! * **get_block_template** — fetch a ZKas template, then hand the ASIC a
//!   *parent* (Kaspa-shaped) block whose single coinbase commits to the ZKas
//!   block hash `H_fc` and whose target (`bits`) is the ZKas target. The ASIC
//!   grinds the parent's kHeavyHash exactly as it would a normal job.
//! * **submit_block** — when a share clears the target, the "block" the bridge hands
//!   back is that solved parent. We rebuild the [`AuxPow`] from it and submit the
//!   *ZKas* block carrying the aux proof (the node accepts it via Option-2 dual
//!   acceptance). Because the aux rides `RpcRawHeader.aux_pow`, the existing
//!   `(&block).into()` submit path transmits it unchanged.
//!
//! In this first version the parent is synthetic (self-generated), which proves the
//! ASIC→aux path end to end. Swapping the synthetic parent for a live Kaspa
//! `getBlockTemplate` (and also submitting winning parents to Kaspa) is what adds the
//! second-chain reward — the struct is identical, only the parent's source changes.

use std::collections::{HashMap, HashSet, VecDeque};

use kaspa_consensus_core::{
    auxpow::AuxPow, block::Block, hashing, header::Header, merkle::calc_hash_merkle_root, subnets::SUBNETWORK_ID_COINBASE,
    tx::Transaction,
};
use kaspa_hashes::{Hash, ZERO_HASH};

/// Build the coinbase (leaf 0) Merkle inclusion branch for a *real* multi-tx parent,
/// i.e. the sequence of right-sibling hashes from the coinbase up to the parent's
/// `hash_merkle_root`. Reproduces Kaspa's tx Merkle tree
/// ([`kaspa_consensus_core::merkle::calc_hash_merkle_root`]): a full binary tree padded
/// to the next power of two, where a wholly-absent right subtree folds against
/// `ZERO_HASH` (matching `calc_merkle_root_with_hasher`'s `unwrap_or(ZERO_HASH)`).
///
/// The coinbase is always leaf 0, hence the left child at every level, so the branch is
/// exactly what [`AuxPow::verify_coinbase_inclusion`] folds (`acc = merkle_hash(acc,
/// sibling)`). Returns an empty branch for a single-tx parent (coinbase is the root).
pub fn coinbase_merkle_branch(txs: &[Transaction]) -> Vec<Hash> {
    if txs.len() <= 1 {
        return vec![];
    }
    // Leaves = per-tx hashes, padded to a power of two with `None` (absent leaves).
    let mut level: Vec<Option<Hash>> = txs.iter().map(|t| Some(hashing::tx::hash(t))).collect();
    level.resize(level.len().next_power_of_two(), None);

    let mut branch = Vec::new();
    let mut idx = 0usize; // coinbase index; stays 0 (leftmost) all the way up
    while level.len() > 1 {
        // Right sibling of the coinbase path. An absent subtree contributes ZERO_HASH,
        // exactly as consensus folds it.
        branch.push(level[idx ^ 1].unwrap_or(ZERO_HASH));
        let mut next = Vec::with_capacity(level.len() / 2);
        for pair in level.chunks(2) {
            let combined = match pair[0] {
                None => None,
                Some(l) => Some(kaspa_merkle::merkle_hash(l, pair.get(1).copied().flatten().unwrap_or(ZERO_HASH))),
            };
            next.push(combined);
        }
        idx /= 2;
        level = next;
    }
    branch
}

/// Which merge-mining form a ZKas block at a given DAA score uses, and the genesis the bound form
/// binds to. Mainnet parameters decide. `ZKAS_NETWORK` (mainnet|testnet|devnet|simnet) picks the
/// network whose genesis and activation apply; `ZKAS_SECURITY_FORK_DAA` overrides the activation
/// score on a test network only. Semantics follow firecash/solo-dual-mode v1.0.12 (643dfdf) so one
/// set of env names works for both bridges.
fn fork_params() -> (Option<u64>, kaspa_consensus_core::config::params::Params) {
    use kaspa_consensus_core::network::NetworkType;
    static P: std::sync::OnceLock<(Option<u64>, kaspa_consensus_core::config::params::Params)> = std::sync::OnceLock::new();
    P.get_or_init(|| {
        // An unknown value would silently bind every merged block to the wrong genesis; refuse it
        // instead of guessing mainnet.
        let raw = std::env::var("ZKAS_NETWORK").unwrap_or_default();
        let net = match raw.as_str() {
            "" | "mainnet" => NetworkType::Mainnet,
            "testnet" => NetworkType::Testnet,
            "devnet" => NetworkType::Devnet,
            "simnet" => NetworkType::Simnet,
            other => panic!("ZKAS_NETWORK={other:?} is not one of mainnet|testnet|devnet|simnet"),
        };
        // Never on mainnet: a test value left in a production launcher would build the wrong form
        // and lose every merged block on one side of the real activation.
        let daa = std::env::var("ZKAS_SECURITY_FORK_DAA").ok().and_then(|v| v.parse().ok()).filter(|_| net != NetworkType::Mainnet);
        (daa, kaspa_consensus_core::config::params::Params::from(net))
    })
    .clone()
}

/// One line naming what the bound form binds to, so a wrong network or a build without the fork
/// is visible at startup (and an invalid `ZKAS_NETWORK` aborts here, not at the first merged job).
pub fn fork_config_summary() -> String {
    let (override_daa, params) = fork_params();
    format!(
        "merge-mining fork config: network={} genesis={} security_fork_activation={:?} override={:?}",
        params.net, params.genesis.hash, params.security_fork_activation, override_daa
    )
}

/// `Some(genesis)` when a ZKas block at `daa_score` must use the security-fork (bound, `ZKM1`)
/// form; `None` for the legacy (`ZKMM`) form. A pure function of the block's OWN score — the same
/// rule the node applies per header — so a template is judged the same way at build, at solve and
/// at submit, however long it sat in a cache.
fn bound_form(daa_score: u64) -> Option<Hash> {
    let (override_daa, params) = fork_params();
    let active = match override_daa {
        Some(d) => daa_score >= d,
        None => params.security_fork_activation.is_active(daa_score),
    };
    active.then_some(params.genesis.hash)
}

/// The value a parent coinbase commits to for `fc_block`, which is also the key pending work is
/// stashed under. Legacy: the block hash `H_fc`. Bound: the genesis-bound commitment over the
/// header hash with the nonce zeroed — the nonce is set only after the parent is solved (it must
/// equal the parent's hash), so the final block hash does not exist yet at template time.
pub fn merge_key(fc_block: &Block) -> Hash {
    merge_key_with(fc_block, bound_form(fc_block.header.daa_score))
}

fn merge_key_with(fc_block: &Block, bound: Option<Hash>) -> Hash {
    match bound {
        None => fc_block.header.hash,
        Some(genesis) => AuxPow::bound_commitment(
            genesis,
            hashing::header::hash_override_nonce_time(&fc_block.header, 0, fc_block.header.timestamp),
        ),
    }
}

/// The extra data a parent coinbase carries for `fc_block`: `prefix || ZKMM || hex(H_fc)` in the
/// legacy form, `prefix || ZKM1 || hex(commitment)` in the bound form. Exactly one tag, never
/// both: the bound rule refuses a parent that also carries `ZKMM`.
pub fn merge_extra_data(prefix: &[u8], fc_block: &Block) -> Vec<u8> {
    merge_extra_data_with(prefix, fc_block, bound_form(fc_block.header.daa_score))
}

fn merge_extra_data_with(prefix: &[u8], fc_block: &Block, bound: Option<Hash>) -> Vec<u8> {
    let key = merge_key_with(fc_block, bound);
    match bound {
        None => AuxPow::embed_commitment(prefix, key, &[]),
        Some(_) => AuxPow::embed_commitment_v1(prefix, key, &[]),
    }
}

/// The ZKas block hash that lands on chain for `fc_block` solved by `parent_block`. Legacy: the
/// template's own hash (the aux rides outside the header hash). Bound: the hash after the nonce is
/// set from the SOLVED parent — an unsolved parent names a block that never exists.
pub fn chain_hash(parent_block: &Block, fc_block: &Block) -> Hash {
    chain_hash_with(parent_block, fc_block, bound_form(fc_block.header.daa_score))
}

fn chain_hash_with(parent_block: &Block, fc_block: &Block, bound: Option<Hash>) -> Hash {
    match bound {
        None => fc_block.header.hash,
        Some(_) => {
            let mut h = (*fc_block.header).clone();
            h.nonce = AuxPow::nonce_binding(&parent_block.header);
            h.finalize();
            h.hash
        }
    }
}

/// Build the parent block an ASIC hashes in merged mode: one coinbase committing to
/// the merge key (see [`merge_key`]), with the ZKas target. Returns `(parent_block, key)`.
pub fn build_parent_block(fc_block: &Block) -> (Block, Hash) {
    build_parent_block_with(fc_block, bound_form(fc_block.header.daa_score))
}

fn build_parent_block_with(fc_block: &Block, bound: Option<Hash>) -> (Block, Hash) {
    let key = merge_key_with(fc_block, bound);
    let coinbase = Transaction::new(0, vec![], vec![], 0, SUBNETWORK_ID_COINBASE, 0, merge_extra_data_with(&[], fc_block, bound));
    let hash_merkle_root = calc_hash_merkle_root(std::iter::once(&coinbase));

    let mut parent = Header::from_precomputed_hash(ZERO_HASH, vec![Hash::from_u64_word(0xF12E_CA54)]);
    parent.hash_merkle_root = hash_merkle_root;
    parent.bits = fc_block.header.bits; // ASIC grinds against the ZKas target
    parent.timestamp = fc_block.header.timestamp;
    parent.finalize();

    (Block::new(parent, vec![coinbase]), key)
}

/// The merge key a parent block commits to (from its coinbase), under either tag: `ZKMM` (the
/// block hash, legacy) or `ZKM1` (the bound commitment, from the security fork). `None` if it
/// carries no valid commitment (i.e. this wasn't a merged-mining parent). This is the stash key,
/// NOT necessarily a block hash — see [`chain_hash`].
pub fn committed_h_fc(parent_block: &Block) -> Option<Hash> {
    let coinbase = parent_block.transactions.first()?.clone();
    let aux = AuxPow { parent_header: (*parent_block.header).clone(), parent_coinbase: coinbase, coinbase_merkle_branch: vec![] };
    aux.committed_hash().or_else(|| aux.committed_hash_v1())
}

/// The legacy (`ZKMM`) commitment alone, which is the ZKas block hash itself.
pub fn committed_h_fc_legacy(parent_block: &Block) -> Option<Hash> {
    let coinbase = parent_block.transactions.first()?.clone();
    AuxPow { parent_header: (*parent_block.header).clone(), parent_coinbase: coinbase, coinbase_merkle_branch: vec![] }.committed_hash()
}

/// Assemble the ZKas block carrying the AuxPoW proof, from the solved `parent_block`
/// (the bridge has set the winning nonce on its header) and the stashed `fc_block`.
///
/// Builds the *real* coinbase Merkle branch from the parent's transactions, so this is
/// correct for real multi-tx Kaspa parents (not just the single-tx synthetic case).
///
/// Bound form: the block's nonce is set to name its parent and the header is re-finalized, so
/// the returned block's `header.hash` differs from the template's. That returned hash is the one
/// on chain; every log line, metric and color check must use it.
pub fn assemble_aux_block(parent_block: &Block, fc_block: &Block) -> Block {
    assemble_aux_block_with(parent_block, fc_block, bound_form(fc_block.header.daa_score))
}

fn assemble_aux_block_with(parent_block: &Block, fc_block: &Block, bound: Option<Hash>) -> Block {
    let coinbase = parent_block.transactions[0].clone();
    let branch = coinbase_merkle_branch(&parent_block.transactions);
    let aux = AuxPow { parent_header: (*parent_block.header).clone(), parent_coinbase: coinbase, coinbase_merkle_branch: branch };
    let mut header = (*fc_block.header).clone();
    if bound.is_some() {
        header.nonce = AuxPow::nonce_binding(&parent_block.header);
        header.finalize();
    }
    let fc_header = header.with_aux_pow(aux);
    Block::new(fc_header, (*fc_block.transactions).clone())
}

/// A small bounded map from the merge key (see [`merge_key`]) to the ZKas block awaiting a solved parent.
/// Bounded FIFO so a busy pool that never solves a given template doesn't grow it
/// without limit.
pub struct MergedPending {
    map: HashMap<Hash, Block>,
    order: VecDeque<Hash>,
    solved: HashSet<Hash>,
    cap: usize,
}

impl MergedPending {
    pub fn new(cap: usize) -> Self {
        Self { map: HashMap::new(), order: VecDeque::new(), solved: HashSet::new(), cap: cap.max(1) }
    }

    pub fn insert(&mut self, h_fc: Hash, fc_block: Block) {
        if self.map.insert(h_fc, fc_block).is_none() {
            self.order.push_back(h_fc);
            while self.order.len() > self.cap {
                if let Some(old) = self.order.pop_front() {
                    self.map.remove(&old);
                    self.solved.remove(&old);
                }
            }
        }
    }

    pub fn get(&self, h_fc: &Hash) -> Option<Block> {
        self.map.get(h_fc).cloned()
    }

    pub fn claim_solution(&mut self, h_fc: Hash) -> bool {
        self.map.contains_key(&h_fc) && self.solved.insert(h_fc)
    }

    pub fn is_unsolved(&self, h_fc: &Hash) -> bool {
        self.map.contains_key(h_fc) && !self.solved.contains(h_fc)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    fn coinbase(h_fc: Hash) -> Transaction {
        Transaction::new(0, vec![], vec![], 0, SUBNETWORK_ID_COINBASE, 0, AuxPow::embed_commitment(&[1, 2, 3], h_fc, &[9]))
    }
    fn tx(tag: u8) -> Transaction {
        use kaspa_consensus_core::subnets::SUBNETWORK_ID_NATIVE;
        Transaction::new(0, vec![], vec![], 0, SUBNETWORK_ID_NATIVE, 0, vec![tag; 16])
    }

    const EASY_BITS: u32 = 0x207f_ffff; // about half of all nonces pass

    fn fc_block(seed: u64, daa_score: u64) -> Block {
        let header = Header::new_finalized(
            1,
            vec![vec![Hash::from_u64_word(seed)]].try_into().unwrap(),
            Hash::from_u64_word(seed.wrapping_mul(3)),
            ZERO_HASH,
            ZERO_HASH,
            1_700_000_000_000 + seed,
            EASY_BITS,
            0,
            daa_score,
            0.into(),
            0,
            ZERO_HASH,
        );
        Block::new(header, vec![])
    }

    fn windows(payload: &[u8], magic: &[u8; 4]) -> usize {
        payload.windows(4).filter(|w| *w == magic).count()
    }

    /// The full post-fork round trip the bridge performs, judged by the node's own verifier: key the
    /// work by the bound commitment, build a parent committing it under `ZKM1`, solve the parent,
    /// read the key back from the solved parent, assemble with the parent-derived nonce. The
    /// assembled block must pass `AuxRule::Bound`, its hash must be what `chain_hash` reports, and
    /// the same work assembled the legacy way must be refused. Adapted from
    /// firecash/solo-dual-mode v1.0.12 `security_fork_round_trip_passes_the_node_verifier`.
    #[test]
    fn security_fork_round_trip_passes_the_node_verifier() {
        use kaspa_pow::auxpow::{AuxRule, check_pow_gated};
        let genesis = Hash::from_u64_word(0x6e6e);
        let mut exercised = 0;
        for seed in 1..64u64 {
            let fc = fc_block(seed, 1_000);
            let (mut parent, key) = build_parent_block_with(&fc, Some(genesis));
            assert_ne!(key, fc.header.hash, "post-fork work is keyed by the nonce-free commitment");
            assert_eq!(committed_h_fc(&parent), Some(key), "the solved parent maps back to the stashed work");
            assert_eq!(committed_h_fc_legacy(&parent), None, "a bound parent names no block hash on its own");

            // Solve the parent against the ZKas target, as the ASIC would.
            let mut h = (*parent.header).clone();
            let state = kaspa_pow::State::new(&h);
            h.nonce = (0u64..).find(|n| state.check_pow(*n).0).unwrap();
            h.finalize();
            parent = Block::new(h, (*parent.transactions).clone());

            // A solved parent with a DIFFERENT nonce is a different ZKas block (one template, many blocks).
            let mut other = (*parent.header).clone();
            other.nonce ^= 1;
            other.finalize();
            let other = Block::new(other, (*parent.transactions).clone());
            assert_ne!(chain_hash_with(&parent, &fc, Some(genesis)), chain_hash_with(&other, &fc, Some(genesis)));

            let block = assemble_aux_block_with(&parent, &fc, Some(genesis));
            assert_eq!(block.header.nonce, AuxPow::nonce_binding(&parent.header));
            assert_eq!(block.header.hash, chain_hash_with(&parent, &fc, Some(genesis)), "chain_hash names the assembled block");
            assert_ne!(block.header.hash, fc.header.hash, "the on-chain hash is not the template hash after the fork");
            if kaspa_pow::State::new(&block.header).check_pow(block.header.nonce).0 {
                continue; // natively valid by chance; pick a block that needs the witness
            }
            let aux = block.header.aux_pow.as_ref().expect("assembled block carries its witness");
            assert!(check_pow_gated(&block.header, Some(aux), AuxRule::Bound(genesis)).0, "bridge output passes the bound rule");
            assert!(!check_pow_gated(&block.header, Some(aux), AuxRule::Bound(Hash::from_u64_word(1))).0, "bound to its own chain");

            // The same work assembled the legacy way is refused under the bound rule.
            let (legacy_parent, legacy_key) = build_parent_block_with(&fc, None);
            assert_eq!(legacy_key, fc.header.hash, "pre-fork the key is the hash");
            let legacy = assemble_aux_block_with(&legacy_parent, &fc, None);
            assert_eq!(legacy.header.hash, fc.header.hash, "pre-fork the template hash is the chain hash");
            if !kaspa_pow::State::new(&legacy.header).check_pow(legacy.header.nonce).0 {
                let legacy_aux = legacy.header.aux_pow.as_ref().unwrap();
                assert!(!check_pow_gated(&legacy.header, Some(legacy_aux), AuxRule::Bound(genesis)).0, "legacy witness refused");
            }
            exercised += 1;
        }
        assert!(exercised > 10, "the witness path ran ({exercised} blocks)");
    }

    /// Straddle: the form is chosen by the template's own DAA score at the MAINNET activation
    /// (read from the params this binary was built with, never typed here), one block either side.
    /// A pre-fork template keeps `ZKMM` and its hash as key; the first post-fork template carries
    /// `ZKM1` only and is keyed by the bound commitment to mainnet genesis.
    #[test]
    fn form_switches_per_template_at_the_mainnet_activation_score() {
        use kaspa_consensus_core::auxpow::{MERGE_MINE_MAGIC, MERGE_MINE_MAGIC_V1};
        use kaspa_consensus_core::config::params::MAINNET_PARAMS;
        let f = MAINNET_PARAMS.security_fork_activation.daa_score();
        let genesis = MAINNET_PARAMS.genesis.hash;
        assert!(f > 0 && f < u64::MAX, "this build carries a mainnet security-fork score");
        assert_eq!(bound_form(f - 1), None);
        assert_eq!(bound_form(f), Some(genesis));

        let prefix = b"RK-Stratum/x";
        let before = fc_block(7, f - 1);
        let data = merge_extra_data(prefix, &before);
        assert_eq!((windows(&data, &MERGE_MINE_MAGIC), windows(&data, &MERGE_MINE_MAGIC_V1)), (1, 0), "pre-fork: ZKMM only");
        assert_eq!(merge_key(&before), before.header.hash);

        let after = fc_block(7, f);
        let data = merge_extra_data(prefix, &after);
        assert_eq!((windows(&data, &MERGE_MINE_MAGIC), windows(&data, &MERGE_MINE_MAGIC_V1)), (0, 1), "post-fork: ZKM1 only");
        let nonce_free = hashing::header::hash_override_nonce_time(&after.header, 0, after.header.timestamp);
        assert_eq!(merge_key(&after), AuxPow::bound_commitment(genesis, nonce_free));
        assert!(data.starts_with(prefix), "the pool tag stays in front of the commitment");

        // The verifier's own extraction reads the key back out of production framing, and the
        // legacy scanner sees nothing (the bound rule requires that).
        let coinbase = Transaction::new(0, vec![], vec![], 0, SUBNETWORK_ID_COINBASE, 0, data);
        let aux = AuxPow { parent_header: Header::from_precomputed_hash(ZERO_HASH, vec![]), parent_coinbase: coinbase, coinbase_merkle_branch: vec![] };
        assert_eq!(aux.committed_hash_v1(), Some(merge_key(&after)));
        assert_eq!(aux.committed_hash(), None);
    }

    #[test]
    fn h_fc_solution_can_be_claimed_only_once() {
        let block = Block::new(Header::from_precomputed_hash(Hash::from_bytes([7; 32]), vec![]), vec![]);
        let h_fc = block.header.hash;
        let mut pending = MergedPending::new(4);
        pending.insert(h_fc, block);

        assert!(pending.is_unsolved(&h_fc));
        assert!(pending.claim_solution(h_fc));
        assert!(!pending.is_unsolved(&h_fc));
        assert!(!pending.claim_solution(h_fc));
    }

    #[test]
    fn evicting_pending_work_also_evicts_solved_marker() {
        let first = Block::new(Header::from_precomputed_hash(Hash::from_bytes([1; 32]), vec![]), vec![]);
        let second = Block::new(Header::from_precomputed_hash(Hash::from_bytes([2; 32]), vec![]), vec![]);
        let first_hash = first.header.hash;
        let second_hash = second.header.hash;
        let mut pending = MergedPending::new(1);
        pending.insert(first_hash, first);
        assert!(pending.claim_solution(first_hash));
        pending.insert(second_hash, second);

        assert!(!pending.is_unsolved(&first_hash));
        assert!(!pending.claim_solution(first_hash));
        assert!(pending.is_unsolved(&second_hash));
    }

    /// For every parent size 1..=9, the branch we build must fold the coinbase back to
    /// the canonical `calc_hash_merkle_root`, i.e. `verify_coinbase_inclusion` passes —
    /// proving the branch matches Kaspa's real tx Merkle tree, not just the 1-tx case.
    #[test]
    fn coinbase_branch_matches_real_merkle_root_for_all_sizes() {
        let h_fc = Hash::from_bytes([0x5Au8; 32]);
        for n in 1..=9usize {
            let mut txs = vec![coinbase(h_fc)];
            for i in 1..n {
                txs.push(tx(i as u8));
            }
            let root = calc_hash_merkle_root(txs.iter());
            let mut header = Header::from_precomputed_hash(ZERO_HASH, vec![]);
            header.hash_merkle_root = root;
            let aux = AuxPow {
                parent_header: header,
                parent_coinbase: txs[0].clone(),
                coinbase_merkle_branch: coinbase_merkle_branch(&txs),
            };
            assert!(aux.verify_coinbase_inclusion(), "branch must reproduce the root for n={n} txs");
            assert!(aux.verify_binding(h_fc), "full binding (commitment + inclusion) must hold for n={n}");
        }
    }
}
