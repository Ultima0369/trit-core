// TCP transport server for Trit-Core distributed nodes.
//
// Each TcpNodeServer binds to a TCP port and accepts connections from
// peer nodes. Incoming messages are deserialized and dispatched to the
// local ResonanceBus for processing. Responses are sent back over the
// same connection.
//
// ## Architecture
//
// The server uses tokio's multi-threaded runtime. Each accepted
// connection is handled in a spawned task. The shared ResonanceBus
// is wrapped in Arc<Mutex<>> for concurrent access.
//
// ## Protocol flow
//
// Client → Server:  RESONATE_REQ / DECOUPLE_REQ / NEGOTIATE / HEARTBEAT
// Server → Client:  RESONATE_ACK / DECOUPLE_ACK / (negotiate result)

use crate::net::bus::ResonanceBus;
use crate::net::frame_codec;
use crate::net::message::{HeartbeatPayload, Message, MessagePayload};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Maximum idle time for a connection read (M7).
pub(crate) const READ_IDLE_TIMEOUT_SECS: u64 = 30;

/// A TCP-transport node server.
///
/// Wraps a ResonanceBus behind a TCP listener. Peers connect and send
/// protocol messages; the server processes them and sends responses.
///
/// ## Timeouts (M7)
///
/// - Read timeout: 30 seconds of idle time before connection is dropped.
pub struct TcpNodeServer {
    /// Shared bus for all connected peers.
    bus: Arc<Mutex<ResonanceBus>>,
    /// The address this server is bound to.
    bind_addr: String,
}

impl TcpNodeServer {
    /// Create a new server bound to the given address.
    pub fn new(bind_addr: impl Into<String>) -> Self {
        Self {
            bus: Arc::new(Mutex::new(ResonanceBus::new())),
            bind_addr: bind_addr.into(),
        }
    }

    /// Create a server with a pre-configured bus (for testing).
    pub fn with_bus(bind_addr: impl Into<String>, bus: ResonanceBus) -> Self {
        Self {
            bus: Arc::new(Mutex::new(bus)),
            bind_addr: bind_addr.into(),
        }
    }

    /// Get a clone of the shared bus handle.
    pub fn bus_handle(&self) -> Arc<Mutex<ResonanceBus>> {
        self.bus.clone()
    }

    /// Start listening and processing connections.
    ///
    /// This function runs until the listener is closed or a fatal error occurs.
    /// Each connection is handled in a spawned task.
    pub async fn serve(&self) -> std::io::Result<()> {
        let listener = TcpListener::bind(&self.bind_addr).await?;
        info!(addr = %self.bind_addr, "TcpNodeServer listening");

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!(peer = %peer_addr, "accepted connection");
                    let bus = self.bus.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, bus).await {
                            warn!(peer = %peer_addr, error = %e, "connection error");
                        }
                        debug!(peer = %peer_addr, "connection closed");
                    });
                }
                Err(e) => {
                    error!(error = %e, "accept failed");
                    // Continue accepting — transient errors shouldn't kill the server
                }
            }
        }
    }
}

/// Handle a single TCP connection: read frames, validate via gatekeeper (M8),
/// dispatch to bus, send responses.
async fn handle_connection(
    mut stream: TcpStream,
    bus: Arc<Mutex<ResonanceBus>>,
) -> std::io::Result<()> {
    let (mut reader, mut writer) = stream.split();

    loop {
        // Read a frame with idle timeout (M7)
        let payload = match tokio::time::timeout(
            Duration::from_secs(READ_IDLE_TIMEOUT_SECS),
            frame_codec::read_frame(&mut reader),
        )
        .await
        {
            Ok(Ok(p)) => p,
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(());
            }
            Ok(Err(e)) => return Err(e),
            Err(_elapsed) => {
                // Idle timeout — peer is likely disconnected or partitioned
                debug!("Connection idle timeout after {}s", READ_IDLE_TIMEOUT_SECS);
                return Ok(());
            }
        };

        let msg: Message = serde_json::from_slice(&payload)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        // M8: validate then dispatch within one lock scope
        let response = {
            let mut bus = bus.lock().await;
            match bus.validate_incoming(&msg) {
                Ok(()) => dispatch_message_inner(&mut bus, &msg),
                Err(rejection) => {
                    warn!(
                        sender = %msg.header.sender,
                        rejection = %rejection,
                        "Gatekeeper rejected message"
                    );
                    Some(build_rejection_response(&msg.header.sender, &rejection))
                }
            }
        };

        if let Some(resp) = response {
            let resp_json = serde_json::to_vec(&resp)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
            frame_codec::write_frame(&mut writer, &resp_json).await?;
        }
    }
}

