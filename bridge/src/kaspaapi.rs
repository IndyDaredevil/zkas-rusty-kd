use crate::log_colors::LogColors;
use crate::share_handler::KaspaApiTrait;
use anyhow::{Context, Result};
use kaspa_addresses::Address;
use kaspa_consensus_core::block::Block;
use kaspa_grpc_client::GrpcClient;
use kaspa_notify::{listener::ListenerId, scope::NewBlockTemplateScope};
use kaspa_rpc_core::notify::mode::NotificationMode;
use kaspa_rpc_core::{
    GetBlockDagInfoRequest, GetBlockTemplateRequest, GetConnectedPeerInfoRequest, GetCurrentBlockColorRequest, GetInfoRequest,
    GetServerInfoRequest, Notification, RpcHash, RpcRawBlock, SubmitBlockRequest, SubmitBlockResponse, api::rpc::RpcApi,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

use crate::notification_hub::{HubScope, NotificationHub, DEFAULT_HUB_CAPACITY};
use tokio::sync::watch;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

const STRATUM_COINBASE_TAG_BYTES: &[u8] = b"RK-Stratum";
const MAX_COINBASE_TAG_SUFFIX_LEN: usize = 64;

fn sanitize_coinbase_tag_suffix(suffix: &str) -> Option<String> {
    let suffix = suffix.trim().trim_start_matches('/');
    if suffix.is_empty() {
        return None;
    }

    let mut out = String::with_capacity(suffix.len().min(MAX_COINBASE_TAG_SUFFIX_LEN));
    for ch in suffix.chars() {
        if out.len() >= MAX_COINBASE_TAG_SUFFIX_LEN {
            break;
        }
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            out.push(ch);
        } else if ch.is_ascii_whitespace() {
            out.push('_');
        }
    }

    let out = out.trim_matches('_').to_string();
    if out.is_empty() { None } else { Some(out) }
}

fn build_coinbase_tag_bytes(suffix: Option<&str>) -> Vec<u8> {
    let mut tag = STRATUM_COINBASE_TAG_BYTES.to_vec();
    if let Some(suffix) = suffix.and_then(sanitize_coinbase_tag_suffix) {
        tag.push(b'/');
        tag.extend_from_slice(suffix.as_bytes());
    }
    tag
}

struct BlockSubmitGuard {
    ttl: Duration,
    max_entries: usize,
    entries: HashMap<String, Instant>,
    order: VecDeque<String>,
}

impl BlockSubmitGuard {
    fn new(ttl: Duration, max_entries: usize) -> Self {
        Self { ttl, max_entries, entries: HashMap::new(), order: VecDeque::new() }
    }

    fn prune(&mut self, now: Instant) {
        while let Some(front) = self.order.front() {
            let remove = match self.entries.get(front) {
                Some(ts) => now.duration_since(*ts) > self.ttl,
                None => true,
            };
            if remove {
                if let Some(key) = self.order.pop_front() {
                    self.entries.remove(&key);
                }
            } else {
                break;
            }
        }

        while self.entries.len() > self.max_entries {
            if let Some(key) = self.order.pop_front() {
                self.entries.remove(&key);
            } else {
                break;
            }
        }
    }

    fn try_mark(&mut self, hash: &str, now: Instant) -> bool {
        self.prune(now);
        if self.entries.contains_key(hash) {
            return false;
        }
        self.entries.insert(hash.to_string(), now);
        self.order.push_back(hash.to_string());
        true
    }

    fn remove(&mut self, hash: &str, now: Instant) {
        self.prune(now);
        self.entries.remove(hash);
    }
}

static BLOCK_SUBMIT_GUARD: Lazy<Mutex<BlockSubmitGuard>> =
    Lazy::new(|| Mutex::new(BlockSubmitGuard::new(Duration::from_secs(600), 50_000)));

#[derive(Clone, Debug, Default)]
pub struct NodeStatusSnapshot {
    pub last_updated: Option<std::time::Instant>,
    pub is_connected: bool,
    pub is_synced: Option<bool>,
    pub network_id: Option<String>,
    pub server_version: Option<String>,
    pub virtual_daa_score: Option<u64>,
    pub block_count: Option<u64>,
    pub header_count: Option<u64>,
    pub difficulty: Option<f64>,
    pub tip_hash: Option<String>,
    pub peers: Option<usize>,
    pub mempool_size: Option<u64>,
}

pub static NODE_STATUS: Lazy<Mutex<NodeStatusSnapshot>> = Lazy::new(|| Mutex::new(NodeStatusSnapshot::default()));

