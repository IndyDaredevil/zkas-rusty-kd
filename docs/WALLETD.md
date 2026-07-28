# `zkas-walletd` — shielded wallet daemon

A REST daemon that turns the ZKas shielded pool into an ordinary-looking wallet API.
It scans the chain, recognises notes belonging to the keys it holds, keeps the Merkle
witnesses a shielded spend needs, and builds/proves/submits payments.

It backs three very different consumers, and the distinction matters for custody:

| Consumer | Endpoints | Who holds the spend key |
| --- | --- | --- |
| Web / mobile wallet (`wallet.zkas.info`) | `prepare` → `submit` | **the device** — daemon has the viewing key only |
| Desktop wallet | embedded in-process (`lib.rs`) | the app |
| Mining pool / payout service | `send`, `send_many`, `consolidate` | **the daemon** (custodial) |

`main.rs` is only flag parsing; the daemon is a library (`lib.rs`) so the desktop
wallet can embed it.

---

## 1. Running it

```bash
./zkas-walletd \
  --network mainnet \
  --rpc-server 127.0.0.1:16110 \
  --listen 127.0.0.1:8501 \
  --wallet-dir /var/lib/zkas/wallets \
  --wallet-secret "$ZKAS_WALLET_SECRET"
```

| Flag | Default | Notes |
| --- | --- | --- |
| `-s, --rpc-server` | `127.0.0.1:16810` | Node gRPC. **Point at a local node** — a tunnel to a remote node was a large, measurable slowdown. |
| `-l, --listen` | `127.0.0.1:8501` | Loopback by default, on purpose. |
| `--wallet-dir` | `~/.ZKas/wallets` | One `<token>.scan` per wallet. |
| `--network` | `mainnet` | `mainnet` \| `testnet` \| `devnet` \| `simnet` |
| `--wallet-secret` | *(none)* | Encrypts seeds at rest (XChaCha20-Poly1305 + Argon2). Also `ZKAS_WALLET_SECRET`. **Without it seeds are stored in plaintext** (0600) and startup warns. |
| `--allow-origin` | *(none — same-origin only)* | Repeatable CORS allow-list. With none set, cross-origin browser calls are refused. This is what stops any page a user visits from reaching a daemon on their machine. |
| `--allow-default-token` | `false` | Allows the tokenless "default" wallet. Only for a trusted single-user localhost. |
| `--allow-remote` | `false` | Bind a non-loopback address directly. Prefer a TLS proxy. |
| `--serve-public ADDR:PORT` | — | Self-hosting mode: auto-provisioned TLS + a pairing QR for the mobile wallet. No proxy, domain, or certbot. Implies a bearer token. |
| `--insecure` | `false` | With `--serve-public`, plaintext HTTP. **Only** behind a VPN — otherwise viewing keys and balances cross the wire in clear. |
| `--public-host`, `--api-token` | — | With `--serve-public`: host baked into the pairing URI / TLS SAN, and a fixed token. |
| `--diagnose` | `false` | Offline: print each wallet's note/base/**stranded-note** report, then exit. Run with the daemon stopped. |
| `--graft TOKEN:/path/older.scan` | — | Offline: repair a stranded wallet from an older snapshot of itself. Daemon stopped. |

The runtime deliberately oversubscribes worker threads (2× cores): the sync loop is
CPU-bound (trial decryption, witness work), and with only `ncpu` workers a mass rescan
pinned every one of them and starved HTTP — `/api/status` timed out at 15 s during a
170-wallet rescan.

### Reverse proxy

Proving takes tens of seconds. Set the read timeout accordingly or clients will appear
to hang on a send that is actually progressing:

```nginx
location /daemon/ {
    proxy_pass http://127.0.0.1:8501/;
    proxy_read_timeout 300s;
    proxy_connect_timeout 5s;
}
```

---

## 2. Authentication

Every request carries `X-Wallet-Token`, which selects the wallet. Mint one once and
keep it — **it is the credential**; whoever holds it controls that wallet.

```bash
TOK=$(head -c16 /dev/urandom | xxd -p)
curl -X POST -H "X-Wallet-Token: $TOK" http://127.0.0.1:8501/api/wallet/create
```

---

## 3. Endpoints

### Wallet lifecycle

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/api/wallet/create` | New wallet. **Returns the seed once** — store it. |
| `POST` | `/api/wallet/import` | Import a seed. Accepts a `birthday` (DAA or date) to skip replaying earlier chain. |
| `POST` | `/api/wallet/watch` | Register **watch-only** from a full viewing key. Cannot spend. |
| `GET` | `/api/wallet/address` | Shielded `zkas:` receive address. |
| `GET` | `/api/wallet/reveal` | Reveal the seed (gated). |
| `GET` | `/api/wallet/balance` | Balance + sync status (see §5). |
| `GET` | `/api/wallet/history` | Chain-derived history. Opt-in — see `settings`. |
| `POST` | `/api/wallet/settings` | Toggle recoverable history (OVK) etc. |
| `POST` | `/api/wallet/rescan` | Retire the checkpoint and rescan from birthday. |

### Paying (custodial — daemon holds the seed)

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/api/wallet/send` | Pay **one** recipient. Splits into several transactions if it needs more than `max_spends_per_tx` (38) notes. |
| `POST` | `/api/wallet/send_many` | Pay **many** recipients per transaction. **Use this for payouts** — see §6. |
| `POST` | `/api/wallet/consolidate` | Merge notes into one, paid to yourself. `{"heal":true}` selects *oldest* rather than smallest. |

