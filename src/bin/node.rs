use std::io::{self, BufRead, Write};
use std::str::FromStr;

use trit_core::frame::Frame;
use trit_core::net::bus::ResonanceBus;
use trit_core::net::discovery;
use trit_core::net::message::Message;
use trit_core::net::node::Node;
use trit_core::net::tcp_server::TcpNodeServer;

/// Display available commands.
fn help() {
    println!("Commands:");
    println!("  resonate <peer_id>  — send RESONATE_REQ to a peer");
    println!("  decouple            — send DECOUPLE_REQ to break coupling");
    println!("  negotiate <ids...>  — run multi-node negotiation");
    println!("  status              — show current node state");
    println!("  peers               — list coupled peers");
    println!("  log                 — show message log (last 10)");
    println!("  help                — show this help");
    println!("  quit                — exit the node");
}

/// Print the current node status line.
fn print_status(node: &Node) {
    println!(
        "Node {} | frame={} | state={:?} | phase={:.4} (sovereign={:.4}) | peers={} | cycles={}",
        node.id,
        node.frame,
        node.state,
        node.current_phase,
        node.sovereign_phase,
        node.peers.len(),
        node.cycles_coupled
    );
}

#[tokio::main]
async fn main() {
    trit_core::tracing_init::init();

    let args: Vec<String> = std::env::args().collect();

    // Parse --frame and --phase (required)
    let frame = parse_flag(&args, "--frame", "Science");
    let phase = parse_flag_f64(&args, "--phase", 0.5);
    let node_id = parse_flag(&args, "--id", &uuid::Uuid::new_v4().to_string());
    let bind_port = parse_flag_u16(&args, "--port", 9000);
    let peers_str = parse_flag(&args, "--peers", "");

    // Also check TRIT_PEERS env var
    let peers_env = std::env::var("TRIT_PEERS").unwrap_or_default();
    let all_peers = if peers_str.is_empty() {
        peers_env
    } else {
        peers_str
    };

    let frame: Frame = match Frame::from_str(&frame) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Invalid frame: {} — {}", frame, e);
            std::process::exit(1);
        }
    };

    if !(0.0..=1.0).contains(&phase) {
        eprintln!("Phase must be in [0.0, 1.0], got: {}", phase);
        std::process::exit(1);
    }

    let bind_addr = format!("0.0.0.0:{}", bind_port);

    println!("Trit-Core Node v0.1.0 (M7 — TCP + Partition Tolerance)");
    println!("Node ID: {}", node_id);
    println!("Frame:   {}", frame);
    println!("Phase:   {:.4}", phase);
    println!("Bind:    {}", bind_addr);

    // Parse seed peers
    let seeds = discovery::parse_seeds(&all_peers);
    if !seeds.is_empty() {
        println!("Seeds:   {}", seeds.join(", "));
    } else {
        println!("Seeds:   (none — standalone mode)");
    }
    println!();

    // Create bus and register local node
    let mut bus = ResonanceBus::new();
    bus.register(Node::new(node_id.clone(), frame, phase));

    // Start TCP server
    let server = TcpNodeServer::with_bus(&bind_addr, bus);
    let bus_handle = server.bus_handle();

    // Spawn the server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.serve().await {
            eprintln!("[Server] Fatal error: {}", e);
        }
    });

    // Give the server a moment to bind
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Bootstrap: connect to seed peers
    let contacted = discovery::bootstrap(&bus_handle, &node_id, &seeds).await;
    if contacted > 0 {
        println!(
            "[Discovery] Connected to {}/{} seed peers.\n",
            contacted,
            seeds.len()
        );
    } else if !seeds.is_empty() {
        println!("[Discovery] Warning: Could not connect to any seed peers. Running standalone.\n");
    }

    println!("Type 'help' for commands.\n");

    // Interactive REPL
    let stdin = io::stdin();
    let reader = stdin.lock();
    print!("trit> ");
    io::stdout().flush().unwrap();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l.trim().to_string(),
            Err(_) => break,
        };

        if line.is_empty() {
            print!("trit> ");
            io::stdout().flush().unwrap();
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts[0] {
            "quit" | "exit" => {
                println!("Shutting down...");
                break;
            }

            "help" => help(),

            "status" => {
                let bus = bus_handle.lock().await;
                if let Some(node) = bus.get_node(&node_id) {
                    print_status(node);
                }
            }

            "peers" => {
                let bus = bus_handle.lock().await;
                if let Some(node) = bus.get_node(&node_id) {
                    if node.peers.is_empty() {
                        println!("No coupled peers.");
                    } else {
                        println!("Coupled peers ({}):", node.peers.len());
                        for peer_id in &node.peers {
                            if let Some(peer) = bus.get_node(peer_id) {
                                println!(
                                    "  {} — frame={}, phase={:.4}, state={:?}",
                                    peer.id, peer.frame, peer.current_phase, peer.state
                                );
                            } else {
                                println!("  {} — (unknown)", peer_id);
                            }
                        }
                    }
                }
            }

            "log" => {
                let bus = bus_handle.lock().await;
                let log = bus.log();
                let log_len = log.len();
                let start = log_len.saturating_sub(10);
                if log_len == 0 {
                    println!("(empty log)");
                } else {
                    for (i, msg) in log.skip(start).enumerate() {
                        println!("  [{}] {:?}", start + i + 1, msg.header.msg_id);
                        println!("    sender={}", msg.header.sender);
                    }
                }
                println!("Total messages: {}", log_len);
            }

            "resonate" => {
                if parts.len() < 2 {
                    eprintln!("Usage: resonate <peer_id>");
                } else {
                    let peer_id = parts[1];
                    let mut bus = bus_handle.lock().await;
                    let req = {
                        let node = bus.get_node(&node_id).unwrap();
                        Message::resonate_req(
                            &node.id,
                            &format!("{}", node.frame),
                            node.current_phase,
                            vec![],
                        )
                    };
                    match bus.handle_resonate_req(&node_id, peer_id, &req) {
                        Some(ack) => {
                            println!(
                                "→ RESONATE_ACK received from {}: interference={}",
                                peer_id,
                                match &ack.payload {
                                    trit_core::net::message::MessagePayload::ResonateAck(ref d) =>
                                        d.interference.as_str(),
                                    _ => "?",
                                }
                            );
                            bus.handle_resonate_ack(&node_id, &ack);
                            println!("✓ Coupling confirmed.");
                        }
                        None => {
                            eprintln!("✗ Failed: peer '{}' not found on bus.", peer_id);
                        }
                    }
                }
            }

            "decouple" => {
                let mut bus = bus_handle.lock().await;
                let cycles = {
                    let node = bus.get_node(&node_id).unwrap();
                    node.cycles_coupled
                };
                let req = Message::decouple_req(&node_id, "user_disconnect");
                let ack = bus.handle_decouple_req(&node_id, &req, cycles);
                println!(
                    "→ DECOUPLE_ACK: phase restored to {:.4} after {} cycles",
                    match &ack.payload {
                        trit_core::net::message::MessagePayload::DecoupleAck(ref d) =>
                            d.restored_phase,
                        _ => 0.0,
                    },
                    cycles
                );
                println!("✓ Decoupled. Node is now Sovereign.");
            }

            "negotiate" => {
                if parts.len() < 2 {
                    eprintln!("Usage: negotiate <id1> <id2> ...");
                } else {
                    let mut bus = bus_handle.lock().await;
                    let participant_ids: Vec<String> =
                        parts[1..].iter().map(|s| s.to_string()).collect();
                    let (result, has_conflict) = bus.negotiate(&participant_ids);
                    println!(
                        "Negotiation result: {:?} (phase={:.4}, conflict={})",
                        result.value,
                        result.phase.inner(),
                        has_conflict
                    );
                }
            }

            _ => {
                eprintln!("Unknown command: '{}'. Type 'help'.", parts[0]);
            }
        }

        print!("trit> ");
        io::stdout().flush().unwrap();
    }

    // Shutdown
    server_handle.abort();
    println!("Node shut down cleanly.");
}

// -- CLI helpers --

fn parse_flag(args: &[String], flag: &str, default: &str) -> String {
    args.iter()
        .position(|a| a == flag)
        .map(|i| args[i + 1].clone())
        .unwrap_or_else(|| default.to_string())
}

fn parse_flag_f64(args: &[String], flag: &str, default: f64) -> f64 {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_flag_u16(args: &[String], flag: &str, default: u16) -> u16 {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
