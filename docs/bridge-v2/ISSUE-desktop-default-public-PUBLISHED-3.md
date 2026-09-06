
**Not a re-open of #1; different area (desktop chain-source chooser).**

## Environment
- ZKas Wallet desktop **1.0.32** (release asset `ZKas-Wallet-macos-aarch64.dmg`, sha256 d49597a14c31063dd6596ca84e549485dd58165cf0a52cedaed2f9382c071bce), macOS aarch64. First observed on 1.0.29; reproduced identically on 1.0.32 the same day (lsof output below is from 1.0.32).
- Side note: the in-app badge reads v1.0.32 while ZKas → About reads "Version 1.0.31-1 (1.0.31-1)" — the bundle version string appears not to have been bumped. Separate, minor.
- Wallet created on 1.0.13, upgraded in place to 1.0.29; two legacy hex-key wallets; App Lock on.
- Embedded node updated in-app to zkas-v1.0.8.

## Steps (measured with `lsof -nP -iTCP -a -c zkas -c ZKas | grep -v LISTEN` at each point)
0. App fully closed. lsof: no connections.
1. Launch from the Dock. The Unlock screen appears. **Before the PIN is entered**, lsof already shows two ESTABLISHED connections from the desktop process to the public node:
   `zkas-desk 92877 … 192.168.1.173:56295->185.147.157.125:16110 (ESTABLISHED)`
   `zkas-desk 92877 … 192.168.1.173:56294->185.147.157.125:16110 (ESTABLISHED)`
   No `zkas-node` process; no connection to 127.0.0.1:16810.
2. Enter PIN, Unlock. Wallet view shows a balance. Chooser reads **This Computer — Public Node** (the previous session's choice was "your own node"). Node page: **Stopped**.
3. Open the chain-source chooser (CONNECTION → CHAIN SOURCE). Options: *Public service* (Use) · *This computer · public node* (**Connected**) · *Tor · over the onion* (Use) · *This computer · your own node* (**Set up**, "Managed here · gRPC 127.0.0.1:16810"). The own-node option shows **Set up**, not Use, although it was set up and running in the previous session.
4. Select *This computer · your own node* → the Node tab opens: **Stopped**, card still reading "Mode: Shielded history" from the previous run. Press **Run node** → "Choose how to run it" dialog, **default: Mining** ("does not keep old wallet notes"). Re-pick **Shielded history** (the previous run's mode; its launcher line carried `--shielded-history=on`). Node starts; app log: `[app] launching …/bin/zkas-node --appdir=… --rpclisten=127.0.0.1:16810 --utxoindex --disable-upnp --yes --nologfiles --addpeer=185.147.157.125:16111 --addpeer=160.187.211.153:16111 --listen=127.0.0.1:16811 --shielded-history=on`; IBD completes in seconds (Synced, 10 peers). Steps 3–4 repeat on every launch.

## Observed
- The public node is contacted at process start, pre-unlock, with no user action.
- The saved chain-source choice is not restored; the default "public node" is applied, and the own-node option presents as not set up.
- A node that was running at the previous quit is not started at launch, and its run mode is not remembered: the dialog defaults to Mining, which by its own description cannot serve the wallet's history.
- After step 3, with the local node synced and reachable, the desktop process still holds the two public-node connections alongside its 127.0.0.1:16810 connections (lsof).

## Expected
- The last selected chain source is restored on launch; a node that was running at quit is started again at launch in its previous mode (or the dialog defaults to the previous mode); with "your own node" selected, the engine uses 127.0.0.1:16810 and does not query the public node.
- The 1.0.18/1.0.19 commit messages ("make the choice survive restarts", "stop silently reverting to the local daemon") read as if persistence is intended — on desktop, is the stored choice not read at launch, or is "public node" applied as a default before it is read?

## Why it matters
With "public node" active, chain queries go to a third-party node (IP + sync pattern visible to it; chain view dependent on it) even though a synced local node is available. (Per the 1.0.32 privacy labels this is the *node* case, not the wallet-service case — the viewing key is not involved.) Users who chose their own node for exactly this reason get the public node on every launch without noticing.

## Questions
- Are the chain-source value and the node run-state persisted at all on desktop, or only on web/mobile?
- When both the public node and the local node are connected, which one serves the wallet's queries?

Happy to test a build or bisect between 1.0.19 and 1.0.32.

