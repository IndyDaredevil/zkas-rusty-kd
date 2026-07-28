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

## 0. Read this first: payments are sized in **notes**, not coins

Almost every surprise operators hit comes from one fact:

> **A transaction can spend at most 38 notes. That is a count of notes, not an amount
> of ZKAS.** How much value one transaction can move is `38 × your average note size`.

Two wallets holding the same balance behave completely differently:

| Wallet | Avg. note | Max per transaction | Moving 100 000 ZKAS |
| --- | --- | --- | --- |
| Pool treasury, raw coinbase notes | ~57 ZKAS | **~2 166 ZKAS** | 1 754 notes → **47 transactions** |
| Same treasury, consolidated | ~2 166 ZKAS | **~82 300 ZKAS** | 47 notes → **2 transactions** |

And proving cost follows the note count, not the amount:

```
wall time  =  2.4 core-seconds  ×  notes spent  ÷  effective cores
```

So a fragmented wallet is slow *no matter how small the payment*, and consolidating is
the only thing that changes the outcome by an order of magnitude. This is why
`--auto-consolidate` is **on by default** (§4).

### Where 38 comes from

It is a **bytes** limit, not an economic one. Kaspa charges transient mass at 4 per
serialized byte and caps a standard transaction at 500 000 mass, so the bundle must fit
`500 000 / 4 − 256 = 124 744` bytes. Each Orchard action costs:

| Part | Bytes per action |
| --- | --- |
| Action data (`ActionWire`) | 884 |
| Its slice of the Halo2 proof | 2 272 |
| **Total** | **3 156** |

Plus a fixed 117-byte header and a 2 720-byte proof preamble, so
`wire_len(n) = 2 837 + 3 156 n`:

- `n = 38` → 122 765 bytes ✓
- `n = 39` → 125 921 bytes ✗

Hence **38**. Raising it would need a consensus change to the mass cap, and it would buy
nothing anyway — proving cost per spend is flat from 4 to 38, so the same notes cost the
same whether they ride in one transaction or four (§4).

### The three caps, and which one applies

| Cap | Value | Applies to |
| --- | --- | --- |
| `max_spends_per_tx()` | **38** | notes consumed (inputs) |
| `max_actions_per_tx()` | **38** | `max(spends, outputs)` — what mass is charged on |
| `max_payees_per_tx()` | **37** | recipients in one `send_many` (+1 output for change) |

A bundle carries `max(spends, outputs)` actions. So one transaction can spend 38 notes
to pay 1 recipient, **or** spend 2 notes to pay 37 recipients — same mass ceiling, very
different proving cost, because cost tracks *spends*.

### Fees are per-byte, so per-action

Minimum relay fee is charged on size, which means on action count:

| Actions | Min relay fee |
| --- | --- |
| 1–2 | 1 855 400 sompi (0.0186 ZKAS) |
| 38 | 24 578 600 sompi (0.2458 ZKAS) |

The wallet raises any fee you pass to this floor automatically (`chunk_fee`). Passing a
lower `fee` does not save money — it just gets corrected.

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
| `--auto-consolidate MAX_NOTES` | `500` | **On by default.** Keeps custodial wallets under `MAX_NOTES` by merging their oldest notes in the background. Wallets below the ceiling are never touched. See §4. |
| `--no-auto-consolidate` | `false` | Turn background consolidation off entirely. |
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

### Proving: the cost model that now decides everything

With the witness fixed, Halo2 proving is the entire cost of a payment, and it obeys one
measured rule:

> **~2.4 core-seconds of CPU work per note spent — however you schedule it.**

Total CPU is flat whatever the thread count (91.7 s at 1 thread, 93.4 s at 4, for
38 spends), so the only honest formula is:

```
wall time  =  2.4 s  x  notes spent  /  effective cores
```

Measured (`bench_proof_scaling_and_parallelism` in `shielded-core/src/walletdb.rs`, run
with `--ignored`), 4 cores of an EPYC 9654:

| Spends | Wall | CPU | Cores used | s/spend |
| --- | --- | --- | --- | --- |
| 1 | 4.3 s | 10.4 s | 2.42× | (includes one-time setup) |
| 4 | 3.4 s | 10.7 s | 3.14× | 0.85 |
| 12 | 9.6 s | 30.2 s | 3.15× | 0.80 |
| 38 | 30.0 s | 93.8 s | 3.13× | 0.79 |