### Paying (non-custodial — device holds the seed)

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/api/wallet/prepare` | Daemon builds + **proves** the bundle with the viewing key only, returns a sighash plus one randomizer per real spend. It cannot authorize the spend. |
| `POST` | `/api/wallet/submit` | Device returns spend-auth signatures; daemon finalizes and submits. |

`ask` (spend authority) is not derivable from the viewing key, so a proven bundle is
worthless until the seed-holder signs it. See `docs/NON_CUSTODIAL_WALLET.md`.

### Other

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/api/wallet/sign` | Prove address control without spending. |
| `POST` | `/api/verify` | Verify such a signature. |
| `GET` | `/api/status` | Daemon/chain status. |

### Examples

```bash
# Receive address
curl -H "X-Wallet-Token: $TOK" http://127.0.0.1:8501/api/wallet/address

# Pay one recipient (amount_fc or amount_sompi; fee optional)
curl -X POST -H "X-Wallet-Token: $TOK" -H 'Content-Type: application/json' \
  -d '{"to":"zkas:<recipient>","amount_fc":5.0,"memo":"invoice 42"}' \
  http://127.0.0.1:8501/api/wallet/send

# Pay many recipients — one proof for the batch
curl -X POST -H "X-Wallet-Token: $TOK" -H 'Content-Type: application/json' \
  -d '{"payees":[
        {"to":"zkas:<miner1>","amount_sompi":1500000000},
        {"to":"zkas:<miner2>","amount_fc":12.5,"memo":"payout"},
        {"to":"zkas:<miner3>","amount_sompi":900000000}]}' \
  http://127.0.0.1:8501/api/wallet/send_many
```

`send_many` returns `{txids, tx_count, payees, paid_sompi, fee_sompi, …}`. Bad address,
zero amount, or insufficient funds fail **before any proving**, in milliseconds.

---

## 4. Performance — what actually costs time

Worth understanding before sizing a deployment, because the intuitive answer is wrong.

**The proof is not the expensive part; the Merkle witness used to be.** The node keeps
only the tree's ~32-node frontier, so by design **wallets hold their own witnesses**.
Producing one meant replaying the note-commitment stream with Sinsemilla, and **a single
Sinsemilla combine costs ~150–260 µs** — a tree replay is exactly one hash per leaf. On
a ~200 K-leaf chain that is ~30–50 s **per send**, and it grows with the chain.

That is now fixed. `WalletDb` retains **complete subtree roots**
(`SubtreeCache`, `shielded-core/src/walletdb.rs`). For a note at position `p` witnessed
at anchor `S`, every authentication sibling is a subtree either entirely below `S`
(complete, therefore immutable forever) or entirely above it (the empty root) — with
exactly **one** exception, the subtree straddling `S`. So all but one sibling is computed
once and kept; the straddler is `O(depth)` per anchor. The roots accumulate in a
Merkle-mountain-range sweep: one combine per leaf amortized, the same per-leaf cost the
tip mirror already paid.

Measured, on the live daemon, same wallet (62 K notes, ~150 K-leaf span):

| Witness build | Before | After |
| --- | --- | --- |
| 1 note | 38.8 s | — |
| 2 notes | 94.7 s | **58.5 ms** |
| 22 notes | — | **243.9 ms** |
| 38 notes | — | **538.8 ms** |

Benchmarked at 200 K leaves: **29.36 s → 0.357 s (82×)** per send.

**This applies to every paying endpoint**, not just the web wallet: `send`,
`send_many`, `consolidate` and `prepare` all witness through the same
`WalletDb::witness_paths_at`. A pool on `/api/wallet/send` gets the identical speedup.

Three independent safety layers, because a wrong witness would be a real problem:

1. A freshly built cache must reproduce the root of the tip mirror — maintained by a
   wholly separate code path — or it is **discarded** and everything keeps replaying.
2. Every path served is still verified against the anchor root before use.
3. A rejected build is remembered, so an `O(chain)` build is never re-paid per tick.

A wrong cache can only *decline to help*. It cannot mis-witness.

### Sizing

- **Cache memory:** ~4 B per leaf of span, but building forces that wallet's decoded
  leaf stream to materialise — together ~**5 MB per 200 K-leaf wallet**. Bounded
  daemon-wide (`SUBTREE_CACHE_TOTAL_SPAN_MAX`, ~300 MB); wallets past the ceiling keep
  the replay path, which is correct, just slower.
