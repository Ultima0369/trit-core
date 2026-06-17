// Seed-based peer discovery for Trit-Core distributed nodes.
//
// Nodes are configured with a list of seed peer addresses (host:port).
// On startup, each node connects to its seeds, exchanges HEARTBEAT messages
// to establish presence, and integrates discovered peers into the local
// ResonanceBus.
//
// ## Protocol
//
// 1. Node starts TcpNodeServer on its bind port
// 2. Node connects to each seed peer via TcpClient
// 3. Node sends HEARTBEAT to each seed (announcing frame + phase)
// 4. Node registers the seed's echoed state in its local bus
// 5. Node enters normal operation — coupling/decoupling/negotiate as usual
//
// ## Configuration
//
// Seeds are provided via:
// - `--peers` CLI flag: comma-separated host:port list
// - `TRIT_PEERS` environment variable: same format

use crate::net::bus::ResonanceBus;
use crate::net::message::MessagePayload;
use crate::net::node::Node;
use crate::net::tcp_client::TcpClient;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Parse a comma-separated list of seed peer addresses.
///
/// Format: `host:port,host:port,...`
pub fn parse_seeds(seeds: &str) -> Vec<String> {
    seeds
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Bootstrap the node by connecting to all seed peers and registering
/// their state in the local bus.
///
/// This function should be called after the local node is registered
/// and the TcpNodeServer is listening.
///
/// Returns the number of successfully contacted seeds.
pub async fn bootstrap(
    bus: &Arc<Mutex<ResonanceBus>>,
    local_node_id: &str,
    seeds: &[String],
) -> usize {
    if seeds.is_empty() {
        info!("No seed peers configured — running in standalone mode");
        return 0;
    }

    info!(seed_count = seeds.len(), "Bootstrapping from seed peers");

    let mut successful = 0usize;

    for seed in seeds {
        debug!(seed = %seed, "Connecting to seed peer");
        match TcpClient::connect(seed).await {
            Ok(mut client) => {
                let (_local_frame, local_phase) = {
                    let b = bus.lock().await;
                    let node = b.get_node(local_node_id).unwrap();
                    (format!("{}", node.frame), node.current_phase)
                };

                // Announce our presence with a HEARTBEAT
                match client
                    .heartbeat(local_node_id, "Sovereign", local_phase)
                    .await
                {
                    Ok(resp) => {
                        // Extract peer info from the heartbeat echo
                        match &resp.payload {
                            MessagePayload::Heartbeat(hb) => {
                                debug!(
                                    peer = %seed,
                                    state = %hb.node_state,
                                    phase = hb.current_phase,
                                    "Received heartbeat echo from seed"
                                );

                                // Parse peer frame from state string
                                let peer_frame = extract_frame_from_state(&hb.node_state);
                                let peer_phase = hb.current_phase;

                                // Register the seed peer in the local bus
                                {
                                    let mut b = bus.lock().await;
                                    if b.get_node(seed).is_none() {
                                        let peer_node =
                                            Node::new(seed.clone(), peer_frame, peer_phase);
                                        b.register(peer_node);
                                        info!(peer = %seed, "Registered seed peer in bus");
                                    }
                                }

                                successful += 1;
                            }
                            _ => {
                                warn!(peer = %seed, "Unexpected response type from seed");
                            }
                        }
                    }
                    Err(e) => {
                        warn!(peer = %seed, error = %e, "Failed to heartbeat with seed");
                    }
                }
            }
            Err(e) => {
                warn!(peer = %seed, error = %e, "Failed to connect to seed peer");
            }
        }
    }

    info!(
        successful = successful,
        total = seeds.len(),
        "Seed bootstrap complete"
    );
    successful
}

/// Try to extract a Frame from a node state string.
/// The heartbeat includes `node_state` as a debug string like "Sovereign".
/// We fall back to a generic frame — the actual frame is embedded in
/// the peer's bus registration via other means (e.g., the seed list
/// itself can carry frame metadata).
fn extract_frame_from_state(state: &str) -> crate::frame::Frame {
    // The state string is the NodeState debug representation.
    // For a more complete implementation, the heartbeat protocol
    // could be extended to carry explicit frame information.
    // For now, we default to General — the peer's actual frame
    // matters most during resonance, not discovery.
    let _ = state;
    crate::frame::Frame::Meta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_seeds() {
        assert_eq!(parse_seeds(""), Vec::<String>::new());
    }

    #[test]
    fn parse_single_seed() {
        let seeds = parse_seeds("192.168.1.1:9000");
        assert_eq!(seeds, vec!["192.168.1.1:9000"]);
    }

    #[test]
    fn parse_multiple_seeds() {
        let seeds = parse_seeds("192.168.1.1:9000, 192.168.1.2:9000, 192.168.1.3:9000");
        assert_eq!(
            seeds,
            vec!["192.168.1.1:9000", "192.168.1.2:9000", "192.168.1.3:9000"]
        );
    }

    #[test]
    fn parse_seeds_ignores_empty_entries() {
        let seeds = parse_seeds("host:9000,, ,host2:9000,");
        assert_eq!(seeds, vec!["host:9000", "host2:9000"]);
    }
}