/// Kaspa API client wrapper using RPC client
/// Both use gRPC under the hood, but through an RPC client wrapper abstraction
pub struct KaspaApi {
    client: Arc<GrpcClient>,
    /// Multi-subscriber fan-out of this client's node notifications. Every
    /// stratum instance (and any future consumer, e.g. a fate tracker on
    /// VirtualChainChanged) calls `hub.subscribe(..)` for an independent
    /// stream — replacing the old take()-once mpsc receiver that limited the
    /// process to a single notification consumer and forced main.rs to gate
    /// real notifications to the first instance only.
    hub: Arc<NotificationHub>,
    connected: Arc<Mutex<bool>>,
    coinbase_tag: Vec<u8>,
}

impl KaspaApi {
    /// Create a new Kaspa API client
    pub async fn new(
        address: String,
        coinbase_tag_suffix: Option<String>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<Arc<Self>> {
        info!("Connecting to Kaspa node at {}", address);

        // GrpcClient requires explicit "grpc://" prefix for connection
        // Always add it if not present (avoids unnecessary connection failure)
        let grpc_address = if address.starts_with("grpc://") { address.clone() } else { format!("grpc://{}", address) };

        // Log connection attempt (detailed logs moved to debug)
        debug!("{} {}", LogColors::api("[API]"), LogColors::label("Establishing RPC connection to Kaspa node:"));
        debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Address:"), &grpc_address);
        debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Protocol:"), "gRPC (via RPC client wrapper)");

        let mut attempt: u64 = 0;
        let mut backoff_ms: u64 = 250;

        let client = loop {
            attempt += 1;
            let connect_fut = GrpcClient::connect_with_args(
                NotificationMode::Direct,
                grpc_address.clone(),
                None,
                true,
                None,
                false,
                Some(500_000),
                Default::default(),
            );

            let res = tokio::select! {
                _ = shutdown_rx.wait_for(|v| *v) => {
                    return Err(anyhow::anyhow!("shutdown requested"));
                }
                res = connect_fut => res,
            };

            match res {
                Ok(client) => break Arc::new(client),
                Err(e) => {
                    let backoff = Duration::from_millis(backoff_ms);
                    warn!(
                        "failed to connect to kaspa node at {} (attempt {}): {}, retrying in {:.2}s",
                        grpc_address,
                        attempt,
                        e,
                        backoff.as_secs_f64()
                    );

                    tokio::select! {
                        _ = shutdown_rx.wait_for(|v| *v) => {
                            return Err(anyhow::anyhow!("shutdown requested"));
                        }
                        _ = sleep(backoff) => {}
                    }

                    backoff_ms = (backoff_ms.saturating_mul(2)).min(5_000);
                }
            }
        };

        // Log successful connection (detailed logs moved to debug)
        debug!("{} {}", LogColors::api("[API]"), LogColors::block("RPC Connection Established Successfully"));
        debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Connected to:"), &grpc_address);
        debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Connection Type:"), "gRPC (via RPC client wrapper)");

        // Start the client (no notify needed for Direct mode)
        client.start(None).await;

        // Subscribe to block template notifications
        // Some nodes may take time to accept notification subscriptions; retry until it succeeds.
        // This retry logic with exponential backoff handles transient failures where nodes are not
        // immediately ready to accept subscriptions after connection, preventing tight-looping and log spam.
        let mut attempt: u64 = 0;
        let mut backoff_ms: u64 = 250;
        loop {
            attempt += 1;
            let notify_fut = client.start_notify(ListenerId::default(), NewBlockTemplateScope {}.into());

            let res = tokio::select! {
                _ = shutdown_rx.wait_for(|v| *v) => {
                    return Err(anyhow::anyhow!("shutdown requested"));
                }
                res = notify_fut => res,
            };

            match res {
                Ok(_) => break,
                Err(e) => {
                    let backoff = Duration::from_millis(backoff_ms);
                    warn!(
                        "failed to subscribe to block template notifications (attempt {}): {}, retrying in {:.2}s",
                        attempt,
                        e,
                        backoff.as_secs_f64()
                    );

                    tokio::select! {
                        _ = shutdown_rx.wait_for(|v| *v) => {
                            return Err(anyhow::anyhow!("shutdown requested"));
                        }
                        _ = sleep(backoff) => {}
                    }
                    backoff_ms = (backoff_ms.saturating_mul(2)).min(5_000);
                }
            }
        }

        // Start the notification hub: one relay owns this client's upstream
        // stream and fans it out to any number of subscribers.
        let hub = NotificationHub::start(client.notification_channel_receiver(), "kaspa", DEFAULT_HUB_CAPACITY);

        let coinbase_tag = build_coinbase_tag_bytes(coinbase_tag_suffix.as_deref());
        let api = Arc::new(Self { client, hub, connected: Arc::new(Mutex::new(true)), coinbase_tag });

        // Start network stats thread
        let api_clone = Arc::clone(&api);
        tokio::spawn(async move {
            api_clone.start_stats_thread().await;
        });

        // Start node status polling thread (for console status display)
        let api_clone = Arc::clone(&api);
        tokio::spawn(async move {
            api_clone.start_node_status_thread().await;
        });

        Ok(api)
    }

    /// Start network stats thread
    /// Fetches network stats every 30 seconds and records them in Prometheus
    async fn start_stats_thread(self: Arc<Self>) {
        use crate::prom::record_network_stats;
        use kaspa_rpc_core::{EstimateNetworkHashesPerSecondRequest, GetBlockDagInfoRequest};

        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            // Get block DAG info
            // GetBlockDagInfoRequest is a unit struct, construct directly
            let dag_response = match self.client.get_block_dag_info_call(None, GetBlockDagInfoRequest {}).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("failed to get network hashrate from kaspa, prom stats will be out of date: {}", e);
                    continue;
                }
            };

