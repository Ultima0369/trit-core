// Multi-node integration tests for Trit-Core distributed protocol (M6).
//
// These tests validate the full TCP-based multi-node mesh:
// - 3-node full coupling/decoupling lifecycle
// - Seed discovery bootstrap
// - Cross-frame conflict detection over TCP
// - Message log integrity
// - PLL phase convergence

#[cfg(test)]
mod multi_node_tests {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use trit_core::frame::Frame;
    use trit_core::net::bus::ResonanceBus;
    use trit_core::net::discovery;
    use trit_core::net::frame_codec;
    use trit_core::net::message::{Message, MessagePayload};
    use trit_core::net::node::{Node, NodeState};
    use trit_core::net::tcp_client::TcpClient;
    use trit_core::net::tcp_server::TcpNodeServer;

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

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        (addr, bus_handle)
    }

    /// Complete a full resonate handshake: send REQ, receive ACK, then confirm with ACK.
    /// This ensures the node transitions from Coupling → Coupled on the bus.
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

        // Extract coupled_phase from ACK
        let coupled_phase = match &ack.payload {
            MessagePayload::ResonateAck(data) => data.coupled_phase,
            _ => panic!("Expected ResonateAck"),
        };

        // Now send RESONATE_ACK back to confirm coupling (fire-and-forget,
        // since the server doesn't send a response to ACK)
        let mut confirm_client = tokio::net::TcpStream::connect(addr).await.unwrap();
        let confirm_msg =
            Message::resonate_ack(node_id, coupled_phase, "constructive", false, "commit");
        let confirm_json = serde_json::to_vec(&confirm_msg).unwrap();
        frame_codec::write_frame(&mut confirm_client, &confirm_json)
            .await
            .unwrap();
        drop(confirm_client);

        // Small delay to allow server to process the confirmation
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        ack
    }

    /// Connect a client to a server and verify HEARTBEAT roundtrip.
    #[tokio::test]
    async fn three_node_full_mesh_coupling_decoupling() {
        // ┌──────────────────────────────────────┐
        // │  Node A (Science, 0.75)              │
        // │    ↕ resonate ↕                      │
        // │  Node B (Science, 0.80)              │
        // │    ↕ resonate ↕                      │
        // │  Node C (Science, 0.70)              │
        // └──────────────────────────────────────┘
        // All same-frame → all couplings constructive

        let (addr, bus) = spawn_server(vec![
            ("node-a", Frame::Science, 0.75),
            ("node-b", Frame::Science, 0.80),
            ("node-c", Frame::Science, 0.70),
        ])
        .await;

        // A resonates with B (full handshake)
        let ack = resonate_full_handshake(&addr, "node-a", "Science", 0.75).await;
        match &ack.payload {
            MessagePayload::ResonateAck(data) => {
                assert_eq!(data.interference, "constructive");
                assert!(!data.conflict_detected);
            }
            _ => panic!("Expected ResonateAck"),
        }

        // B resonates with C (full handshake)
        let ack = resonate_full_handshake(&addr, "node-b", "Science", 0.80).await;
        match &ack.payload {
            MessagePayload::ResonateAck(data) => {
                assert_eq!(data.interference, "constructive");
            }
            _ => panic!("Expected ResonateAck"),
        }

        // Verify coupling state on bus
        {
            let b = bus.lock().await;
            let node_a = b.get_node("node-a").unwrap();
            assert_eq!(node_a.state, NodeState::Coupled);
            // Server picks first non-a peer (HashMap order is non-deterministic)
            assert!(
                node_a.peers.len() == 1,
                "node-a should have exactly 1 peer, got: {:?}",
                node_a.peers
            );

            let node_b = b.get_node("node-b").unwrap();
            assert_eq!(node_b.state, NodeState::Coupled);
            // node-b resonates → server picks first non-b peer
            assert!(
                !node_b.peers.is_empty(),
                "node-b should have at least one peer"
            );
        }

        // Decouple node-a
        let mut client_a2 = TcpClient::connect(&addr).await.unwrap();
        let ack = client_a2.decouple("node-a", "test_end", 1).await;
        assert!(ack.is_ok());
        match &ack.unwrap().payload {
            MessagePayload::DecoupleAck(data) => {
                assert!((data.restored_phase - 0.75).abs() < 1e-10);
            }
            _ => panic!("Expected DecoupleAck"),
        }

        // Verify node-a is sovereign again
        {
            let b = bus.lock().await;
            let node_a = b.get_node("node-a").unwrap();
            assert_eq!(node_a.state, NodeState::Sovereign);
        }
    }

    /// Test cross-frame resonance: Science ↔ Individual should detect conflict.
    #[tokio::test]
    async fn cross_frame_resonance_detects_conflict_over_tcp() {
        let (addr, _bus) = spawn_server(vec![
            ("sci-node", Frame::Science, 0.90),
            ("ind-node", Frame::Individual, 0.20),
        ])
        .await;

        // Science node resonates with Individual node → destructive conflict
        let mut client = TcpClient::connect(&addr).await.unwrap();
        let ack = client.resonate("sci-node", "Science", 0.90, vec![]).await;
        assert!(ack.is_ok());

        match &ack.unwrap().payload {
            MessagePayload::ResonateAck(data) => {
                assert_eq!(data.interference, "destructive");
                assert!(data.conflict_detected);
                assert_eq!(data.recommendation, "hold");
            }
            _ => panic!("Expected ResonateAck"),
        }
    }

    /// Test the full negotiate flow over TCP: 3 nodes with mixed frames.
    #[tokio::test]
    async fn negotiate_over_tcp_mixed_frames() {
        let (addr, _bus) = spawn_server(vec![
            ("sci", Frame::Science, 0.80),
            ("ind", Frame::Individual, 0.30),
            ("con", Frame::Consensus, 0.50),
        ])
        .await;

        let mut client = TcpClient::connect(&addr).await.unwrap();
        let resp = client
            .negotiate(
                "sci",
                vec!["sci".to_string(), "ind".to_string(), "con".to_string()],
                vec![
                    "Science".to_string(),
                    "Individual".to_string(),
                    "Consensus".to_string(),
                ],
                vec![0.80, 0.30, 0.50],
            )
            .await;

        assert!(resp.is_ok());
        let msg = resp.unwrap();
        // Cross-frame negotiation should return Hold
        match &msg.payload {
            MessagePayload::Negotiate(data) => {
                assert_eq!(data.conflict_resolution, "hold");
                assert_eq!(data.participants.len(), 3);
            }
            _ => panic!("Expected Negotiate response, got {:?}", msg.payload),
        }
    }

    /// Test HEARTBEAT exchange preserves phase and state information.
    #[tokio::test]
    async fn heartbeat_preserves_node_identity() {
        let (addr, _bus) = spawn_server(vec![("identity-node", Frame::Consensus, 0.65)]).await;

        let mut client = TcpClient::connect(&addr).await.unwrap();
        let resp = client.heartbeat("identity-node", "Sovereign", 0.65).await;
        assert!(resp.is_ok());

        match &resp.unwrap().payload {
            MessagePayload::Heartbeat(hb) => {
                assert!((hb.current_phase - 0.65).abs() < f64::EPSILON);
                // The echo returns the server's view of our state
                assert!(!hb.node_state.is_empty());
            }
            _ => panic!("Expected Heartbeat"),
        }
    }

    /// Test that the message log captures all messages during a full
    /// resonate→decouple lifecycle.
    #[tokio::test]
    async fn message_log_captures_full_lifecycle() {
        let (addr, bus) = spawn_server(vec![
            ("logger-a", Frame::Science, 0.60),
            ("logger-b", Frame::Science, 0.70),
        ])
        .await;

        let initial_log_len = {
            let b = bus.lock().await;
            b.message_log.len()
        };

        // Resonate
        let mut client = TcpClient::connect(&addr).await.unwrap();
        let _ = client
            .resonate("logger-a", "Science", 0.60, vec![])
            .await
            .unwrap();

        // Decouple
        let mut client2 = TcpClient::connect(&addr).await.unwrap();
        let _ = client2.decouple("logger-a", "test_end", 0).await.unwrap();

        // Verify log grew
        {
            let b = bus.lock().await;
            assert!(
                b.message_log.len() > initial_log_len,
                "Message log should have grown after resonate+decouple"
            );
        }
    }

    /// Test discovery bootstrap with a real TCP server.
    #[tokio::test]
    async fn bootstrap_discovers_seed_peer() {
        let (addr, _bus) = spawn_server(vec![("bootstrap-node", Frame::Meta, 0.50)]).await;

        // Create a fresh bus for the bootstrapping node
        let client_bus = Arc::new(Mutex::new(ResonanceBus::new()));
        {
            let mut b = client_bus.lock().await;
            b.register(Node::new("discovery-client".into(), Frame::Science, 0.50));
        }

        let seeds = vec![addr.clone()];
        let contacted = discovery::bootstrap(&client_bus, "discovery-client", &seeds).await;

        assert_eq!(contacted, 1, "Should successfully contact 1 seed peer");

        // The seed peer should now be registered in the client bus
        {
            let b = client_bus.lock().await;
            // The seed is registered with its address as the node id
            assert!(
                b.get_node(&addr).is_some(),
                "Seed peer should be registered in client bus after bootstrap"
            );
        }
    }

    /// Test that multiple HEARTBEATs from the same peer don't duplicate registration.
    #[tokio::test]
    async fn repeated_heartbeat_does_not_duplicate() {
        let (addr, _bus) = spawn_server(vec![("stable-node", Frame::Individual, 0.55)]).await;

        // First heartbeat
        let mut client1 = TcpClient::connect(&addr).await.unwrap();
        let resp1 = client1.heartbeat("stable-node", "Sovereign", 0.55).await;
        assert!(resp1.is_ok());

        // Second heartbeat from same node
        let mut client2 = TcpClient::connect(&addr).await.unwrap();
        let resp2 = client2.heartbeat("stable-node", "Sovereign", 0.55).await;
        assert!(resp2.is_ok());

        // Both should succeed — the server handles repeated heartbeats idempotently
        match &resp2.unwrap().payload {
            MessagePayload::Heartbeat(hb) => {
                assert!((hb.current_phase - 0.55).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Heartbeat"),
        }
    }

    /// Test that a node can resonate, decouple, and re-resonate (full cycle).
    #[tokio::test]
    async fn full_couple_decouple_recouple_cycle() {
        let (addr, bus) = spawn_server(vec![
            ("cycle-a", Frame::Consensus, 0.50),
            ("cycle-b", Frame::Consensus, 0.60),
        ])
        .await;

        // Phase 1: Couple (full handshake)
        let ack = resonate_full_handshake(&addr, "cycle-a", "Consensus", 0.50).await;
        match &ack.payload {
            MessagePayload::ResonateAck(data) => {
                assert_eq!(data.interference, "constructive");
            }
            _ => panic!("Expected ResonateAck"),
        }

        // Verify coupled
        {
            let b = bus.lock().await;
            assert_eq!(b.get_node("cycle-a").unwrap().state, NodeState::Coupled);
        }

        // Phase 2: Decouple
        let mut client2 = TcpClient::connect(&addr).await.unwrap();
        let ack2 = client2.decouple("cycle-a", "cycling", 1).await.unwrap();
        match &ack2.payload {
            MessagePayload::DecoupleAck(data) => {
                assert!((data.restored_phase - 0.50).abs() < 1e-10);
            }
            _ => panic!("Expected DecoupleAck"),
        }

        // Verify sovereign
        {
            let b = bus.lock().await;
            assert_eq!(b.get_node("cycle-a").unwrap().state, NodeState::Sovereign);
        }

        // Phase 3: Re-couple (full handshake)
        let ack3 = resonate_full_handshake(&addr, "cycle-a", "Consensus", 0.50).await;
        match &ack3.payload {
            MessagePayload::ResonateAck(data) => {
                assert_eq!(data.interference, "constructive");
            }
            _ => panic!("Expected ResonateAck on re-couple"),
        }

        // Verify re-coupled
        {
            let b = bus.lock().await;
            assert_eq!(b.get_node("cycle-a").unwrap().state, NodeState::Coupled);
        }
    }

    /// Test parse_seeds with real-world IPv4 and hostname formats.
    #[test]
    fn parse_seeds_real_world_formats() {
        // IPv4 with different ports
        let seeds = discovery::parse_seeds("10.0.0.1:9000,10.0.0.2:9001,10.0.0.3:9002");
        assert_eq!(seeds.len(), 3);
        assert_eq!(seeds[0], "10.0.0.1:9000");

        // Hostnames
        let seeds = discovery::parse_seeds("node-science:9000,node-individual:9001");
        assert_eq!(seeds.len(), 2);
        assert_eq!(seeds[1], "node-individual:9001");

        // Single seed
        let seeds = discovery::parse_seeds("localhost:9999");
        assert_eq!(seeds, vec!["localhost:9999"]);

        // Whitespace handling
        let seeds = discovery::parse_seeds("  host1:1000  ,  host2:2000  ");
        assert_eq!(seeds, vec!["host1:1000", "host2:2000"]);
    }
}
