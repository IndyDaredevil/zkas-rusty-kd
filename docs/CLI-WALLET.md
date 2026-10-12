# `shielded-pay` — CLI wallet

Quick offline/CLI operations against a running node. For everyday use prefer the wallet
daemon ([WALLETD.md](WALLETD.md)) or the web/mobile wallet — this tool takes seeds as a
single repeated byte (`[byte; 32]`) for test convenience and exists for testing and
scripted checks, not for holding real balances.

```bash
# Obtain your shielded address (this is what you give a sender or the miner's -a)
./shielded-pay address --seed-byte 1 --network mainnet
# -> zkas:pyfjy228l6gukj2vwztyq6q88eeyggjhvcuzf2jx8u4lvla42d6x0y3dsgp0w...

# Check spendable balance + owned notes (scans the chain via the node RPC)
./shielded-pay balance -s 127.0.0.1:16810 --seed-byte 1

# Send a private payment (amount/fee in sompi; change returns to you)
./shielded-pay send -s 127.0.0.1:16810 --owner-seed-byte 1 \
  --to zkas:<recipient-address> --amount 500000000 --fee 3000000

# Prove you control an address without spending (offline; discloses viewing key)
./shielded-pay sign   --seed-byte 1 --network mainnet --message "gm"
./shielded-pay verify --address zkas:<addr> --message "gm" --signature <hex>
```

## Export a full viewing key without putting a seed in argv

`sign` prints `full-viewing-key || signature` as hexadecimal. The first 192 hexadecimal
characters are the full viewing key (FVK), which can be used by a watch-only wallet to
detect incoming notes but cannot spend them.

For a real seed, use `--seed-stdin` rather than `--seed-hex`: command-line arguments
can be visible to other local processes. This option is deliberately available only to
the offline `sign` command; it is mutually exclusive with `--seed-hex` and
`--seed-byte`, and cannot make an RPC payment.

Run this only on a trusted local machine. It avoids argv, environment, and shell-history
exposure, but cannot protect a compromised machine or privileged local process from
reading memory.

```bash
read -rsp 'Seed (64 hex): ' SEED; echo
SIG=$(printf '%s' "$SEED" | ./shielded-pay sign \
  --seed-stdin --network mainnet --message zkas-watch-export-v1)
unset SEED
FVK=$(printf '%s\n' "$SIG" | awk '/^signature:/ {print substr($2, 1, 192)}')
unset SIG
printf '%s\n' "$FVK"
```