            // Get tip hash (first one)
            // tip_hashes is Vec<Hash> in the response (already parsed)
            let tip_hash = match dag_response.tip_hashes.first() {
                Some(hash) => Some(*hash), // Clone the Hash
                None => {
                    warn!("no tip hashes available for network hashrate estimation");
                    continue;
                }
            };

            // Estimate network hashes per second
            // new(window_size: u32, start_hash: Option<RpcHash>)
            // RpcHash is the same as Hash, so we can use tip_hash directly
            let hashrate_response = match self
                .client
                .estimate_network_hashes_per_second_call(None, EstimateNetworkHashesPerSecondRequest::new(1000, tip_hash))
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!("failed to get network hashrate from kaspa, prom stats will be out of date: {}", e);
                    continue;
                }
            };

            // Record network stats
            record_network_stats(hashrate_response.network_hashes_per_second, dag_response.block_count, dag_response.difficulty);
        }
    }

    async fn start_node_status_thread(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;

            let connected = self.client.is_connected();

            let server_info_fut = self.client.get_server_info_call(None, GetServerInfoRequest {});
            let dag_info_fut = self.client.get_block_dag_info_call(None, GetBlockDagInfoRequest {});
            let peers_fut = self.client.get_connected_peer_info_call(None, GetConnectedPeerInfoRequest {});
            let info_fut = self.client.get_info_call(None, GetInfoRequest {});

            let (server_info, dag_info, peers_info, info_resp) = tokio::join!(server_info_fut, dag_info_fut, peers_fut, info_fut);

            let mut snapshot = NODE_STATUS.lock();
            snapshot.last_updated = Some(std::time::Instant::now());
            snapshot.is_connected = connected;

            if let Ok(server_info) = server_info {
                snapshot.is_synced = Some(server_info.is_synced);
                snapshot.network_id = Some(format!("{:?}", server_info.network_id));
                snapshot.server_version = Some(server_info.server_version);
                snapshot.virtual_daa_score = Some(server_info.virtual_daa_score);
            }

            if let Ok(dag) = dag_info {
                snapshot.block_count = Some(dag.block_count);
                snapshot.header_count = Some(dag.header_count);
                snapshot.difficulty = Some(dag.difficulty);
                snapshot.tip_hash = dag.tip_hashes.first().map(|h| format!("{}", h));
                if snapshot.virtual_daa_score.is_none() {
                    snapshot.virtual_daa_score = Some(dag.virtual_daa_score);
                }
                if snapshot.network_id.is_none() {
                    snapshot.network_id = Some(format!("{:?}", dag.network));
                }
            }

            if let Ok(peers) = peers_info {
                snapshot.peers = Some(peers.peer_info.len());
            }

            if let Ok(info) = info_resp {
                snapshot.mempool_size = Some(info.mempool_size);
                if snapshot.server_version.is_none() {
                    snapshot.server_version = Some(info.server_version);
                }
            }
        }
    }

    /// Submit a block
    pub async fn submit_block(&self, block: Block) -> Result<SubmitBlockResponse> {
        // Use kaspa_consensus_core::hashing::header::hash() for block hash calculation
        // In Kaspa, the block hash is the header hash (transactions are represented by hash_merkle_root in header)
        use kaspa_consensus_core::hashing::header;
        let block_hash = header::hash(&block.header).to_string();
        let blue_score = block.header.blue_score;
        let timestamp = block.header.timestamp;
        let nonce = block.header.nonce;

        {
            let now = Instant::now();
            let mut guard = BLOCK_SUBMIT_GUARD.lock();
            if !guard.try_mark(&block_hash, now) {
                return Err(anyhow::anyhow!("ErrDuplicateBlock: block already submitted"));
            }
        }

        debug!(
            "{} {}",
            LogColors::api("[API]"),
            LogColors::api(&format!("===== ATTEMPTING BLOCK SUBMISSION TO KASPA NODE ===== Hash: {}", block_hash))
        );
        debug!("{} {}", LogColors::api("[API]"), LogColors::label("Block Details:"));
        debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Hash:"), block_hash);
        debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Blue Score:"), blue_score);
        debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Timestamp:"), timestamp);
        debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Nonce:"), format!("{:x} ({})", nonce, nonce));
        debug!("{} {}", LogColors::api("[API]"), "Converting block to RPC format and sending to node...");

        // Convert Block to RpcRawBlock (use reference)
        let rpc_block: RpcRawBlock = (&block).into();

        // Submit block (don't allow non-DAA blocks)
        debug!("{} {}", LogColors::api("[API]"), "Calling submit_block via RPC client...");
        let result =
            self.client.submit_block_call(None, SubmitBlockRequest::new(rpc_block, false)).await.context("Failed to submit block");

        if let Err(e) = &result {
            let error_str = e.to_string();
            let is_duplicate = error_str.contains("ErrDuplicateBlock") || error_str.contains("duplicate");
            if !is_duplicate {
                let now = Instant::now();
                let mut guard = BLOCK_SUBMIT_GUARD.lock();
                guard.remove(&block_hash, now);
            }
        }

        match &result {
            Ok(response) => {
                // IMPORTANT: The RPC call can succeed while the node still rejects the block.
                // Only treat SubmitBlockReport::Success as accepted.
                if !response.report.is_success() {
                    let now = Instant::now();
                    let mut guard = BLOCK_SUBMIT_GUARD.lock();
                    guard.remove(&block_hash, now);

                    warn!(
                        "{} {}",
                        LogColors::api("[API]"),
                        LogColors::validation(&format!("===== BLOCK REJECTED BY KASPA NODE ===== Hash: {}", block_hash))
                    );
                    warn!(
                        "{} {} {}",
                        LogColors::api("[API]"),
                        LogColors::label("REJECTION REASON:"),
                        format!("{:?}", response.report)
                    );
                    warn!(
                        "{} {} {}",
                        LogColors::api("[API]"),
                        LogColors::label("  - Blue Score:"),
                        format!("{}, Timestamp: {}, Nonce: {:x}", blue_score, timestamp, nonce)
                    );
                    return Err(anyhow::anyhow!("Block rejected by node: {:?}", response.report));
                }

                // Keep block accepted message at info (important operational event)
                info!(
                    "{} {}",
                    LogColors::api("[API]"),
                    LogColors::block(&format!("===== BLOCK ACCEPTED BY KASPA NODE ===== Hash: {}", block_hash))
                );
                // Detailed acceptance logs moved to debug
                debug!(
                    "{} {} {}",
                    LogColors::api("[API]"),
                    LogColors::label("ACCEPTANCE REASON:"),
                    "Block passed all node validation checks"
                );
                debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Block structure:"), "VALID");
                debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Block header:"), "VALID");
                debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Transactions:"), "VALID");
                debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - DAA validation:"), "PASSED");
                debug!("{} {} {}", LogColors::api("[API]"), LogColors::label("  - Node Response:"), format!("{:?}", response));
                debug!(
                    "{} {} {}",
                    LogColors::api("[API]"),
                    LogColors::label("  - Blue Score:"),
                    format!("{}, Timestamp: {}, Nonce: {:x}", blue_score, timestamp, nonce)
                );

                // Optional: Check if block appears in tip hashes (verifies propagation)
                // This is informational only - block may still propagate even if not immediately in tips
                let client_clone = Arc::clone(&self.client);
                let block_hash_clone = block_hash.clone();
                let block_hash_for_check = header::hash(&block.header); // Use the actual Hash type
                tokio::spawn(async move {
                    // Wait a bit for block to be processed and potentially added to DAG
                    tokio::time::sleep(Duration::from_secs(2)).await;

                    // Check if block appears in tip hashes
                    if let Ok(dag_response) = client_clone.get_block_dag_info_call(None, GetBlockDagInfoRequest {}).await {
                        // Check if our block hash is in tip hashes
                        let in_tips = dag_response.tip_hashes.contains(&block_hash_for_check);

                        if in_tips {
                            info!(
                                "{} {} {}",
                                LogColors::api("[API]"),
                                LogColors::block("Block appears in tip hashes (good sign for propagation)"),
                                format!("Hash: {}", block_hash_clone)
                            );
                        } else {
                            // This is not necessarily bad - block may still propagate or be in a side chain
                            info!(
                                "{} {} {}",
                                LogColors::api("[API]"),
                                LogColors::label("Block not yet in tip hashes (may still propagate)"),
                                format!("Hash: {}", block_hash_clone)
                            );
                            info!(
                                "{} {} {}",
                                LogColors::api("[API]"),
                                LogColors::label("  - Note:"),
                                "Block may be in a side chain or still propagating"
                            );
                            info!(
                                "{} {} {}",
                                LogColors::api("[API]"),
                                LogColors::label("  - Tip hashes count:"),
                                dag_response.tip_hashes.len()
                            );
                        }
                    }
                });
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("ErrDuplicateBlock") || error_str.contains("duplicate") {
                    warn!(
                        "{} {}",
                        LogColors::api("[API]"),
                        LogColors::validation(&format!("===== BLOCK REJECTED BY KASPA NODE: STALE ===== Hash: {}", block_hash))
                    );
                    warn!(
                        "{} {} {}",
                        LogColors::api("[API]"),
                        LogColors::label("REJECTION REASON:"),
                        "Block already exists in the network"
                    );
                    warn!("{} {}", LogColors::api("[API]"), LogColors::label("  - Block was previously submitted and accepted"));
                    warn!("{} {}", LogColors::api("[API]"), LogColors::label("  - This is a duplicate/stale block submission"));
                    warn!("{} {} {}", LogColors::api("[API]"), LogColors::error("  - Error:"), error_str);
                    warn!(
                        "{} {} {}",
                        LogColors::api("[API]"),
                        LogColors::label("  - Blue Score:"),
                        format!("{}, Timestamp: {}, Nonce: {:x}", blue_score, timestamp, nonce)
                    );
                } else {
                    error!(
                        "{} {}",
                        LogColors::api("[API]"),
                        LogColors::error(&format!("===== BLOCK REJECTED BY KASPA NODE: INVALID ===== Hash: {}", block_hash))
                    );
                    error!("{} {} {}", LogColors::api("[API]"), LogColors::label("REJECTION REASON:"), "Block failed node validation");
                    error!("{} {}", LogColors::api("[API]"), LogColors::label("  - Possible validation failures:"));
                    error!("{} {}", LogColors::api("[API]"), "    * Invalid block structure or format");
                    error!("{} {}", LogColors::api("[API]"), "    * Block header validation failed");
                    error!("{} {}", LogColors::api("[API]"), "    * Transaction validation failed");
                    error!("{} {}", LogColors::api("[API]"), "    * DAA (Difficulty Adjustment Algorithm) validation failed");
                    error!("{} {}", LogColors::api("[API]"), "    * Block does not meet network consensus rules");
                    error!("{} {} {}", LogColors::api("[API]"), LogColors::error("  - Error from node:"), error_str);
                    error!(
                        "{} {} {}",
                        LogColors::api("[API]"),
                        LogColors::label("  - Blue Score:"),
                        format!("{}, Timestamp: {}, Nonce: {:x}", blue_score, timestamp, nonce)
                    );
                }
            }
        }

        result
    }

    /// Wait for node to sync
    async fn wait_for_sync(&self) -> Result<()> {
        loop {
            match self.client.get_sync_status().await {
                Ok(is_synced) => {
                    if is_synced {
                        break;
                    }
                }
                Err(e) => {
                    debug!("failed to get sync status: {}, retrying...", e);
                }
            }

            sleep(Duration::from_secs(10)).await;
        }

        Ok(())
    }

    pub async fn wait_for_sync_with_shutdown(&self, mut shutdown_rx: watch::Receiver<bool>) -> Result<()> {
        debug!("checking kaspad sync state");

        loop {
            let sync_fut = self.client.get_sync_status();
            let sync_res = tokio::select! {
                _ = shutdown_rx.wait_for(|v| *v) => {
                    return Err(anyhow::anyhow!("shutdown requested"));
                }
                res = sync_fut => res,
            };

            match sync_res {
                Ok(is_synced) => {
                    if is_synced {
                        debug!("kaspad synced, starting server");
                        break;
                    }
                }
                Err(e) => {
                    warn!("failed to get sync status: {}, retrying...", e);
                }
            }

            warn!("Kaspa is not synced, waiting for sync before starting bridge");

            tokio::select! {
                _ = shutdown_rx.wait_for(|v| *v) => {
                    return Err(anyhow::anyhow!("shutdown requested"));
                }
                _ = sleep(Duration::from_secs(10)) => {}
            }
        }

        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        *self.connected.lock()
    }

    /// Get block template for a client
    pub async fn get_block_template(&self, wallet_addr: &str, _remote_app: &str, _canxium_addr: &str) -> Result<Block> {
        // Retry up to 3 times if we get "Odd number of digits" error
        // This error can occur if the block template has malformed hash fields
        let max_retries = 3;
        let mut last_error = None;

        for attempt in 0..max_retries {
            // Parse wallet address each time (in case Address doesn't implement Clone)
            let address =
                Address::try_from(wallet_addr).map_err(|e| anyhow::anyhow!("Could not decode address {}: {}", wallet_addr, e))?;

            // Request block template using RPC client wrapper
            let response = match self
                .client
                .get_block_template_call(None, GetBlockTemplateRequest::new(address, self.coinbase_tag.clone()))
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    if attempt < max_retries - 1 {
                        warn!("Failed to get block template (attempt {}/{}): {}, retrying...", attempt + 1, max_retries, e);
                        sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
                        continue;
                    }
                    return Err(anyhow::anyhow!("Failed to get block template after {} attempts: {}", max_retries, e));
                }
            };

            // Get RPC block from response
            let rpc_block = response.block;

            // Convert RpcRawBlock to Block
            // The RpcRawBlock contains the block data that we need to convert
            // The "Odd number of digits" error can occur here if hash fields have malformed hex strings
            match Block::try_from(rpc_block) {
                Ok(block) => {
                    // Validate that we can serialize the block header
                    // This catches "Odd number of digits" errors early
                    // Convert error to String immediately to avoid Send issues
                    let serialize_result = crate::hasher::serialize_block_header(&block).map_err(|e| e.to_string());

                    match serialize_result {
                        Ok(_) => {
                            return Ok(block);
                        }
                        Err(error_str) => {
                            if error_str.contains("Odd number of digits") {
                                last_error = Some(format!("Block has malformed hash field: {}", error_str));
                                if attempt < max_retries - 1 {
                                    warn!(
                                        "Block template has malformed hash field (attempt {}/{}), retrying...",
                                        attempt + 1,
                                        max_retries
                                    );
                                    sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
                                    continue;
                                }
                            }
                            // If it's a different error, return it
                            return Err(anyhow::anyhow!("Failed to serialize block header: {}", error_str));
                        }
                    }
                }
                Err(e) => {
                    let error_str = format!("{:?}", e);
                    last_error = Some(error_str.clone());
                    if error_str.contains("Odd number of digits") && attempt < max_retries - 1 {
                        warn!(
                            "Block conversion failed with 'Odd number of digits' error (attempt {}/{}), retrying...",
                            attempt + 1,
                            max_retries
                        );
                        sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
                        continue;
                    }
                    // If the error contains "Odd number of digits", provide more context
                    if error_str.contains("Odd number of digits") {
                        return Err(anyhow::anyhow!(
                            "Failed to convert RPC block to Block after {} attempts: {} - This usually indicates a malformed hash field in the block template from the Kaspa node. The block may have a hash with an odd-length hex string.",
                            max_retries,
                            error_str
                        ));
                    } else {
                        return Err(anyhow::anyhow!("Failed to convert RPC block to Block: {}", error_str));
                    }
                }
            }
        }

        // Should never reach here, but handle it just in case
        Err(anyhow::anyhow!("Failed to get valid block template after {} attempts: {:?}", max_retries, last_error))
    }

    /// Get balances by addresses (for Prometheus metrics)
    pub async fn get_balances_by_addresses(&self, addresses: &[String]) -> Result<Vec<(String, u64)>> {
        let parsed_addresses: Result<Vec<Address>, _> = addresses.iter().map(|addr| Address::try_from(addr.as_str())).collect();

        let addresses = parsed_addresses.map_err(|e| anyhow::anyhow!("Failed to parse addresses: {:?}", e))?;

        let utxos = self
            .client
            .get_utxos_by_addresses_call(None, kaspa_rpc_core::GetUtxosByAddressesRequest::new(addresses))
            .await
            .context("Failed to get UTXOs by addresses")?;

        // Calculate balances from UTXOs
        // Group entries by address
        let mut balance_map: HashMap<String, u64> = HashMap::new();
        for entry in utxos.entries {
            if let Some(address) = entry.address {
                let addr_str = address.to_string();
                let amount = entry.utxo_entry.amount;
                *balance_map.entry(addr_str).or_insert(0) += amount;
            }
        }
        let balances: Vec<(String, u64)> = balance_map.into_iter().collect();

        Ok(balances)
    }

    pub async fn get_current_block_color(&self, block_hash: &str) -> Result<bool> {
        let hash = RpcHash::from_str(block_hash).context("Failed to parse block hash")?;
        let resp = self
            .client
            .get_current_block_color_call(None, GetCurrentBlockColorRequest { hash })
            .await
            .context("Failed to query current block color")?;
        Ok(resp.blue)
    }

    /// Multi-subscriber notification hub for this client. Any consumer may
    /// subscribe; every subscriber independently sees every notification.
    pub fn notification_hub(&self) -> &Arc<NotificationHub> {
        &self.hub
    }

    /// Start listening for block template notifications (node-push with ticker
    /// fallback). May be called by EVERY stratum instance: each call takes an
    /// independent hub subscription, so all instances receive real
    /// notifications — the old implementation could only serve one caller
    /// (take()-once receiver) which forced instances 2..N onto pure polling.
    pub async fn start_block_template_listener<F>(self: Arc<Self>, block_wait_time: Duration, block_cb: F) -> Result<()>
    where
        F: FnMut() + Send + 'static,
    {
        let rx = self.hub.subscribe(HubScope::NewBlockTemplate);
        tokio::spawn(run_template_listener(rx, block_wait_time, None, block_cb));
        Ok(())
    }

    pub async fn start_block_template_listener_with_shutdown<F>(
        self: Arc<Self>,
        block_wait_time: Duration,
        shutdown_rx: watch::Receiver<bool>,
        block_cb: F,
    ) -> Result<()>
    where
        F: FnMut() + Send + 'static,
    {
        let rx = self.hub.subscribe(HubScope::NewBlockTemplate);
        tokio::spawn(run_template_listener(rx, block_wait_time, Some(shutdown_rx), block_cb));
        Ok(())
    }
}

