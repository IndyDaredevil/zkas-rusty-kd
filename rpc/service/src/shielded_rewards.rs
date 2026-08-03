//! Matching coinbase payments to shielded recipients — the pure half of
//! `GetShieldedCoinbaseRewards`.
//!
//! ## Why this RPC exists
//!
//! On a BlockDAG a block's own coinbase pays the miners of its **mergeset** (the
//! blocks it merges), never its own finder, and only a *selected-chain* block's
//! coinbase is ever applied — a merged block's coinbase transaction is inert. A
//! pool that credits miners by summing the outputs of a block it submitted is
//! therefore counting money it will not receive, at a ratio set by how many
//! blocks the chain merges per chain block (measured ~3 on mainnet).
//!
//! The fix is to credit *income*: what the chain actually paid, walking selected
//! chain blocks. This module is that match, factored out so the rule below is
//! pinned by tests.
//!
//! ## The two rules that are easy to get wrong
//!
//! 1. **Match on the 43-byte prefix, not on script equality.** Consensus derives
//!    a coinbase note from `script[..43]` (`build_coinbase_mint`), so a script
//!    with trailing bytes still pays the recipient. Exact equality would silently
//!    miss a payment consensus did make.
//! 2. **One coinbase can pay the same recipient more than once, and every one of
//!    those payments is real.** Two of a pool's blocks in one mergeset produce two
//!    outputs to its address. Deduplicating them (a natural-looking "guard" against
//!    double-crediting) under-credits miners exactly when the pool is doing well.
//!    Payments are identified by `(coinbase_txid, output_index)` and by nothing else.

/// Length of a raw Orchard recipient — the prefix of a shielded coinbase output's
/// script that consensus turns into a note (`build_coinbase_mint`).
pub const RECIPIENT_LEN: usize = 43;

/// One coinbase output paying a requested recipient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedReward {
    /// Position in the coinbase transaction; with the txid this is the payment's identity.
    pub output_index: u32,
    /// The matched 43-byte recipient (echoed for multi-recipient attribution).
    pub recipient: Vec<u8>,
    pub value: u64,
}

/// Every coinbase output of one chain block that pays one of `recipients`, in
/// output order. See the module docs for the prefix rule and the (deliberate)
/// absence of deduplication.
#[must_use]
pub fn match_coinbase_outputs(outputs: &[(Vec<u8>, u64)], recipients: &[Vec<u8>]) -> Vec<MatchedReward> {
    let mut matched = Vec::new();
    for (index, (script, value)) in outputs.iter().enumerate() {
        if script.len() < RECIPIENT_LEN {
            // Not a shielded coinbase output; consensus would have rejected the
            // block, so this can only be a transparent-coinbase network.
            continue;
        }
        let prefix = &script[..RECIPIENT_LEN];
        if let Some(recipient) = recipients.iter().find(|r| r.as_slice() == prefix) {
            matched.push(MatchedReward { output_index: index as u32, recipient: recipient.clone(), value: *value });
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipient(tag: u8) -> Vec<u8> {
        vec![tag; RECIPIENT_LEN]
    }

    #[test]
    fn matches_a_bare_43_byte_recipient() {
        let outs = vec![(recipient(1), 57_00000000u64)];
        let got = match_coinbase_outputs(&outs, &[recipient(1)]);
        assert_eq!(got, vec![MatchedReward { output_index: 0, recipient: recipient(1), value: 57_00000000 }]);
    }

    /// THE regression test for the pool-accounting bug class: a chain block whose
    /// mergeset contains two of the caller's blocks pays the caller twice, and both
    /// payments are real money. Anything that "dedupes" them under-credits miners.
    #[test]
    fn the_same_recipient_paid_twice_in_one_coinbase_yields_two_rewards() {
        let outs = vec![(recipient(1), 57_00000000u64), (recipient(2), 57_00000000), (recipient(1), 57_00000000)];
        let got = match_coinbase_outputs(&outs, &[recipient(1)]);
        assert_eq!(got.len(), 2, "both payments to the recipient must be reported");
        assert_eq!(got[0].output_index, 0);
        assert_eq!(got[1].output_index, 2, "identity is the output index, not the value");
        assert_eq!(got.iter().map(|m| m.value).sum::<u64>(), 114_00000000);
    }

    /// Consensus reads `script[..43]`, so trailing bytes do not stop a payment from
    /// being made — and must not stop it from being reported.
    #[test]
    fn matches_a_script_with_trailing_bytes_by_prefix() {
        let mut script = recipient(7);
        script.extend_from_slice(&[0xaa, 0xbb]);
        let got = match_coinbase_outputs(&[(script, 42)], &[recipient(7)]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].recipient, recipient(7), "the echoed recipient is the 43-byte prefix, not the script");
    }

    #[test]
    fn a_script_shorter_than_a_recipient_never_matches() {
        let short = vec![9u8; RECIPIENT_LEN - 1];
        assert!(match_coinbase_outputs(&[(short, 1)], &[recipient(9)]).is_empty());
    }

    #[test]
    fn unrelated_recipients_are_not_matched() {
        let outs = vec![(recipient(1), 10), (recipient(2), 20)];
        assert!(match_coinbase_outputs(&outs, &[recipient(3)]).is_empty());
    }

    #[test]
    fn multiple_recipients_are_attributed_to_the_right_one() {
        let outs = vec![(recipient(1), 10), (recipient(2), 20), (recipient(3), 30)];
        let got = match_coinbase_outputs(&outs, &[recipient(3), recipient(1)]);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], MatchedReward { output_index: 0, recipient: recipient(1), value: 10 });
        assert_eq!(got[1], MatchedReward { output_index: 2, recipient: recipient(3), value: 30 });
    }

    /// The dev-fee output is the last output of every coinbase that rewards a
    /// subsidy. It is an ordinary payment to an ordinary recipient: a caller that
    /// asks for it gets it, a caller that doesn't, doesn't.
    #[test]
    fn the_dev_fee_output_is_matched_only_when_asked_for() {
        let outs = vec![(recipient(1), 57_00000000u64), (recipient(0xde), 3_00000000)];
        assert_eq!(match_coinbase_outputs(&outs, &[recipient(1)]).len(), 1);
        let dev = match_coinbase_outputs(&outs, &[recipient(0xde)]);
        assert_eq!(dev, vec![MatchedReward { output_index: 1, recipient: recipient(0xde), value: 3_00000000 }]);
    }

    #[test]
    fn no_recipients_matches_nothing() {
        assert!(match_coinbase_outputs(&[(recipient(1), 10)], &[]).is_empty());
    }
}
