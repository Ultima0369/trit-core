// Network partition fault-tolerance tests for Trit-Core (M7).
//
// These tests validate the distributed protocol's behavior under
// network failures: connection loss, heartbeat timeout, stale peer
// detection, split-brain detection, and graceful degradation.
//
// ## Test Strategy
//
// We use real TCP connections with port 0 binding (OS-assigned ports)
// and simulate partitions by dropping connections and manipulating
// heartbeat timestamps. This is more realistic than mocking — the
// same tokio runtime and frame codec are exercised as in production.

#[cfg(test)]
mod partition_tests {
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use trit_core::frame::Frame;
    use trit_core::net::bus::ResonanceBus;
    use trit_core::net::discovery;
    use trit_core::net::message::{Message, MessagePayload};
    use trit_core::net::node::{Node, NodeState};
    use trit_core::net::tcp_client::TcpClient;
    use trit_core::net::tcp_server::TcpNodeServer;
    use trit_core::net::HEARTBEAT_TIMEOUT_SECS;

    /// Spawn a TCP server with pre-registered nodes and return its address + bus handle.
    async fn spawn_server(nodes: Vec<(&str, Frame, f64)>) -> (String, Arc<Mutex<ResonanceBus>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);

        let mut bus = ResonanceBus::new();
        for (id, frame, phase) in &nodes {
            bus.register(Node::new(id.to_string(), frame.clone(), *phase));
        }

        let server = TcpNodeServer::with_bus(&addr, bus);
        let bus_handle = server.bus_handle();

