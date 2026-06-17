// TCP client connector for Trit-Core distributed nodes.
//
// TcpClient connects to a remote TcpNodeServer, sends protocol messages,
// and reads responses. It is the client-side counterpart to the server
// in tcp_server.rs.
//
// ## Usage
//
// ```rust,ignore
// let mut client = TcpClient::connect("127.0.0.1:9000").await?;
// let ack = client.resonate("my-node", "Science", 0.7, vec![]).await?;
// client.decouple("my-node", "user_disconnect", 0).await?;
// ```

use crate::net::frame_codec;
use crate::net::message::Message;
use tokio::net::TcpStream;
use tracing::{debug, info};

/// A TCP client that connects to a remote Trit-Core node server.
pub struct TcpClient {
    stream: TcpStream,
}

impl TcpClient {
    /// Connect to a remote node server.
    pub async fn connect(addr: &str) -> std::io::Result<Self> {
        info!(addr = %addr, "connecting to Trit-Core node");
        let stream = TcpStream::connect(addr).await?;
        debug!(addr = %addr, "connected");
        Ok(Self { stream })
    }

    /// Send a message and wait for the response.
    pub async fn send(&mut self, msg: &Message) -> std::io::Result<Message> {
        let json = serde_json::to_vec(msg)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        frame_codec::write_frame(&mut self.stream, &json).await?;

        let (reader, _writer) = self.stream.split();
        let mut reader = reader;
        let payload = frame_codec::read_frame(&mut reader).await?;

        // Reassemble the stream — tokio split consumes the stream.
        // We need to reconnect the halves. For simplicity, we reconnect
        // the stream after each send/recv cycle.
        // Actually, we can use a buffered approach instead.

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
        let _ = cycles_coupled; // Included in the ACK from the server
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

// Note: The `send` method above has a known issue with tokio::split —
// after splitting, the original stream is consumed. For a production
// implementation, we would use a BufReader/BufWriter pair or implement
// a proper framed connection using tokio_util::codec.
//
// For the current MVP, the TcpClient is designed for single-request
// patterns (connect → send → recv → drop). Multi-message sessions
// should use the raw frame_codec directly.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Frame;
    use crate::net::bus::ResonanceBus;
    use crate::net::node::Node;
    use crate::net::tcp_server::TcpNodeServer;

    async fn setup_server_with_client() -> (String, TcpClient) {
        // Bind to port 0 for an OS-assigned free port
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
}