/// The template-listener loop, shared by every stratum instance and factored
/// free of `KaspaApi` so multi-instance behavior is directly unit-testable.
///
/// Semantics preserved from the original single-consumer listener:
/// - node-push notifications trigger the callback immediately; the ticker is
///   reset after each real notification so polling only covers gaps;
/// - queued notification bursts are drained to a single callback invocation;
/// - ticker fallback fires the callback on `block_wait_time` cadence.
///
/// New with broadcast subscriptions:
/// - `Lagged(n)`: this subscriber fell behind the ring buffer. Notifications
///   are edge triggers ("a new template exists"), so the correct recovery is
///   one resync callback — identical in cost to a ticker tick — never silent
///   loss.
/// - `Closed`: the hub (and thus the client) is gone; the loop exits.
///
/// Deliberately REMOVED from the original: the per-iteration `wait_for_sync`
/// preamble and the `restart_channel` flag. Both were vestigial — the code's
/// own comments note the gRPC client reconnects automatically and the flag's
/// branch did nothing — and running the sync probe in N instances' loops
/// would multiply redundant RPC load under fan-out. Connectivity is observed
/// once, centrally (hub health + the existing stats/status threads).
pub(crate) async fn run_template_listener<F>(
    mut rx: broadcast::Receiver<Notification>,
    block_wait_time: Duration,
    mut shutdown_rx: Option<watch::Receiver<bool>>,
    mut block_cb: F,
) where
    F: FnMut() + Send + 'static,
{
    let mut ticker = tokio::time::interval(block_wait_time);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        if let Some(s) = shutdown_rx.as_ref() {
            if *s.borrow() {
                break;
            }
        }

        tokio::select! {
            _ = async {
                match shutdown_rx.as_mut() {
                    Some(s) => { let _ = s.changed().await; }
                    None => std::future::pending::<()>().await,
                }
            } => {
                if shutdown_rx.as_ref().is_some_and(|s| *s.borrow()) {
                    break;
                }
            }
            notification_result = rx.recv() => {
                match notification_result {
                    Ok(Notification::NewBlockTemplate(_)) => {
                        // Drain any queued burst into a single callback.
                        while rx.try_recv().is_ok() {}
                        block_cb();
                        ticker = tokio::time::interval(block_wait_time);
                        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    }
                    Ok(_) => {
                        // Other notification variants routed here would be a hub
                        // demux bug; the hub only feeds this scope's channel.
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("template listener lagged, missed {n} notifications; forcing resync");
                        while rx.try_recv().is_ok() {}
                        block_cb();
                        ticker = tokio::time::interval(block_wait_time);
                        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("Block template notification channel closed");
                        break;
                    }
                }
            }
            _ = ticker.tick() => {
                block_cb();
            }
        }
    }
}