Consequences, all measured, several counter-intuitive:

- **Raising the per-transaction spend cap wins nothing.** Cost per spend is flat from 4
  to 38 spends, so 38 notes cost the same whether they go in one transaction or four.
- **Throwing more cores at a *single* proof has diminishing returns** — 77 % efficiency
  at 4 threads and falling. Running several proofs at once instead recovers part of that
  (1.21× measured; see "Proving several transactions at once" below), but it is a
  constant factor, not a fix.
- **`RUSTFLAGS=-C target-cpu=native` is worth nothing** — 33.3 s vs 32.3 s baseline at
  38 spends. `pasta_curves` is already hand-optimised; don't re-try this.
- **Release-profile tuning is already done** (`[profile.release.package.*]` for
  `halo2_proofs`, `pasta_curves`, `halo2_gadgets`, `orchard`, `ff`, `group`).

**So there are exactly three levers, because the formula has exactly three terms:**

| Lever | Realistic gain | Cost |
| --- | --- | --- |
| **Fewer, larger notes** (`--auto-consolidate`) | **up to 38×** | free, on by default |
| **More cores** | ~8× going 4 → 64 | hardware |
| **GPU proving** | 3.8–7.2× (published, halo2 on BN254) | months; Orchard is Pasta/IPA, where GPU support barely exists |

This is not academic. A pool treasury takes **one coinbase note per block** — at 1 BPS
that is up to 86 400 notes/day, each worth ~57 ZKAS. Moving 513 484 ZKAS out of such a
wallet requires 9 006 spends, which the 38-spend cap splits into **237 transactions and
~2 hours of proving** — measured on the live pool. Not a bug: it is the note count.
Consolidated to the 500-note ceiling, the same payment is a handful of spends.

### Keeping note count down: `--auto-consolidate` (on by default)

```bash
zkas-walletd ...                          # ceiling 500, already active
zkas-walletd ... --auto-consolidate 200   # tighter
zkas-walletd ... --no-auto-consolidate    # off
```

Merges each over-ceiling custodial wallet's **oldest** notes, up to 38 at a time, one
transaction per minute, **only while nothing is proving**.

Merging does not reduce total proof work — it *relocates* it. Each merge makes one note
worth ~38× more, so the eventual payment spends ~38× fewer notes. Run continuously, the
cost is paid ~30 s at a time in the background and the note count never runs away.
Oldest-first (`heal`) is deliberate: it also lets the fast-sync base roll forward past
the spent notes, shortening every later witness rebuild.

**Why it defaults to on.** The failure it prevents is severe and silent. A wallet that
receives a coinbase note per block crosses into unusability without any symptom until
someone tries to spend — and by then the cure costs as much as the disease. There is no
cheap moment to start except early.

**Why that is safe on a shared daemon.** The ceiling does the work: an ordinary wallet
holds a handful of notes and is **never touched**, so it never pays a fee. Only wallets
far past any normal usage — miners and treasuries, precisely the ones that suffer — are
merged, at roughly **0.05 % of the merged value** in fees.

- **Watch-only wallets are skipped entirely** — the daemon holds no seed and cannot
  spend for them. Only custodial wallets are eligible.
- **It converges and stops.** Each merge removes 37 notes, so once a wallet is under the
  ceiling no further merges happen. There is no perpetual fee drain.
- **Bounded CPU.** One merge (~30 core-seconds) per 60 s cooldown caps background
  merging at about a third of the box, and it stands down completely while any payment
  is proving.
- **Races are closed both ways.** A payment holds `PROVING_IN_FLIGHT` across its whole
  select→submit span, so no merge starts during it; a payment arriving mid-merge waits
  on `CONSOLIDATING` before selecting notes. The two can never pick the same note —
  which would cost one of them a nullifier rejection, never funds.

`POST /api/wallet/consolidate` does exactly one such merge on demand — same code path
(`consolidate_once`), so the manual and automatic paths cannot drift.

### Proving several transactions at once

A chunked payment proves its transactions in groups rather than one at a time, each
proof in its own rayon pool of `PROOF_THREADS_EACH` (2) threads. This exists because a
single proof's parallel efficiency is sublinear, so handing one proof every core wastes
some of them:

| Threads given to one proof | Wall (38 spends) | Efficiency |
| --- | --- | --- |
| 1 | 91.7 s | 100 % |
| 2 | 50.1 s | 91.5 % |
| 3 | 37.6 s | 81 % |
| 4 | 29.7 s | 77 % |

Measured: 2 × 38-spend proofs take **63.9 s sequentially** (each on all 4 cores) vs
**52.9 s grouped** (2 × 2 threads) — **1.21×**, and the gap widens with core count.

A group of one keeps the global pool, so an ordinary single-transaction send is
unchanged and never pays for this. Concurrency is bounded by cores **and** free memory
(`PROOF_MEM_PER_EXTRA_MB`), so a squeezed box degrades to the sequential path rather
than running out of memory.

### Sizing

- **Cache memory:** ~4 B per leaf of span, but building forces that wallet's decoded leaf
  stream to materialise — together ~**11 MB per 200 K-leaf wallet** (32 B/leaf stored +
  32 B/leaf decoded). Gated on live free memory: a build is deferred while
  `MemAvailable` is under `SUBTREE_CACHE_FREE_FLOOR_MB` (1 200 MB) and retried on a later
  pass. This replaced a fixed daemon-wide leaf budget that was both an arbitrary number
  and a *lifetime* counter — once spent, every wallet loaded afterwards was permanently
  stuck on the replay path (observed live: 31 wallets skipped, one then taking 16.1 s to
  witness 3 notes where a cached wallet took 32 ms).
- **Only built where it pays:** wallets with a span below `SUBTREE_CACHE_MIN_SPAN`
  (20 000 leaves) skip it — their replay is already fast.
- **One-time build:** 20–60 s of CPU per large wallet, on a blocking thread.
- **Give the daemon cores.** Proving scales with them at ~3.1/4 efficiency, so on a
  16-core box the same 38-spend transaction proves in roughly a quarter the time. This is
  the cheapest real win available for a payout service.
- Background cache builds stand down entirely while any payment is proving
  (`PROVING_IN_FLIGHT`) — on a 4-core box an unthrottled build sweep stretched a 38-spend
  proof from ~40 s to ~92 s.

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

## 6. Integration playbooks

Two operator shapes dominate and they want opposite things. A **mining pool** has one
fragmenting treasury and pays many recipients at once. An **exchange** has many customer
balances, pays one recipient at a time, and must never be slow at it.

Both are governed by §0: cost tracks **notes spent**, and one transaction spends 38.

### Key material: there are no mnemonics in ZKas

Worth stating plainly before either playbook, because integrators assume otherwise:
**ZKas has no seed phrase / BIP-39 anywhere** — not in `zkas-walletd`, not in
`shielded-core`, not in the SDK, not in the wallet apps. The unit of key material is a
**raw 32-byte seed, hex-encoded**:

```jsonc
POST /api/wallet/create            // daemon generates 32 random bytes (OsRng)
POST /api/wallet/import            // { "seed_hex": "<64 hex chars>", "birthday": 0 }
GET  /api/wallet/reveal            // returns seed_hex — treat as the private key it is
```

You **can** derive that seed from a BIP-39 mnemonic on your own side and post the result
as `seed_hex` — that is a perfectly good way to get a human-transcribable cold backup.
But understand what you are doing: ZKas defines no derivation standard, so the mnemonic
is **your private convention**. The official ZKas wallet cannot restore from it. For an
exchange hot wallet that never needs to be opened in the consumer app, that is fine;
just document your derivation and test the round-trip before you fund anything.

`birthday` is the block height to start scanning from. Set it to the tip when creating a
fresh wallet — omitting it scans the whole chain for funds that do not exist.

---

### 6a. Mining pool / payout service

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

- **Leave `--auto-consolidate` on. This is the single most important thing for a pool.**
  It is the default, so you get it by upgrading; `--no-auto-consolidate` disables it.
  A mining treasury accrues one coinbase note per block, and proving costs a flat
  ~2.4 core-seconds *per note spent* (§4), so a treasury left to fragment turns a payout
  into hundreds of transactions and hours of proving — measured live: 47 000 notes →
  a 237-transaction, ~2-hour payment. Nothing else you do will fix that.
  - Expect a **grind-down period** on an already-fragmented treasury: going from 47 000
    notes to 500 is ~1 250 merges, roughly a day of background work and ~0.3 % of the
    treasury in fees. The benefit arrives long before that finishes — note selection is
    value-descending, so payments start drawing on the merged notes immediately.
