@echo off
REM v2.1 RC merged-mining bridge launcher — bakes the env-per-window footgun away.
REM Edit the two ZKAS_ lines once; then double-click or run from any shell.
set ZKAS_MERGED_NODE=127.0.0.1:16810
set ZKAS_TREASURY_ADDRESS=zkas:px7ggt9l6kh45k2nffc63mpclvz92mln4z6cvt2dcnewxa7c8950dgtl8lhyk3nqvdqyw8qc5r3fxrn
set RUST_LOG=info
cd /d "%~dp0"
target\release\stratum-bridge.exe --config rc-v2-smoke.yaml --node-mode external
pause