#[async_trait::async_trait]
impl KaspaApiTrait for KaspaApi {
    async fn get_block_template(
        &self,
        wallet_addr: &str,
        _remote_app: &str,
        _canxium_addr: &str,
    ) -> Result<Block, Box<dyn std::error::Error + Send + Sync>> {
        KaspaApi::get_block_template(self, wallet_addr, "", "").await.map_err(|e| {
            let error_msg = e.to_string();
            Box::new(std::io::Error::other(error_msg)) as Box<dyn std::error::Error + Send + Sync>
        })
    }

    async fn submit_block(
        &self,
        block: Block,
    ) -> Result<kaspa_rpc_core::SubmitBlockResponse, Box<dyn std::error::Error + Send + Sync>> {
        KaspaApi::submit_block(self, block)
            .await
            .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_balances_by_addresses(
        &self,
        addresses: &[String],
    ) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error + Send + Sync>> {
        KaspaApi::get_balances_by_addresses(self, addresses)
            .await
            .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_current_block_color(&self, block_hash: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        KaspaApi::get_current_block_color(self, block_hash)
            .await
            .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send + Sync>)
    }
}

#[cfg(test)]
mod listener_tests {
    use super::*;
    use crate::notification_hub::{HubScope, NotificationHub, DEFAULT_HUB_CAPACITY};
    use kaspa_rpc_core::NewBlockTemplateNotification;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::timeout;