- **Run the daemon next to your node** (`--rpc-server 127.0.0.1:…`).
- **Give it cores.** Total proving work is ~2.4 core-seconds per note spent regardless of
  how it is scheduled, so wall time is that divided by cores. After note count, this is
  the only lever that matters.
- **Do not run payout calls concurrently to go faster.** Proving is already parallel; the
  daemon groups transaction proofs itself (see §4). Concurrent calls just multiply memory.

#### Recommended pool configuration

```bash
zkas-walletd \
  --network mainnet \
  --rpc-server 127.0.0.1:16110 \
  --listen 127.0.0.1:8501 \
  --wallet-dir /var/lib/zkas/wallets \
  --wallet-secret "$ZKAS_WALLET_SECRET" \
  --auto-consolidate 300
```

`--auto-consolidate` is already on at 500; 300 is tighter because a treasury churns hard.
Do **not** pass `--no-auto-consolidate`.

#### Worked example: paying 500 miners, 100 000 ZKAS total

The treasury holds raw coinbase notes (~57 ZKAS each), so 100 000 ZKAS needs **1 754
notes** — and that note count, not the amount, decides everything:

| Approach | Transactions | Proving (4 cores) | Fees |
| --- | --- | --- | --- |
| 500 × `send`, one call per miner | 500+ | 500+ proofs, serial | 500 × 0.019–0.246 ZKAS |
| 1 × `send_many`, unconsolidated | **47** (spend-bound: 1 754 ÷ 38) | 1 754 spends ≈ **70 min** | 47 × 0.246 ≈ 11.6 ZKAS |
| 1 × `send_many`, **consolidated** | **14** (payee-bound: 500 ÷ 37) | ~47 spends ≈ **2 min** | 14 × 0.246 ≈ 3.4 ZKAS |

Consolidated, the batch stops being spend-bound and becomes payee-bound — the cheap
direction, since cost tracks spends and not outputs.

```bash
curl -X POST -H "X-Wallet-Token: $TOK" -H 'Content-Type: application/json' \
  -d '{"payees":[
        {"to":"zkas:...miner1...","amount_fc":12.5},
        {"to":"zkas:...miner2...","amount_fc":8.25}
      ]}' \
  http://127.0.0.1:8501/api/wallet/send_many
```

Batches past 37 payees are split for you; the response carries `txids` and `tx_count`.

#### Pool health checks

```bash
# the number that decides your payout time
curl -s -H "X-Wallet-Token: $TOK" http://127.0.0.1:8501/api/status | jq '.notes'

grep "auto-consolidate: merged" walletd.log | tail -5      # merging keeping up?
grep -E "send: (building|tx .* proven)" walletd.log | tail  # payout in progress
```

If `notes` climbs steadily while `auto-consolidate: merged` lines are rare, merging is
being starved — check whether something keeps a payment permanently in flight.

---

### 6b. Exchange / custodial service

#### The deposit-attribution problem — read before designing anything

**One wallet has exactly one address.** `/api/wallet/address` returns
`address_at(0, External)` and always the same string; there is no endpoint that mints a
fresh per-customer address. So there are two workable designs, and one of them does not
scale:

| Design | How | Verdict |
| --- | --- | --- |
| **One wallet per customer** | a token per customer, each its own `.scan` file | **Does not scale.** Every wallet syncs independently; 350 loaded wallets already strain a 4-core box. Fine for hundreds, not for hundreds of thousands. |
| **One deposit wallet + memo** | every customer gets the same address plus a unique memo/payment-id | **Use this.** The XRP/XLM destination-tag pattern. One wallet, one scan. |

Memos survive on the receive side and are readable per transaction:

```bash
# enable the readable record first — off by default
curl -X POST -H "X-Wallet-Token: $TOK" -H 'Content-Type: application/json' \
  -d '{"recoverable_history":true}' http://127.0.0.1:8501/api/wallet/settings

curl -s -H "X-Wallet-Token: $TOK" http://127.0.0.1:8501/api/wallet/history \
  | jq '.[] | select(.kind=="received") | {txid, amount, memo}'
```

