// Byzantine fault-tolerance tests for Trit-Core (M8).
//
// These tests validate the ByzantineGatekeeper in a real TCP environment
// using tokio's port 0 binding for OS-assigned ports. Each test verifies
// that the gatekeeper correctly rejects malformed/malicious messages
// before they reach the ResonanceBus.
//
// ## Test Strategy
//
// We use the same spawn_server pattern as partition_test.rs and
// multi_node_test.rs, but create servers with a ByzantineGatekeeper
// configured. This exercises the full TCP → deserialize → validate →
// dispatch pipeline.

#[cfg(test)]
mod byzantine_tests {
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use trit_core::frame::Frame;
    use trit_core::net::bus::ResonanceBus;
    use trit_core::net::gate::ByzantineGatekeeper;
    use trit_core::net::message::{Message, MessagePayload};
    use trit_core::net::node::Node;
    use trit_core::net::tcp_client::TcpClient;
    use trit_core::net::tcp_server::TcpNodeServer;

    /// Spawn a TCP server with a Byzantine gatekeeper and pre-registered nodes.
    async fn spawn_gatekept_server(
        nodes: Vec<(&str, Frame, f64)>,
        gk: ByzantineGatekeeper,
    ) -> (String, Arc<Mutex<ResonanceBus>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);

        let mut bus = ResonanceBus::with_gatekeeper(gk);
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