    fn template_notification() -> Notification {
        Notification::NewBlockTemplate(NewBlockTemplateNotification {})
    }

    /// WS2 acceptance: with one hub feeding N listener instances (the
    /// production shape is 9 per rig), a single node-push notification
    /// reaches EVERY instance's callback — the property the old
    /// is_first_instance gate made impossible.
    #[tokio::test]
    async fn all_nine_instances_receive_push_notifications() {
        let (tx, rx) = async_channel::unbounded();
        let hub = NotificationHub::start(rx, "test", DEFAULT_HUB_CAPACITY);

        let counters: Vec<Arc<AtomicUsize>> = (0..9).map(|_| Arc::new(AtomicUsize::new(0))).collect();
        for c in &counters {
            let c = Arc::clone(c);
            // Long block_wait_time so the ticker cannot masquerade as a push.
            tokio::spawn(run_template_listener(
                hub.subscribe(HubScope::NewBlockTemplate),
                Duration::from_secs(3600),
                None,
                move || {
                    c.fetch_add(1, Ordering::SeqCst);
                },
            ));
        }
        // Interval's first tick fires immediately; absorb it before pushing.
        tokio::time::sleep(Duration::from_millis(100)).await;
        let baseline: Vec<usize> = counters.iter().map(|c| c.load(Ordering::SeqCst)).collect();

        tx.send(template_notification()).await.unwrap();

        timeout(Duration::from_secs(2), async {
            loop {
                let all = counters.iter().zip(&baseline).all(|(c, b)| c.load(Ordering::SeqCst) > *b);
                if all {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("every instance's callback fires from one push notification");
    }

    /// Ticker fallback: with no notifications flowing, the callback still
    /// fires on block_wait_time cadence (polling as safety net, not primary).
    #[tokio::test]
    async fn ticker_fallback_fires_without_notifications() {
        let (_tx, rx) = async_channel::unbounded::<Notification>();
        let hub = NotificationHub::start(rx, "test", DEFAULT_HUB_CAPACITY);
        let count = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&count);
        tokio::spawn(run_template_listener(
            hub.subscribe(HubScope::NewBlockTemplate),
            Duration::from_millis(50),
            None,
            move || {
                c.fetch_add(1, Ordering::SeqCst);
            },
        ));
        tokio::time::sleep(Duration::from_millis(300)).await;
        let n = count.load(Ordering::SeqCst);
        assert!(n >= 3, "expected several ticker callbacks, got {n}");
    }

    /// Shutdown signal terminates the listener promptly.
    #[tokio::test]
    async fn shutdown_stops_listener() {
        let (tx, rx) = async_channel::unbounded();
        let hub = NotificationHub::start(rx, "test", DEFAULT_HUB_CAPACITY);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let count = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&count);
        let handle = tokio::spawn(run_template_listener(
            hub.subscribe(HubScope::NewBlockTemplate),
            Duration::from_secs(3600),
            Some(shutdown_rx),
            move || {
                c.fetch_add(1, Ordering::SeqCst);
            },
        ));
        tokio::time::sleep(Duration::from_millis(50)).await;
        shutdown_tx.send(true).unwrap();
        timeout(Duration::from_secs(2), handle).await.expect("listener exits on shutdown").unwrap();
        // Later notifications reach no callback.
        let before = count.load(Ordering::SeqCst);
        tx.send(template_notification()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(count.load(Ordering::SeqCst), before);
    }
}