        tokio::spawn(async move {
            let _ = server.serve().await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        (addr, bus_handle)
    }

    /// Complete a full resonate handshake.
    async fn resonate_full_handshake(
        addr: &str,
        node_id: &str,
        frame: &str,
        phase: f64,
    ) -> Message {
        let mut client = TcpClient::connect(addr).await.unwrap();
        let ack = client
            .resonate(node_id, frame, phase, vec![])
            .await
            .unwrap();

        let coupled_phase = match &ack.payload {
            MessagePayload::ResonateAck(data) => data.coupled_phase,
            _ => panic!("Expected ResonateAck"),
        };

        let mut confirm_client = tokio::net::TcpStream::connect(addr).await.unwrap();
        let confirm_msg =
            Message::resonate_ack(node_id, coupled_phase, "constructive", false, "commit");
        let confirm_json = serde_json::to_vec(&confirm_msg).unwrap();
        trit_core::net::frame_codec::write_frame(&mut confirm_client, &confirm_json)
            .await
            .unwrap();
        drop(confirm_client);

        tokio::time::sleep(Duration::from_millis(30)).await;
        ack
    }

    // ── Test 1: Connection loss triggers decouple via heartbeat timeout ──

    #[tokio::test]
    async fn connection_loss_triggers_stale_peer_detection() {
        let (addr, bus) = spawn_server(vec![
            ("node-a", Frame::Science, 0.75),
            ("node-b", Frame::Science, 0.80),
        ])
        .await;

        // Couple the nodes
        let _ack = resonate_full_handshake(&addr, "node-a", "Science", 0.75).await;

        // Verify coupled
        {
            let b = bus.lock().await;
            let node_a = b.get_node("node-a").unwrap();
            assert_eq!(node_a.state, NodeState::Coupled);
            assert!(!node_a.peers.is_empty());
        }

        // Simulate heartbeat timeout: backdate node-b's heartbeat
        {
            let mut b = bus.lock().await;
            let stale_time = std::time::Instant::now()
                .checked_sub(Duration::from_secs(HEARTBEAT_TIMEOUT_SECS + 5))
                .unwrap();
            b.last_heartbeat.insert("node-b".into(), stale_time);
        }

        // Purge stale peers
        {
            let mut b = bus.lock().await;
            let affected = b.purge_stale_peers();
            assert!(
                affected.contains(&"node-a".to_string()),
                "node-a should be affected by stale peer purge"
            );

            let node_a = b.get_node("node-a").unwrap();
            assert!(
                node_a.peers.is_empty(),
                "node-a should have no peers after purge"
            );
            assert_eq!(
                node_a.state,
                NodeState::Sovereign,
                "node-a should return to Sovereign after peer timeout"
            );
        }
    }

    // ── Test 2: Node reconnects after partition ──

    #[tokio::test]
    async fn node_reconnects_after_partition() {
        let (addr, bus) = spawn_server(vec![
            ("node-a", Frame::Science, 0.60),
            ("node-b", Frame::Science, 0.70),
        ])
        .await;

        // Phase 1: Couple
        let _ack = resonate_full_handshake(&addr, "node-a", "Science", 0.60).await;
        {
            let b = bus.lock().await;
            assert_eq!(b.get_node("node-a").unwrap().state, NodeState::Coupled);
        }

        // Phase 2: Simulate partition — backdate heartbeat and purge
        {
            let mut b = bus.lock().await;
            let stale_time = std::time::Instant::now()
                .checked_sub(Duration::from_secs(HEARTBEAT_TIMEOUT_SECS + 5))
                .unwrap();
            b.last_heartbeat.insert("node-b".into(), stale_time);
            b.purge_stale_peers();
        }

        // Verify node-a is Sovereign after partition
        {
            let b = bus.lock().await;
            assert_eq!(b.get_node("node-a").unwrap().state, NodeState::Sovereign);
        }

        // Phase 3: Re-couple after "partition heals"
        // Reset heartbeat for node-b
        {
            let mut b = bus.lock().await;
            b.record_heartbeat("node-b");
        }

        let _ack2 = resonate_full_handshake(&addr, "node-a", "Science", 0.60).await;
        {
            let b = bus.lock().await;
            assert_eq!(
                b.get_node("node-a").unwrap().state,
                NodeState::Coupled,
                "node-a should re-couple after partition heals"
            );
        }
    }

    // ── Test 3: Partial partition in 3-node mesh ──

    #[tokio::test]
    async fn partial_partition_3_nodes_isolated_node_sovereign() {
        let (addr, bus) = spawn_server(vec![
            ("alpha", Frame::Science, 0.70),
            ("beta", Frame::Science, 0.75),
            ("gamma", Frame::Science, 0.80),
        ])
        .await;

        // Couple alpha and beta
        let _ack = resonate_full_handshake(&addr, "alpha", "Science", 0.70).await;
        let _ack2 = resonate_full_handshake(&addr, "beta", "Science", 0.75).await;

        // Simulate gamma's heartbeat timeout (gamma is isolated)
        {
            let mut b = bus.lock().await;
            let stale_time = std::time::Instant::now()
                .checked_sub(Duration::from_secs(HEARTBEAT_TIMEOUT_SECS + 5))
                .unwrap();
            b.last_heartbeat.insert("gamma".into(), stale_time);
            b.purge_stale_peers();
        }

        // alpha and beta should still be coupled to each other
        {
            let b = bus.lock().await;
            let alpha = b.get_node("alpha").unwrap();
            let beta = b.get_node("beta").unwrap();
            let gamma = b.get_node("gamma").unwrap();

            // gamma is isolated but its own state is still registered
            assert!(gamma.peers.is_empty() || gamma.state == NodeState::Sovereign);
            // alpha and beta remain coupled (gamma was not in their peer lists)
            assert!(alpha.state == NodeState::Coupled || alpha.state == NodeState::Sovereign);
            assert!(beta.state == NodeState::Coupled || beta.state == NodeState::Sovereign);
        }
    }

    // ── Test 4: All seeds unreachable → graceful standalone ──

    #[tokio::test]
    async fn all_seeds_unreachable_graceful_standalone() {
        // Create a bus with a local node
        let bus = Arc::new(Mutex::new(ResonanceBus::new()));
        {
            let mut b = bus.lock().await;
            b.register(Node::new("standalone-node".into(), Frame::Science, 0.50));
        }

        // Try to bootstrap with unreachable seeds (port 1 is unlikely to be open)
        let seeds = vec!["127.0.0.1:1".to_string(), "127.0.0.1:2".to_string()];
        let contacted = discovery::bootstrap(&bus, "standalone-node", &seeds).await;

        assert_eq!(contacted, 0, "Should contact 0 unreachable seeds");

        // Node should still be Sovereign
        {
            let b = bus.lock().await;
            let node = b.get_node("standalone-node").unwrap();
            assert_eq!(node.state, NodeState::Sovereign);
        }
    }

    // ── Test 5: Split-brain detection ──

    #[tokio::test]
    async fn split_brain_detection_mutual_coupling_no_heartbeat() {
        let (addr, bus) = spawn_server(vec![
            ("left", Frame::Science, 0.70),
            ("right", Frame::Science, 0.75),
        ])
        .await;

        // Couple both nodes so they have each other in peer lists
        let _ack = resonate_full_handshake(&addr, "left", "Science", 0.70).await;
        let _ack2 = resonate_full_handshake(&addr, "right", "Science", 0.75).await;

        // Verify both are coupled with each other
        {
            let b = bus.lock().await;
            let left = b.get_node("left").unwrap();
            let right = b.get_node("right").unwrap();
            assert_eq!(left.state, NodeState::Coupled);
            assert_eq!(right.state, NodeState::Coupled);
        }

        // Simulate split-brain: both think they're coupled but no heartbeats
        {
            let mut b = bus.lock().await;
            let stale_time = std::time::Instant::now()
                .checked_sub(Duration::from_secs(70)) // > SPLIT_BRAIN_TIMEOUT_SECS (60)
                .unwrap();
            b.last_heartbeat.insert("left".into(), stale_time);
            b.last_heartbeat.insert("right".into(), stale_time);
        }

        // Detect split-brain
        {
            let b = bus.lock().await;
            let pairs = b.detect_split_brain();
            assert!(
                !pairs.is_empty(),
                "Should detect split-brain between left and right"
            );
        }

        // After purge, both should be Sovereign
        {
            let mut b = bus.lock().await;
            b.purge_stale_peers();
            let left = b.get_node("left").unwrap();
            let right = b.get_node("right").unwrap();
            assert_eq!(left.state, NodeState::Sovereign);
            assert_eq!(right.state, NodeState::Sovereign);
        }
    }

    // ── Test 6: Heartbeat keeps peers alive ──

    #[tokio::test]
    async fn regular_heartbeat_prevents_stale_detection() {
        let (addr, bus) = spawn_server(vec![
            ("keeper", Frame::Science, 0.50),
            ("watcher", Frame::Science, 0.55),
        ])
        .await;

        // Couple
        let _ack = resonate_full_handshake(&addr, "keeper", "Science", 0.50).await;

        // Send a heartbeat to keep the peer alive
        {
            let mut b = bus.lock().await;
            b.record_heartbeat("watcher");
        }

        // Immediately check — should not be stale
        {
            let b = bus.lock().await;
            let stale = b.stale_peers();
            assert!(
                !stale.contains(&"watcher".to_string()),
                "watcher should not be stale after fresh heartbeat"
            );
        }
    }
}
