// Multi-threaded concurrency stress tests for Trit-Core (M9).
//
// These tests validate the ResonanceBus under concurrent access:
// - Multiple TCP clients hammering a single server
// - Concurrent register/heartbeat/dispatch from spawned tasks
// - No deadlocks, no data races, no state corruption
// - Bus state consistency after concurrent operations
//
// These tests use #[tokio::test] with multi-threaded runtime to
// exercise the Arc<tokio::sync::Mutex<ResonanceBus>> architecture.

#[cfg(test)]
mod concurrency_tests {
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use trit_core::frame::Frame;
    use trit_core::net::bus::ResonanceBus;
    use trit_core::net::node::Node;

    /// Spawn a TCP server with pre-registered nodes.
    async fn spawn_server(nodes: Vec<(&str, Frame, f64)>) -> (String, Arc<Mutex<ResonanceBus>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);

        let mut bus = ResonanceBus::new();
        for (id, frame, phase) in &nodes {
            bus.register(Node::new(id.to_string(), frame.clone(), *phase));
        }

        let server = trit_core::net::tcp_server::TcpNodeServer::with_bus(&addr, bus);
        let bus_handle = server.bus_handle();

        tokio::spawn(async move {
            let _ = server.serve().await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        (addr, bus_handle)
    }

    // ── Test 1: Concurrent heartbeat flood ──

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_heartbeat_flood() {
        let (addr, bus) = spawn_server(vec![
            ("node-a", Frame::Science, 0.5),
            ("node-b", Frame::Individual, 0.6),
            ("node-c", Frame::Consensus, 0.7),
        ])
        .await;

        let client_count = 20;
        let heartbeats_per_client = 50;
        let mut handles = Vec::new();

        for i in 0..client_count {
            let addr = addr.clone();
            let node_char = match i % 3 {
                0 => 'a',
                1 => 'b',
                _ => 'c',
            };
            let node_id = format!("node-{}", node_char);
            handles.push(tokio::spawn(async move {
                let mut client = trit_core::net::tcp_client::TcpClient::connect(&addr)
                    .await
                    .unwrap();
                for j in 0..heartbeats_per_client {
                    let phase = 0.3 + ((j as f64) * 0.01) % 0.4;
                    let _ = client.heartbeat(&node_id, "Coupled", phase).await;
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Verify bus state is consistent
        let bus = bus.lock().await;
        assert_eq!(bus.nodes.len(), 3, "Should still have 3 registered nodes");
        for id in &["node-a", "node-b", "node-c"] {
            assert!(bus.nodes.contains_key(*id), "Node {} should exist", id);
            assert!(
                bus.last_heartbeat.contains_key(*id),
                "Node {} should have a heartbeat timestamp",
                id
            );
        }
        // Message log should have grown
        assert!(
            !bus.message_log.is_empty(),
            "Message log should contain heartbeats"
        );
    }

    // ── Test 2: Concurrent register and access ──

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_register_and_access() {
        let bus = Arc::new(Mutex::new(ResonanceBus::new()));
        let register_count = 50;
        let mut handles = Vec::new();

        // Spawn register tasks
        for i in 0..register_count {
            let bus = bus.clone();
            handles.push(tokio::spawn(async move {
                let node_id = format!("node-{}", i);
                let node = Node::new(node_id.clone(), Frame::Science, 0.5);
                let mut bus = bus.lock().await;
                bus.register(node);
                node_id
            }));
        }

        let mut registered_ids = Vec::new();
        for handle in handles {
            registered_ids.push(handle.await.unwrap());
        }

        // All should be registered (up to MAX_NODES=256)
        let bus = bus.lock().await;
        assert_eq!(
            bus.nodes.len(),
            register_count,
            "All {} nodes should be registered",
            register_count
        );
        for id in &registered_ids {
            assert!(bus.nodes.contains_key(id), "Node {} should exist", id);
        }
    }

    // ── Test 3: Concurrent bus operations directly (no TCP) ──

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_bus_operations_no_deadlock() {
        let bus = Arc::new(Mutex::new(ResonanceBus::new()));

        // Pre-register nodes
        {
            let mut bus = bus.lock().await;
            for id in &["a", "b", "c", "d", "e"] {
                bus.register(Node::new(id.to_string(), Frame::Science, 0.5));
            }
        }

        let mut handles = Vec::new();
        let iterations = 200;

        // Heartbeat tasks
        for i in 0..5 {
            let bus = bus.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..iterations {
                    let mut bus = bus.lock().await;
                    let node_id_actual = match i % 5 {
                        0 => "a",
                        1 => "b",
                        2 => "c",
                        3 => "d",
                        _ => "e",
                    };
                    bus.record_heartbeat(node_id_actual);
                    // Use record_heartbeat to exercise the bus; message_log
                    // is populated via TCP dispatch in integration tests
                }
            }));
        }

        // Negotiate tasks
        for _ in 0..3 {
            let bus = bus.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..iterations / 2 {
                    let mut bus = bus.lock().await;
                    let _ = bus.negotiate(&["a".into(), "b".into(), "c".into()]);
                }
            }));
        }

        // Reader tasks
        for _ in 0..2 {
            let bus = bus.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..iterations {
                    let bus = bus.lock().await;
                    let _ = bus.nodes.len();
                    let _ = bus.stale_peers();
                    let _ = bus.detect_split_brain();
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Verify consistency
        let bus = bus.lock().await;
        assert_eq!(bus.nodes.len(), 5, "All 5 nodes should still be registered");
        for id in &["a", "b", "c", "d", "e"] {
            assert!(bus.last_heartbeat.contains_key(*id));
        }
        assert!(!bus.message_log.is_empty());
        // No panics = no deadlocks
    }

    // ── Test 4: Concurrent register + purge ──

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_register_and_purge() {
        let bus = Arc::new(Mutex::new(ResonanceBus::new()));
        let node_count = 40;
        let mut handles = Vec::new();

        // Register many nodes concurrently
        for i in 0..node_count {
            let bus = bus.clone();
            handles.push(tokio::spawn(async move {
                let id = format!("peer-{}", i);
                let mut bus = bus.lock().await;
                bus.register(Node::new(id.clone(), Frame::Science, 0.5));
                bus.record_heartbeat(&id);
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all registered
        {
            let bus = bus.lock().await;
            assert_eq!(bus.nodes.len(), node_count as usize);
        }

        // Purge half concurrently
        let mut purge_handles = Vec::new();
        for i in 0..node_count / 2 {
            let bus = bus.clone();
            purge_handles.push(tokio::spawn(async move {
                let id = format!("peer-{}", i);
                let mut bus = bus.lock().await;
                bus.purge_node(&id);
            }));
        }

        for handle in purge_handles {
            handle.await.unwrap();
        }

        let bus = bus.lock().await;
        assert_eq!(
            bus.nodes.len(),
            (node_count / 2) as usize,
            "Half the nodes should remain after purging"
        );
    }

    // ── Test 5: Concurrent TCP clients with gatekeeper ──

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_clients_with_gatekeeper() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);

        let mut bus = ResonanceBus::with_default_gatekeeper();
        for (id, frame, phase) in &[
            ("legit-a", Frame::Science, 0.5),
            ("legit-b", Frame::Individual, 0.6),
        ] {
            bus.register(Node::new(id.to_string(), frame.clone(), *phase));
        }

        let server = trit_core::net::tcp_server::TcpNodeServer::with_bus(&addr, bus);
        let bus_handle = server.bus_handle();

        tokio::spawn(async move {
            let _ = server.serve().await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut handles = Vec::new();

        // Legit clients
        for i in 0..10 {
            let addr = addr.clone();
            handles.push(tokio::spawn(async move {
                let node_id = if i % 2 == 0 { "legit-a" } else { "legit-b" };
                let mut client = trit_core::net::tcp_client::TcpClient::connect(&addr)
                    .await
                    .unwrap();
                for j in 0..20 {
                    let phase = 0.4 + (j as f64 * 0.02) % 0.2;
                    let _ = client.heartbeat(node_id, "Coupled", phase).await;
                }
            }));
        }

        // Byzantine clients (should be rejected)
        for _ in 0..5 {
            let addr = addr.clone();
            handles.push(tokio::spawn(async move {
                let mut client = trit_core::net::tcp_client::TcpClient::connect(&addr)
                    .await
                    .unwrap();
                // Send with unknown sender
                let _ = client.heartbeat("intruder", "Sovereign", 1.5).await;
                // Send with bad phase
                let _ = client.heartbeat("legit-a", "Sovereign", f64::NAN).await;
            }));
        }

        for handle in handles {
            let _ = handle.await;
        }

        // Verify bus still consistent
        let bus = bus_handle.lock().await;
        assert_eq!(bus.nodes.len(), 2, "Only legit nodes should be registered");
        assert!(
            !bus.message_log.is_empty(),
            "Message log should contain legit messages"
        );
    }

    // ── Test 6: Concurrent negotiate under load ──

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_negotiate_under_load() {
        let bus = Arc::new(Mutex::new(ResonanceBus::new()));

        // Register 8 nodes with mixed frames
        {
            let mut bus = bus.lock().await;
            let configs = [
                ("n0", Frame::Science),
                ("n1", Frame::Science),
                ("n2", Frame::Individual),
                ("n3", Frame::Individual),
                ("n4", Frame::Consensus),
                ("n5", Frame::Consensus),
                ("n6", Frame::Absolute),
                ("n7", Frame::Absolute),
            ];
            for (id, frame) in &configs {
                bus.register(Node::new(
                    id.to_string(),
                    frame.clone(),
                    0.5 + 0.05 * (id.as_bytes()[1] - b'0') as f64,
                ));
                bus.record_heartbeat(id);
            }
        }

        let mut handles = Vec::new();

        // Concurrent negotiate with different participant groups
        let groups = [
            vec!["n0", "n1", "n2"],
            vec!["n3", "n4", "n5"],
            vec!["n6", "n7", "n0"],
            vec!["n1", "n3", "n6"],
            vec!["n2", "n4", "n7"],
            vec!["n0", "n5", "n7"],
        ];

        for group in &groups {
            let bus = bus.clone();
            let participants: Vec<String> = group.iter().map(|s| s.to_string()).collect();
            handles.push(tokio::spawn(async move {
                for _ in 0..50 {
                    let mut bus = bus.lock().await;
                    let _ = bus.negotiate(&participants);
                }
            }));
        }

        // Concurrent heartbeats
        for i in 0..8 {
            let bus = bus.clone();
            let node_id = format!("n{}", i);
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    let mut bus = bus.lock().await;
                    bus.record_heartbeat(&node_id);
                    // record_heartbeat exercises the bus under concurrency;
                    // message_log is populated via TCP dispatch in integration tests
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let bus = bus.lock().await;
        assert_eq!(bus.nodes.len(), 8, "All nodes should survive");
    }

    // ── Test 7: Bus state snapshot consistency ──

    #[tokio::test(flavor = "multi_thread")]
    async fn bus_state_consistent_under_concurrent_mutation() {
        let bus = Arc::new(Mutex::new(ResonanceBus::new()));

        // Register 10 nodes
        {
            let mut bus = bus.lock().await;
            for i in 0..10 {
                let id = format!("n{}", i);
                bus.register(Node::new(id, Frame::Science, 0.5));
            }
        }

        let mut handles = Vec::new();

        // Mutators: heartbeat + log push
        for i in 0..5 {
            let bus = bus.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    let mut bus = bus.lock().await;
                    for j in 0..10 {
                        let id = format!("n{}", (i + j) % 10);
                        bus.record_heartbeat(&id);
                    }
                }
            }));
        }

        // Readers: snapshot queries
        for _ in 0..3 {
            let bus = bus.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    let bus = bus.lock().await;
                    let count = bus.nodes.len();
                    let stale = bus.stale_peers();
                    let split = bus.detect_split_brain();
                    // Basic invariants
                    assert!(count <= 10);
                    assert!(stale.len() <= 10, "stale_peers can't exceed total nodes");
                    assert!(split.len() <= 5, "split pairs can't exceed n/2");
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let bus = bus.lock().await;
        assert_eq!(bus.nodes.len(), 10);
        for i in 0..10 {
            let id = format!("n{}", i);
            assert!(bus.last_heartbeat.contains_key(&id), "n{} has heartbeat", i);
        }
    }
}