/// Dispatch a previously-validated message to the bus (M8: lock already held).
fn dispatch_message_inner(bus: &mut ResonanceBus, msg: &Message) -> Option<Message> {
    let sender = &msg.header.sender;

    match &msg.payload {
        MessagePayload::ResonateReq(_req) => {
            let target_id = bus.nodes.keys().find(|id| *id != sender).cloned();

            if let Some(target) = target_id {
                debug!(from = %sender, to = %target, "dispatching RESONATE_REQ");
                bus.handle_resonate_req(sender, &target, msg)
            } else {
                warn!(sender = %sender, "RESONATE_REQ with no peer available");
                None
            }
        }
        MessagePayload::ResonateAck(_ack) => {
            debug!(node = %sender, "dispatching RESONATE_ACK");
            bus.handle_resonate_ack(sender, msg);
            None
        }
        MessagePayload::DecoupleReq(_req) => {
            let cycles = { bus.nodes.get(sender).map(|n| n.cycles_coupled).unwrap_or(0) };
            debug!(node = %sender, cycles = cycles, "dispatching DECOUPLE_REQ");
            Some(bus.handle_decouple_req(sender, msg, cycles))
        }
        MessagePayload::DecoupleAck(_ack) => {
            debug!(node = %sender, "received DECOUPLE_ACK");
            bus.message_log.push_back(msg.clone());
            None
        }
        MessagePayload::Negotiate(payload) => {
            debug!(participants = ?payload.participants, "dispatching NEGOTIATE");
            let (result, _has_conflict) = bus.negotiate(&payload.participants);
            let response = Message::negotiate(
                "tcp-server",
                payload.participants.clone(),
                payload.frames.clone(),
                payload.phases.clone(),
                if result.value == crate::trit::TritValue::Hold {
                    "hold"
                } else {
                    "commit_true"
                },
            );
            Some(response)
        }
        MessagePayload::Heartbeat(hb) => {
            debug!(
                node = %sender,
                state = %hb.node_state,
                phase = hb.current_phase,
                "received HEARTBEAT"
            );
            bus.record_heartbeat(sender);
            bus.message_log.push_back(msg.clone());
            let node = bus.nodes.get(sender);
            let state_str = node
                .map(|n| format!("{:?}", n.state))
                .unwrap_or_else(|| "unknown".to_string());
            let phase = node.map(|n| n.current_phase).unwrap_or(0.5);
            Some(Message::heartbeat("tcp-server", &state_str, phase))
        }
    }
}

