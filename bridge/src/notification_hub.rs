//! NotificationHub: multi-subscriber fan-out for a node's notification stream.
//!
//! # Why this exists
//!
//! `GrpcClient::notification_channel_receiver()` returns a clone of an
//! `async_channel::Receiver` — a work-stealing MPMC queue where each message is
//! delivered to exactly ONE receiver clone. The bridge historically worked
//! around this by wrapping the stream in a `tokio::sync::mpsc` receiver stored
//! as `Arc<Mutex<Option<...>>>` and claimed via `.take()`, which allows exactly
//! one consumer per process. `main.rs` therefore gated real notifications to
//! the first stratum instance (`is_first_instance`) and left every other
//! instance on ticker polling.
//!
//! The hub replaces that pattern: ONE relay task per node client owns the
//! upstream receiver, demultiplexes each notification by variant, and fans it
//! out through per-scope `tokio::sync::broadcast` channels. Any number of
//! consumers — every stratum instance, and future subscribers such as an
//! own-block fate tracker on `VirtualChainChanged` — call [`NotificationHub::subscribe`]
//! and each independently sees every notification for their scope.
//!
//! # Critical design note: one hub per CLIENT, not per scope
//!
//! All scopes subscribed on a `GrpcClient` multiplex into its single
//! notification channel. Two hubs reading the same client would steal
//! notifications from each other (the MPMC trap, one layer down). The hub
//! therefore owns the client's entire stream and demuxes internally; adding a
//! new scope is [`HubScope`] + one `start_notify` call + one match arm, never a
//! second reader.
//!
//! # Health state
//!
//! The relay doubles as the client's liveness observer, publishing
//! [`ClientHealth`] through a `tokio::sync::watch` channel. This is the signal
//! the WS4 mode machine consumes to drive MERGED / KAS-ONLY / ZKAS-ONLY /
//! ISLANDED transitions; building it here means node-health tracking exists in
//! exactly one place.
//!
//! # Lag semantics
//!
//! `broadcast` is a bounded ring buffer. A subscriber that falls behind
//! receives `RecvError::Lagged(n)` instead of silently losing data; consumers
//! treat that as "notifications happened while you weren't looking" and
//! resync (for template listeners: fire the refresh callback). The capacity
//! default is generous relative to notification rates (10 BPS worst case on a
//! Kaspa parent stream) precisely so `Lagged` marks a stalled consumer rather
//! than normal operation.

use kaspa_rpc_core::Notification;
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use tracing::{debug, warn};

/// Default broadcast ring capacity per scope. At 10 notifications/sec (Kaspa
/// parent templates, the fastest stream), 256 slots is ~25 seconds of buffer —
/// a consumer that far behind is stalled, and `Lagged` + resync is the correct
/// outcome.
pub const DEFAULT_HUB_CAPACITY: usize = 256;

/// Notification scopes the hub demultiplexes. Mirrors the subset of
/// `kaspa_notify::scope::Scope` the bridge consumes; extend alongside a new
/// `start_notify` registration and a match arm in the relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HubScope {
    /// New block template available (job trigger for either chain's node).
    NewBlockTemplate,
    /// Virtual selected-parent chain changed (own-block fate tracking, WS3-FT).
    VirtualChainChanged,
}

/// Liveness of the relayed node stream, published via `watch` so any number of
/// observers (mode machine, status display, metrics) can watch transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientHealth {
    /// Relay is consuming the client's notification stream.
    Receiving,
    /// Upstream channel closed or errored; relay is waiting for the client's
    /// automatic reconnect before resuming. Consumers should treat templates
    /// from this client as stale and (per WS4) transition modes if the state
    /// persists past their grace period.
    Disconnected,
}

/// Per-client notification fan-out. Construct one per node connection via
/// [`NotificationHub::start`]; hold it wherever the client lives (KaspaApi).
pub struct NotificationHub {
    template_tx: broadcast::Sender<Notification>,
    chain_tx: broadcast::Sender<Notification>,
    health_rx: watch::Receiver<ClientHealth>,
    _relay: tokio::task::JoinHandle<()>,
}

