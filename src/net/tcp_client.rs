// TCP client connector for Trit-Core distributed nodes.
//
// TcpClient connects to a remote TcpNodeServer, sends protocol messages,
// and reads responses. It supports multi-message sessions with buffered
// I/O — unlike the earlier MVP version that could only handle one request
// per connection.
//
// ## Usage
//
// ```rust,ignore
// let mut client = TcpClient::connect("127.0.0.1:9000").await?;
// let ack = client.resonate("my-node", "Science", 0.7, vec![]).await?;
// client.decouple("my-node", "user_disconnect", 0).await?;
// ```
//
// ## Timeouts
//
// - Connect timeout: 5 seconds (M7)
// - Read timeout: 30 seconds (M7)

use crate::net::frame_codec;
use crate::net::message::Message;
use std::time::Duration;
use tokio::io::{BufReader, BufWriter};
use tokio::net::TcpStream;
use tracing::{debug, info};

/// Connect timeout in seconds.
pub const CONNECT_TIMEOUT_SECS: u64 = 5;
/// Read timeout for individual frame reads.
pub const READ_TIMEOUT_SECS: u64 = 30;
/// Write timeout for frame writes.
pub const WRITE_TIMEOUT_SECS: u64 = 10;

/// A TCP client that connects to a remote Trit-Core node server.
///
/// Uses buffered I/O (BufReader / BufWriter over a split TcpStream)
/// to support multi-message sessions without consuming the underlying
/// stream.
pub struct TcpClient {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>,
}

impl TcpClient {
    /// Connect to a remote node server with a 5-second timeout.
    pub async fn connect(addr: &str) -> std::io::Result<Self> {
        info!(addr = %addr, "connecting to Trit-Core node");
        let stream = tokio::time::timeout(
            Duration::from_secs(CONNECT_TIMEOUT_SECS),
            TcpStream::connect(addr),
        )
        .await
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!(
                    "Connect to {} timed out after {}s",
                    addr, CONNECT_TIMEOUT_SECS
                ),
            )
        })??;

        let (read_half, write_half) = stream.into_split();
        let reader = BufReader::new(read_half);
        let writer = BufWriter::new(write_half);
        debug!(addr = %addr, "connected");
        Ok(Self { reader, writer })
    }

    /// Send a message and wait for the response.
    ///
    /// Uses buffered I/O — the underlying stream is NOT consumed, so this
    /// method can be called multiple times on the same client.
    pub async fn send(&mut self, msg: &Message) -> std::io::Result<Message> {
        let json = serde_json::to_vec(msg)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        // Write with timeout
        tokio::time::timeout(
            Duration::from_secs(WRITE_TIMEOUT_SECS),
            frame_codec::write_frame(&mut self.writer, &json),
        )
        .await
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("Write timed out after {}s", WRITE_TIMEOUT_SECS),
            )
        })??;

        // Read response with timeout
        let payload = tokio::time::timeout(
            Duration::from_secs(READ_TIMEOUT_SECS),
            frame_codec::read_frame(&mut self.reader),
        )
        .await
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("Read timed out after {}s", READ_TIMEOUT_SECS),
            )
        })??;

        let response: Message = serde_json::from_slice(&payload)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(response)
    }

    /// Send a RESONATE_REQ and return the ACK.
    pub async fn resonate(
        &mut self,
        node_id: &str,
        frame: &str,
        phase: f64,
        history: Vec<f64>,
    ) -> std::io::Result<Message> {
        let req = Message::resonate_req(node_id, frame, phase, history);
        debug!(node = %node_id, frame = %frame, phase = phase, "sending RESONATE_REQ");
        self.send(&req).await
    }

    /// Send a DECOUPLE_REQ and return the ACK.
    pub async fn decouple(
        &mut self,
        node_id: &str,
        reason: &str,
        cycles_coupled: u64,
    ) -> std::io::Result<Message> {
        let req = Message::decouple_req(node_id, reason);
        debug!(node = %node_id, reason = %reason, "sending DECOUPLE_REQ");
        let _ = cycles_coupled;
        self.send(&req).await
    }

    /// Send a HEARTBEAT and return the echo.
    pub async fn heartbeat(
        &mut self,
        node_id: &str,
        state: &str,
        phase: f64,
    ) -> std::io::Result<Message> {
        let req = Message::heartbeat(node_id, state, phase);
        debug!(node = %node_id, state = %state, "sending HEARTBEAT");
        self.send(&req).await
    }

    /// Send a NEGOTIATE request and return the consensus result.
    pub async fn negotiate(
        &mut self,
        node_id: &str,
        participants: Vec<String>,
        frames: Vec<String>,
        phases: Vec<f64>,
    ) -> std::io::Result<Message> {
        let req = Message::negotiate(node_id, participants, frames, phases, "hold");
        debug!(node = %node_id, "sending NEGOTIATE");
        self.send(&req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Frame;
    use crate::net::bus::ResonanceBus;
    use crate::net::node::Node;
    use crate::net::tcp_server::TcpNodeServer;

    async fn setup_server_with_client() -> (String, TcpClient) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);

        let mut bus = ResonanceBus::new();
        bus.register(Node::new("test-client".into(), Frame::Science, 0.7));
        bus.register(Node::new("peer-a".into(), Frame::Science, 0.8));

        let server = TcpNodeServer::with_bus(&addr, bus);
        tokio::spawn(async move {
            let _ = server.serve().await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = TcpClient::connect(&addr).await.unwrap();
        (addr, client)
    }

    #[tokio::test]
    async fn client_connect_and_heartbeat() {
        let (_addr, mut client) = setup_server_with_client().await;
        let resp = client.heartbeat("test-client", "Sovereign", 0.7).await;
        assert!(resp.is_ok());
        match &resp.unwrap().payload {
            crate::net::message::MessagePayload::Heartbeat(hb) => {
                assert!((hb.current_phase - 0.7).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Heartbeat"),
        }
    }

    #[tokio::test]
    async fn client_resonate_gets_constructive_ack() {
        let (_addr, mut client) = setup_server_with_client().await;
        let resp = client.resonate("test-client", "Science", 0.7, vec![]).await;
        assert!(resp.is_ok());
        match &resp.unwrap().payload {
            crate::net::message::MessagePayload::ResonateAck(data) => {
                assert_eq!(data.interference, "constructive");
                assert!(!data.conflict_detected);
            }
            _ => panic!("Expected ResonateAck"),
        }
    }

    #[tokio::test]
    async fn client_decouple_gets_ack() {
        let (_addr, mut client) = setup_server_with_client().await;
        let resp = client.decouple("test-client", "user_disconnect", 0).await;
        assert!(resp.is_ok());
        match &resp.unwrap().payload {
            crate::net::message::MessagePayload::DecoupleAck(data) => {
                assert!((data.restored_phase - 0.7).abs() < f64::EPSILON);
            }
            _ => panic!("Expected DecoupleAck"),
        }
    }

    #[tokio::test]
    async fn multi_message_session() {
        // Verify TcpClient supports multiple send/recv cycles on one connection.
        let (_addr, mut client) = setup_server_with_client().await;

        // First message: heartbeat
        let resp = client.heartbeat("test-client", "Sovereign", 0.7).await;
        assert!(resp.is_ok());

        // Second message: resonate
        let resp = client.resonate("test-client", "Science", 0.7, vec![]).await;
        assert!(resp.is_ok());

        // Third message: decouple
        let resp = client.decouple("test-client", "test_done", 0).await;
        assert!(resp.is_ok());
    }
}