Memos are up to 512 bytes and are **encrypted to the recipient** — nobody but your wallet
sees them on-chain. Two caveats: a customer who forgets the memo produces an
unattributable deposit (have a manual reconciliation path, as XRP exchanges do), and
`recoverable_history` must be on or no memo is recorded to read back.

#### Recommended exchange configuration

```bash
zkas-walletd \
  --network mainnet \
  --rpc-server 127.0.0.1:16110 \
  --listen 127.0.0.1:8501 \
  --wallet-dir /var/lib/zkas/wallets \
  --wallet-secret "$ZKAS_WALLET_SECRET" \   # MANDATORY: seeds are plaintext without it
  --auto-consolidate 200
```

Never pass `--allow-default-token` on a custodial daemon: it makes a request carrying no
`X-Wallet-Token` resolve to a shared "default" wallet.

#### Yes, an exchange needs consolidation — more than a pool does

This is the part integrators get wrong. **Every customer deposit is one note.** A hot
wallet taking 10 000 deposits holds 10 000 notes, which is exactly the pool treasury
failure with a different cause: a withdrawal then needs hundreds of spends and minutes of
proving, and it gets worse every day you operate. `--auto-consolidate` is what keeps a
withdrawal at 1–3 spends (~2–7 s) instead of drifting into minutes. Leave it on, and set
the ceiling **lower** than a pool would (200), because withdrawal latency is customer-
visible in a way a payout run is not.

#### Deposits

Poll `/api/status`. Two fields matter and they are not the same:

| Field | Meaning |
| --- | --- |
| `balance` | everything the wallet has seen |
| `spendable` | notes past the maturity anchor — what a withdrawal can actually use |

**Credit on `spendable`, not `balance`.** A note needs ~10 minutes
(`DEFAULT_ANCHOR_DEPTH + ANCHOR_SLACK` blue blocks) before it can be spent. Crediting on
`balance` lets a customer withdraw against value the daemon cannot yet move, and the
withdrawal fails with `insufficient matured funds`.

The mempool loop surfaces an incoming payment within about a second of the sender hitting
send, so a zero-confirmation preview exists — treat it as a UI hint, never as credit.

#### Withdrawals

```bash
curl -X POST -H "X-Wallet-Token: $TOK" -H 'Content-Type: application/json' \
  -d '{"to":"zkas:...customer...","amount_sompi":25000000000,"memo":"withdrawal 88213"}' \
  http://127.0.0.1:8501/api/wallet/send
```

- **`tx_count` may be > 1.** A payment needing more than 38 notes is split across
  transactions; every id is in `txids`. Record all of them, not just `txid`.
- **Handle the partial-failure shape.** If the node rejects a later chunk you get
  HTTP 502 with `{"error": …, "txids": [...], "sent_sompi": N}` — money **did** move.
  Reconcile against `txids`; never assume all-or-nothing.
- **Serialise withdrawals per wallet.** Two concurrent sends on one wallet can select the
  same notes and the second is rejected for reusing a nullifier. Across *different*
  wallets, concurrency is fine.
- **Prefer `amount_sompi`** (integer) to `amount_fc` (float). `amount_sompi_exact` in the
  response is the authoritative string form.

#### Cold storage

Sweep hot → cold with an ordinary `send`. Because cost tracks notes, sweeping a large
balance out of a *fragmented* wallet is the worst case in the entire system — which is
the same reason to keep the hot wallet consolidated. Back the cold seed up as `seed_hex`
(or your own mnemonic encoding of it, per the note above) and verify the restore before
funding.

---

## 7. Operations

### Log lines worth alerting on

```
prepare: batch-witnessed 38/38 notes in 538.8ms      # witness phase — expect ms
prepare: Halo2 proof took 5.1s                       # proving — expect seconds
subtree cache built in 38.1s (200935 leaves, …)      # one-time, per wallet
subtree cache rejected by its root gate …            # INVESTIGATE: falling back to replay
subtree cache deferred …: only N MB free             # add RAM, or accept replay on that wallet
auto-consolidate: merged 38 notes into one …          # healthy; treasury staying compact
auto-consolidate: merge failed …                      # INVESTIGATE: treasury will fragment
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