impl NotificationHub {
    /// Spawn the relay over an already-subscribed upstream receiver.
    ///
    /// The caller performs `start_notify` registration for every scope it
    /// wants relayed BEFORE or AFTER calling this (the gRPC client maintains
    /// subscriptions across its own reconnects); the hub's contract is only
    /// "whatever arrives on `upstream` is demuxed and fanned out". Taking the
    /// receiver rather than the client keeps the hub free of gRPC coupling and
    /// makes it directly unit-testable with a locally constructed channel.
    ///
    /// `label` names the client in logs (e.g. "zkas", "kaspa-parent").
    pub fn start(upstream: async_channel::Receiver<Notification>, label: &str, capacity: usize) -> Arc<Self> {
        let (template_tx, _) = broadcast::channel(capacity);
        let (chain_tx, _) = broadcast::channel(capacity);
        let (health_tx, health_rx) = watch::channel(ClientHealth::Receiving);

        let relay_template_tx = template_tx.clone();
        let relay_chain_tx = chain_tx.clone();
        let label = label.to_string();

        let relay = tokio::spawn(async move {
            loop {
                match upstream.recv().await {
                    Ok(notification) => {
                        // A successful receive after a disconnect means the
                        // client reconnected underneath us; flip health back
                        // exactly once per transition (send_if_modified keeps
                        // watch wakeups minimal).
                        health_tx.send_if_modified(|h| {
                            if *h != ClientHealth::Receiving {
                                *h = ClientHealth::Receiving;
                                true
                            } else {
                                false
                            }
                        });
                        match &notification {
                            Notification::NewBlockTemplate(_) => {
                                // send() errs only when zero subscribers exist,
                                // which is legal (e.g. during startup ordering).
                                let _ = relay_template_tx.send(notification);
                            }
                            Notification::VirtualChainChanged(_) => {
                                let _ = relay_chain_tx.send(notification);
                            }
                            other => {
                                debug!("[hub:{label}] ignoring unrouted notification variant: {other:?}");
                            }
                        }
                    }
                    Err(_) => {
                        // Upstream async_channel closed: the client dropped its
                        // sender, which happens on hard disconnect. The gRPC
                        // client reconnects automatically and its channel is
                        // long-lived, so closure here is terminal for THIS
                        // receiver; publish Disconnected and end the relay.
                        // (KaspaApi recreates the hub on client rebuild; the
                        // watch channel keeps serving the last value to
                        // existing observers.)
                        warn!("[hub:{label}] upstream notification channel closed; relay exiting");
                        let _ = health_tx.send(ClientHealth::Disconnected);
                        break;
                    }
                }
            }
        });

        Arc::new(Self { template_tx, chain_tx, health_rx, _relay: relay })
    }

    /// Independent subscription to a scope's stream. Every subscriber sees
    /// every notification for the scope (subject to `Lagged` semantics).
    pub fn subscribe(&self, scope: HubScope) -> broadcast::Receiver<Notification> {
        match scope {
            HubScope::NewBlockTemplate => self.template_tx.subscribe(),
            HubScope::VirtualChainChanged => self.chain_tx.subscribe(),
        }
    }

    /// Watchable health of the relayed client stream (WS4 mode-machine input).
    pub fn health(&self) -> watch::Receiver<ClientHealth> {
        self.health_rx.clone()
    }