/// Build a rejection response message (M8).
fn build_rejection_response(sender: &str, rejection: &crate::net::gate::GateRejection) -> Message {
    let error_msg = format!("{}", rejection);
    let mut msg = Message::heartbeat("tcp-server", "Sovereign", 0.5);
    msg.payload = MessagePayload::Heartbeat(HeartbeatPayload {
        node_state: format!("REJECTED:{}", error_msg),
        current_phase: 0.0,
    });
    msg.header.sender = format!("tcp-server[{}]", sender);
    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Frame;
    use crate::net::message::Message;
    use crate::net::node::Node;
    use tokio::net::TcpStream;

    async fn spawn_test_server() -> (String, Arc<Mutex<ResonanceBus>>) {
        // Bind to port 0 to let the OS assign a free port
        let server = TcpNodeServer::new("127.0.0.1:0");
        let bus = server.bus_handle();
        let _addr = server.bind_addr.clone();

        // Register a test node so there's a peer to resonate with
        {
            let mut b = bus.lock().await;
            b.register(Node::new("peer-a".into(), Frame::Science, 0.8));
        }

        tokio::spawn(async move {
            let _ = server.serve().await;
        });

        // Give the server a moment to bind
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // We need to know the actual bound port. Since we used port 0,
        // we need to extract it. For testing, we'll use a fixed port.
        // Re-bind with a known approach: spawn on a known port.
        drop(bus); // This doesn't actually drop the Arc, just our clone

        // Simpler approach: use a fixed port for tests
        let server = TcpNodeServer::new("127.0.0.1:0");
        let bus = server.bus_handle();

        {
            let mut b = bus.lock().await;
            b.register(Node::new("peer-a".into(), Frame::Science, 0.8));
        }

        // Bind and get the actual port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);

        let server = TcpNodeServer::new(&addr);
        let bus = server.bus_handle();

        {
            let mut b = bus.lock().await;
            b.register(Node::new("peer-a".into(), Frame::Science, 0.8));
        }

        tokio::spawn(async move {
            let _ = server.serve().await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        (addr, bus)
    }

    #[tokio::test]
    async fn server_binds_and_accepts_connection() {
        let (addr, _bus) = spawn_test_server().await;
        let stream = TcpStream::connect(&addr).await;
        assert!(stream.is_ok());
    }

    #[tokio::test]
    async fn heartbeat_roundtrip() {
        let (addr, bus) = spawn_test_server().await;

        // Register the test client so the echo returns the correct phase
        {
            let mut b = bus.lock().await;
            b.register(Node::new("test-client".into(), Frame::Science, 0.75));
        }

        let mut stream = TcpStream::connect(&addr).await.unwrap();

        let msg = Message::heartbeat("test-client", "Sovereign", 0.75);
        let json = serde_json::to_vec(&msg).unwrap();
        frame_codec::write_frame(&mut stream, &json).await.unwrap();

        // Read response
        let (mut reader, _writer) = stream.split();
        let resp_payload = frame_codec::read_frame(&mut reader).await.unwrap();
        let resp: Message = serde_json::from_slice(&resp_payload).unwrap();

        match &resp.payload {
            MessagePayload::Heartbeat(hb) => {
                assert!((hb.current_phase - 0.75).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Heartbeat response"),
        }
    }

    #[tokio::test]
    async fn resonate_req_gets_ack() {
        let (addr, bus) = spawn_test_server().await;

        // Register the client node so the server knows about it
        {
            let mut b = bus.lock().await;
            b.register(Node::new("test-client".into(), Frame::Science, 0.7));
        }

        let mut stream = TcpStream::connect(&addr).await.unwrap();

        let msg = Message::resonate_req("test-client", "Science", 0.7, vec![]);
        let json = serde_json::to_vec(&msg).unwrap();
        frame_codec::write_frame(&mut stream, &json).await.unwrap();

        let (mut reader, _writer) = stream.split();
        let resp_payload = frame_codec::read_frame(&mut reader).await.unwrap();
        let resp: Message = serde_json::from_slice(&resp_payload).unwrap();

        match &resp.payload {
            MessagePayload::ResonateAck(data) => {
                assert_eq!(data.interference, "constructive");
                assert!(!data.conflict_detected);
            }
            _ => panic!("Expected ResonateAck, got {:?}", resp.payload),
        }
    }

    #[tokio::test]
    async fn decouple_req_gets_ack() {
        let (addr, bus) = spawn_test_server().await;

        {
            let mut b = bus.lock().await;
            b.register(Node::new("test-client".into(), Frame::Science, 0.7));
        }

        let mut stream = TcpStream::connect(&addr).await.unwrap();

        let msg = Message::decouple_req("test-client", "user_disconnect");
        let json = serde_json::to_vec(&msg).unwrap();
        frame_codec::write_frame(&mut stream, &json).await.unwrap();

        let (mut reader, _writer) = stream.split();
        let resp_payload = frame_codec::read_frame(&mut reader).await.unwrap();
        let resp: Message = serde_json::from_slice(&resp_payload).unwrap();

        match &resp.payload {
            MessagePayload::DecoupleAck(data) => {
                assert!((data.restored_phase - 0.7).abs() < f64::EPSILON);
            }
            _ => panic!("Expected DecoupleAck, got {:?}", resp.payload),
        }
    }
}
