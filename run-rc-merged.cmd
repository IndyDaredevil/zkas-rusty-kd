@echo off
REM RC v2.1 merged-mining launcher — bakes in everything the PowerShell
REM window kept forgetting. Lives in the repo root; always launches the
REM exe from this repo's own target\release, never a stale PATH copy.
cd /d %~dp0

REM Both REQUIRED for merged mining — either one missing = plain KAS-only
REM mode (fail-safe, but it just cost ~53.8 ZKAS on a full-clear block).
set ZKAS_MERGED_NODE=127.0.0.1:16810
set ZKAS_TREASURY_ADDRESS=zkas:px7ggt9l6kh45k2nffc63mpclvz92mln4z6cvt2dcnewxa7c8950dgtl8lhyk3nqvdqyw8qc5r3fxrn

set RUST_LOG=info

echo.
echo === RC merged launcher: verify BOTH "MERGED MINING ENABLED" lines below ===
echo.
target\release\stratum-bridge.exe --config rc-v2-smoke.yaml --node-mode external
