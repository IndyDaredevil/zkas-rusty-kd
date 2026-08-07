pub mod app_config;
pub mod client_handler;
pub mod default_client;
pub mod errors;
pub mod hasher;
pub mod jsonrpc_event;
pub mod kaspaapi;
pub mod log_colors;
pub mod merged_obs;
pub mod mining_state;
pub mod net_utils;
pub mod merged;
pub mod notification_hub;
pub mod pow_diagnostic;
pub mod prom;
#[cfg(feature = "rkstratum_cpu_miner")]
pub mod rkstratum_cpu_miner;
pub mod share_handler;
pub mod stratum_context;
pub mod stratum_listener;
pub mod stratum_server;

pub use app_config::{BridgeConfig, InstanceConfig};
pub use client_handler::*;
pub use default_client::*;
pub use errors::*;
pub use hasher::*;
pub use jsonrpc_event::*;
pub use kaspaapi::*;
pub use mining_state::*;
pub use prom::{WorkerContext, *};
#[cfg(feature = "rkstratum_cpu_miner")]
pub use rkstratum_cpu_miner::*;
pub use share_handler::*;
pub use stratum_context::*;
pub use stratum_listener::*;
pub use stratum_server::BridgeConfig as StratumServerBridgeConfig;
pub use stratum_server::*;

/// Windows linker shim for risc0-zkvm-platform (bug ledger #1, second vector).
///
/// The shielded consensus stack (kaspa-consensus-core -> kaspa-shielded-core)
/// pulls in risc0's zkVM platform crate, whose `sys_alloc_words` references
/// the guest-side syscall `sys_alloc_aligned` — a symbol that only exists on
/// the RISC-V zkVM target. Release builds dead-strip the never-called codegen
/// unit via LTO, but debug/test-profile links on Windows resolve eagerly and
/// fail with LNK2019/LNK1120. This host stub satisfies link.exe; it is
/// unreachable in practice (the syscall path only executes inside the zkVM
/// guest) and aborts loudly if that ever stops being true.
#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "C" fn sys_alloc_aligned(_bytes: usize, _align: usize) -> *mut u8 {
    eprintln!("fatal: risc0 guest syscall sys_alloc_aligned invoked on host");
    std::process::abort();
}
