use std::io::{self, BufRead, Write};
use std::str::FromStr;

use trit_core::frame::Frame;
use trit_core::net::bus::ResonanceBus;
use trit_core::net::message::Message;
use trit_core::net::node::Node;

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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 || args[1] != "--frame" || args[3] != "--phase" {
        eprintln!("Usage: trit-node --frame <Science|Individual|Consensus|Absolute> --phase <0.0-1.0> [--id <name>]");
        std::process::exit(1);
    }

    let frame: Frame = match Frame::from_str(&args[2]) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Invalid frame: {} — {}", args[2], e);
            std::process::exit(1);
        }
    };

    let phase: f64 = match args[4].parse() {
        Ok(p) if (0.0..=1.0).contains(&p) => p,
        Ok(p) => {
            eprintln!("Phase must be in [0.0, 1.0], got: {}", p);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Invalid phase: {} — {}", args[4], e);
            std::process::exit(1);
        }
    };

    // Optional --id flag
    let node_id = args
        .iter()
        .position(|a| a == "--id")
        .map(|i| args[i + 1].clone())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let node = Node::new(node_id.clone(), frame, phase);
    println!("Trit-Core Node v0.1.0");
    println!("Node ID: {}", node_id);
    println!("Frame:   {}", node.frame);
    println!("Phase:   {:.4}", node.sovereign_phase);
    println!("Type 'help' for commands.\n");

    let mut bus = ResonanceBus::new();
    bus.register(node);

    // Register preset peers for simulation (all same-bus local)
    // In real deployment these would be discovered via network
    println!("[Bus] Node registered. ResonanceBus ready.\n");

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
                if let Some(node) = bus.get_node(&node_id) {
                    print_status(node);
                }
            }

            "peers" => {
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
                let log = bus.log();
                let start = if log.len() > 10 { log.len() - 10 } else { 0 };
                if log.is_empty() {
                    println!("(empty log)");
                } else {
                    for (i, msg) in log[start..].iter().enumerate() {
                        println!("  [{}] {:?}", start + i + 1, msg.header.msg_id);
                        println!("    sender={}", msg.header.sender);
                    }
                }
                println!("Total messages: {}", log.len());
            }

            "resonate" => {
                if parts.len() < 2 {
                    eprintln!("Usage: resonate <peer_id>");
                } else {
                    let peer_id = parts[1];
                    let req = {
                        let node = bus.get_node(&node_id).unwrap();
                        Message::resonate_req(
                            &node.id,
                            &format!("{}", node.frame),
                            node.current_phase,
                            vec![], // no history in MVP
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

    println!("Node shut down cleanly.");
}