- **Only built where it pays:** wallets with a span below `SUBTREE_CACHE_MIN_SPAN`
  (20 000 leaves) skip it — their replay is already fast.
- **One-time build:** 20–60 s of CPU per large wallet, on a blocking thread.
- **Proving is now the dominant cost** and saturates every core it is given: ~3 s for
  1 spend, ~5 s for 2, and tens of seconds for a full 38-spend transaction. **Give the
  daemon cores.** Background cache builds stand down entirely while any payment is
  proving (`PROVING_IN_FLIGHT`) — on a 4-core box, an unthrottled build sweep stretched
  a 38-spend proof from ~40 s to ~92 s.
- **Fewer, larger notes is the remaining lever.** A payment spending 2 notes proves in
  seconds; one spending 38 takes tens of seconds. `consolidate` is how you get there.

---

## 5. Sync model

- Wallets sync in the background; the request path never loads a wallet inline
  (doing so caused a hosted outage — each restart re-stormed).
- `balance` reports sync state; a wallet still catching up says so rather than
  reporting a wrong balance.
- **`import` with a `birthday`** skips replaying chain older than the wallet. This is
  the difference between a ~45-minute restore and ~20 seconds.
- A spend roots at the **matured anchor** (`DEFAULT_ANCHOR_DEPTH + ANCHOR_SLACK` = 630
  blue below the sink), so funds must be ~10 minutes old to be spendable. The anchor
  trails the tip *by design* — this is not a sync deficiency.

---

## 6. Integrating a mining pool or payout service

The single-recipient shape is the wrong one for payouts. Paying N miners with N calls to
`/api/wallet/send` means N bundles: **N Halo 2 proofs, N witness sets, N relay fees,
serially.** That was a measured 45-minute payout run.

An Orchard bundle carries `max(spends, outputs)` actions, so one bundle can hold many
payees plus change. **Use `send_many`:**

```jsonc
{ "payees": [ {"to": "zkas:...", "amount_sompi": 1500000000},
              {"to": "zkas:...", "amount_sompi": 2500000000} ],
  "fee": 3000000 }            // optional floor; raised to the node's byte minimum
```

- Batches are split automatically at `max_payees_per_tx()`.
- Note selection runs **once, under one lock, across all batches**, so two batches can
  never select the same note; all witnesses come from one pass at one anchor.
- Per-transaction fee is priced on the **action** count (`max(spends, payees+1)`),
  because that is what the node charges mass for.

Also worth doing:

- **Consolidate periodically.** A mining treasury accrues one coinbase note per block;
  thousands of small notes mean many spends per payment, and spends are what proving
  costs scale with. `{"heal":true}` merges *oldest*-first, which also rolls the
  compaction base forward.
- **Run the daemon next to your node** (`--rpc-server 127.0.0.1:…`).
- **Serialise your payout calls** or expect them to contend for cores.

---

## 7. Operations

### Log lines worth alerting on

```
prepare: batch-witnessed 38/38 notes in 538.8ms      # witness phase — expect ms
prepare: Halo2 proof took 5.1s                       # proving — expect seconds
subtree cache built in 38.1s (200935 leaves, …)      # one-time, per wallet
subtree cache rejected by its root gate …            # INVESTIGATE: falling back to replay
subtree cache skipped …: daemon-wide cache budget    # raise the budget or add RAM
send: skipping inline witness climb of N leaves      # normal on note-heavy wallets
```

### Troubleshooting

**`HTTP 409 wallet checkpoint is being repaired after a reorg`** — transient; retry.

**`HTTP 409 wallet has not established a matured anchor yet`** — the wallet has not yet
seen a matured anchor. Wait for initial sync. Note this is *not* a tip-proximity check:
a send roots at the matured anchor, which trails the tip anyway, so a wallet slightly
behind the tip can still send.

**`insufficient matured funds`** — likely unmatured notes (~10 min) rather than an empty
wallet. The error reports what is available. If it also hints at stranded notes, run
`--diagnose`.

**Stranded notes** (`no witness path`) — a note below the compaction base. Diagnose with
`--diagnose`; repair with `--graft TOKEN:/path/to/older.scan` against an older snapshot
of the same wallet. Keep `.scan.bak` files.

**A send appears to hang** — check the log for `batch-witnessed` / `Halo2 proof took`.
If proving is running, the payment is progressing; make sure your proxy read timeout is
long enough (§1). Do **not** iteratively restart the daemon to debug — each restart
re-storms sync.

**Never rescan casually.** `rescan` rebuilds from the node's pruning point; against a
*pruned* node that silently loses history. Both production nodes are archival for this
reason.

### Backups

Back up `--wallet-dir` (`<token>.scan` per wallet) **and** the tokens — a token is the
credential. Seeds are only encrypted if `--wallet-secret` is set. For the web/mobile
wallet the seed lives on the device, so the daemon's files are not sufficient to
recover funds — the user's seed phrase is.