    /// Spawn a server WITHOUT a gatekeeper (for baseline comparison).
    async fn spawn_ungatekept_server(
        nodes: Vec<(&str, Frame, f64)>,
    ) -> (String, Arc<Mutex<ResonanceBus>>) {
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

    /// Send a raw message over a TCP connection and read the response.
    async fn send_raw_message(addr: &str, msg: &Message) -> Option<Message> {
        let mut client = TcpClient::connect(addr).await.unwrap();
        match msg.payload {
            MessagePayload::Heartbeat(ref hb) => client
                .heartbeat(&msg.header.sender, &hb.node_state, hb.current_phase)
                .await
                .ok(),
            MessagePayload::ResonateReq(ref req) => client
                .resonate(
                    &msg.header.sender,
                    &req.frame,
                    req.phase,
                    req.history.clone(),
                )
                .await
                .ok(),
            MessagePayload::Negotiate(ref payload) => client
                .negotiate(
                    &msg.header.sender,
                    payload.participants.clone(),
                    payload.frames.clone(),
                    payload.phases.clone(),
                )
                .await
                .ok(),
            _ => None,
        }
    }

    /// Check that a response is a rejection (REJECTED prefix in node_state).
    fn is_rejection_response(resp: &Message) -> bool {
        match &resp.payload {
            MessagePayload::Heartbeat(hb) => hb.node_state.starts_with("REJECTED:"),
            _ => false,
        }
    }

    // ── Test 1: Fake heartbeat with out-of-range phase is rejected ──

    #[tokio::test]
    async fn fake_heartbeat_out_of_range_phase_rejected() {
        let (addr, _bus) = spawn_gatekept_server(
            vec![("legit-node", Frame::Science, 0.5)],
            ByzantineGatekeeper::default(),
        )
        .await;

        // Send a heartbeat with phase 1.5 (out of range)
        let msg = Message::heartbeat("legit-node", "Sovereign", 1.5);
        let resp = send_raw_message(&addr, &msg).await;

        assert!(resp.is_some(), "Should get a response (rejection)");
        let resp = resp.unwrap();
        assert!(
            is_rejection_response(&resp),
            "Expected rejection response for phase 1.5, got: {:?}",
            resp.payload
        );
    }

    // ── Test 2: Valid heartbeat passes gatekeeper ──

    #[tokio::test]
    async fn valid_heartbeat_passes_gatekeeper() {
        let (addr, _bus) = spawn_gatekept_server(
            vec![("good-node", Frame::Science, 0.75)],
            ByzantineGatekeeper::default(),
        )
        .await;

        // Send a valid heartbeat
        let msg = Message::heartbeat("good-node", "Sovereign", 0.75);
        let resp = send_raw_message(&addr, &msg).await;

        assert!(resp.is_some(), "Should get a response");
        let resp = resp.unwrap();
        assert!(
            !is_rejection_response(&resp),
            "Valid heartbeat should not be rejected, got: {:?}",
            resp.payload
        );
    }

    // ── Test 3: Unknown sender is rejected ──

    #[tokio::test]
    async fn unknown_sender_rejected() {
        let (addr, _bus) = spawn_gatekept_server(
            vec![("known-node", Frame::Science, 0.5)],
            ByzantineGatekeeper::default(),
        )
        .await;

        // Send from an unregistered sender
        let msg = Message::heartbeat("intruder-node", "Sovereign", 0.5);
        let resp = send_raw_message(&addr, &msg).await;

        assert!(resp.is_some(), "Should get a response (rejection)");
        let resp = resp.unwrap();
        assert!(
            is_rejection_response(&resp),
            "Expected rejection for unknown sender, got: {:?}",
            resp.payload
        );
    }

    // ── Test 4: Malformed frame in ResonateReq is rejected ──

    #[tokio::test]
    async fn malformed_frame_in_resonate_req_rejected() {
        let (addr, _bus) = spawn_gatekept_server(
            vec![("science-node", Frame::Science, 0.7)],
            ByzantineGatekeeper::default(),
        )
        .await;

        // Send RESONATE_REQ with invalid frame
        let msg = Message::resonate_req("science-node", "__MALICIOUS__", 0.5, vec![]);
        let resp = send_raw_message(&addr, &msg).await;

        assert!(resp.is_some(), "Should get a response (rejection)");
        let resp = resp.unwrap();
        assert!(
            is_rejection_response(&resp),
            "Expected rejection for malformed frame, got: {:?}",
            resp.payload
        );
    }

    // ── Test 5: Phase manipulation via crafted Negotiate is rejected ──

    #[tokio::test]
    async fn phase_manipulation_via_crafted_negotiate_rejected() {
        let (addr, _bus) = spawn_gatekept_server(
            vec![
                ("node-a", Frame::Science, 0.5),
                ("node-b", Frame::Individual, 0.5),
            ],
            ByzantineGatekeeper::default(),
        )
        .await;

        // Send NEGOTIATE with bad phase values
        let msg = Message::negotiate(
            "node-a",
            vec!["node-a".into(), "node-b".into()],
            vec!["Science".into(), "Individual".into()],
            vec![2.0, -0.5], // both out of range
            "hold",
        );
        let resp = send_raw_message(&addr, &msg).await;

        assert!(resp.is_some(), "Should get a response (rejection)");
        let resp = resp.unwrap();
        // Negotiate rejection comes back as a Heartbeat with REJECTED
        assert!(
            is_rejection_response(&resp),
            "Expected rejection for bad negotiate phases, got: {:?}",
            resp.payload
        );
    }

    // ── Test 6: Rapid message flood is rate-limited ──

    #[tokio::test]
    async fn rapid_resonate_flood_rate_limited() {
        // Create gatekeeper with tight rate limit: 5 messages per 60 seconds
        let gk = ByzantineGatekeeper::new(5, 60, 1000);
        let (addr, _bus) = spawn_gatekept_server(vec![("flooder", Frame::Science, 0.5)], gk).await;

        // Send 10 rapid messages
        let mut rejections = 0u32;
        let mut passes = 0u32;
        for i in 0..10 {
            let msg = Message::heartbeat("flooder", "Sovereign", 0.5);
            let resp = send_raw_message(&addr, &msg).await;

            if let Some(resp) = resp {
                if is_rejection_response(&resp) {
                    rejections += 1;
                } else {
                    passes += 1;
                }
            }
            let _ = i;
        }

        assert!(
            rejections > 0,
            "Expected at least one rate-limit rejection in 10 rapid messages"
        );
        assert!(
            passes > 0,
            "Expected at least some messages to pass before rate limit"
        );
        assert!(
            passes <= 5,
            "At most 5 messages should pass before rate limit kicks in"
        );
    }

    // ── Test 7: Without gatekeeper, invalid messages are NOT rejected ──

    #[tokio::test]
    async fn without_gatekeeper_invalid_messages_pass_through() {
        // This test verifies that the gatekeeper is truly optional —
        // without it, the same invalid messages reach the bus.
        let (addr, _bus) = spawn_ungatekept_server(vec![("node1", Frame::Science, 0.5)]).await;

        // Send heartbeat with phase 1.5 — should NOT be rejected
        let msg = Message::heartbeat("node1", "Sovereign", 1.5);
        let resp = send_raw_message(&addr, &msg).await;

        assert!(
            resp.is_some(),
            "Should get a response even without gatekeeper"
        );
        let resp = resp.unwrap();
        assert!(
            !is_rejection_response(&resp),
            "Without gatekeeper, phase 1.5 should NOT be rejected (no validation)"
        );
    }
}