    /// Current subscriber count for a scope (metrics/diagnostics).
    pub fn subscriber_count(&self, scope: HubScope) -> usize {
        match scope {
            HubScope::NewBlockTemplate => self.template_tx.receiver_count(),
            HubScope::VirtualChainChanged => self.chain_tx.receiver_count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaspa_rpc_core::{NewBlockTemplateNotification, VirtualChainChangedNotification};
    use std::time::Duration;
    use tokio::time::timeout;

    fn template_notification() -> Notification {
        Notification::NewBlockTemplate(NewBlockTemplateNotification {})
    }

    fn chain_notification() -> Notification {
        Notification::VirtualChainChanged(VirtualChainChangedNotification {
            added_chain_block_hashes: Arc::new(vec![]),
            removed_chain_block_hashes: Arc::new(vec![]),
            accepted_transaction_ids: Arc::new(vec![]),
        })
    }

    /// Every subscriber sees every notification for its scope — the property
    /// the old take()-once mpsc pattern made impossible and the reason the
    /// is_first_instance gate existed.
    #[tokio::test]
    async fn all_subscribers_receive_every_template_notification() {
        let (tx, rx) = async_channel::unbounded();
        let hub = NotificationHub::start(rx, "test", DEFAULT_HUB_CAPACITY);

        // Nine subscribers: the production instance count.
        let mut subs: Vec<_> = (0..9).map(|_| hub.subscribe(HubScope::NewBlockTemplate)).collect();

        for _ in 0..3 {
            tx.send(template_notification()).await.unwrap();
        }

        for (i, sub) in subs.iter_mut().enumerate() {
            for n in 0..3 {
                let got = timeout(Duration::from_secs(1), sub.recv())
                    .await
                    .unwrap_or_else(|_| panic!("subscriber {i} timed out waiting for notification {n}"))
                    .unwrap();
                assert!(matches!(got, Notification::NewBlockTemplate(_)));
            }
        }
    }

    /// Demux correctness: scopes are isolated. A VirtualChainChanged consumer
    /// (WS3-FT) never steals a template notification from a stratum instance,
    /// and vice versa — the MPMC trap this design exists to prevent.
    #[tokio::test]
    async fn scopes_are_demultiplexed_without_cross_stealing() {
        let (tx, rx) = async_channel::unbounded();
        let hub = NotificationHub::start(rx, "test", DEFAULT_HUB_CAPACITY);

        let mut template_sub = hub.subscribe(HubScope::NewBlockTemplate);
        let mut chain_sub = hub.subscribe(HubScope::VirtualChainChanged);

        tx.send(chain_notification()).await.unwrap();
        tx.send(template_notification()).await.unwrap();
        tx.send(chain_notification()).await.unwrap();

        // Template subscriber: exactly the one template, nothing else pending.
        let got = timeout(Duration::from_secs(1), template_sub.recv()).await.unwrap().unwrap();
        assert!(matches!(got, Notification::NewBlockTemplate(_)));
        assert!(matches!(template_sub.try_recv(), Err(broadcast::error::TryRecvError::Empty)));

        // Chain subscriber: exactly the two chain notifications.
        for _ in 0..2 {
            let got = timeout(Duration::from_secs(1), chain_sub.recv()).await.unwrap().unwrap();
            assert!(matches!(got, Notification::VirtualChainChanged(_)));
        }
        assert!(matches!(chain_sub.try_recv(), Err(broadcast::error::TryRecvError::Empty)));
    }

    /// A slow subscriber gets Lagged (with an accurate skip count) and then
    /// resumes from the live stream — the consumer contract is "treat Lagged
    /// as missed-work-happened and resync", never silent loss.
    #[tokio::test]
    async fn lagged_subscriber_learns_it_lagged_then_recovers() {
        let (tx, rx) = async_channel::unbounded();
        // Tiny capacity to force lag deterministically.
        let hub = NotificationHub::start(rx, "test", 2);

        let mut sub = hub.subscribe(HubScope::NewBlockTemplate);

        // Overrun the ring: 5 sends into capacity 2 without any recv.
        for _ in 0..5 {
            tx.send(template_notification()).await.unwrap();
        }
        // Let the relay drain the upstream channel into the ring.
        tokio::time::sleep(Duration::from_millis(50)).await;

        match sub.recv().await {
            Err(broadcast::error::RecvError::Lagged(n)) => assert_eq!(n, 3, "capacity 2, 5 sent -> 3 skipped"),
            other => panic!("expected Lagged, got {other:?}"),
        }
        // Post-lag: the two retained notifications, then live traffic again.
        for _ in 0..2 {
            assert!(matches!(sub.recv().await.unwrap(), Notification::NewBlockTemplate(_)));
        }
        tx.send(template_notification()).await.unwrap();
        let got = timeout(Duration::from_secs(1), sub.recv()).await.unwrap().unwrap();
        assert!(matches!(got, Notification::NewBlockTemplate(_)));
    }

    /// Upstream closure publishes Disconnected exactly once and live
    /// subscribers see channel-closed — the WS4 mode machine's input signal.
    #[tokio::test]
    async fn upstream_closure_publishes_disconnected_health() {
        let (tx, rx) = async_channel::unbounded();
        let hub = NotificationHub::start(rx, "test", DEFAULT_HUB_CAPACITY);
        let mut health = hub.health();
        assert_eq!(*health.borrow(), ClientHealth::Receiving);

        let mut sub = hub.subscribe(HubScope::NewBlockTemplate);
        drop(tx); // hard disconnect

        health.changed().await.expect("health transition");
        assert_eq!(*health.borrow(), ClientHealth::Disconnected);
        assert!(matches!(sub.recv().await, Err(broadcast::error::RecvError::Closed)));
    }

    /// Subscribing before any notification flows, and late subscription after
    /// traffic, both behave sanely: late subscribers see only post-subscribe
    /// notifications (broadcast semantics — templates are ephemeral, so
    /// missing history is correct; a fresh consumer fetches current state
    /// directly, it doesn't replay stale templates).
    #[tokio::test]
    async fn late_subscriber_sees_only_future_notifications() {
        let (tx, rx) = async_channel::unbounded();
        let hub = NotificationHub::start(rx, "test", DEFAULT_HUB_CAPACITY);

        tx.send(template_notification()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await; // relay drains

        let mut late = hub.subscribe(HubScope::NewBlockTemplate);
        assert!(matches!(late.try_recv(), Err(broadcast::error::TryRecvError::Empty)));

        tx.send(template_notification()).await.unwrap();
        let got = timeout(Duration::from_secs(1), late.recv()).await.unwrap().unwrap();
        assert!(matches!(got, Notification::NewBlockTemplate(_)));
    }
}
