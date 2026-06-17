// TCP frame protocol for Trit-Core distributed nodes.
//
// Wire format: 4-byte big-endian length prefix followed by JSON payload.
// This is the simplest framing protocol that is interoperable across
// languages and easy to debug (the payload is human-readable JSON).
//
// ## Frame layout
//
// | 0..4         | 4..(4+len)   |
// |--------------|--------------|
// | len (u32 BE) | JSON payload |
//
// ## Design rationale
//
// Length-prefix framing was chosen over newline-delimited JSON for
// two reasons:
// 1. Binary safety: JSON payloads may contain embedded newlines in
//    string fields (e.g., conflict reasons with multi-line descriptions).
// 2. Zero-copy potential: knowing the exact length upfront allows
//    pre-allocating the read buffer.

use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Maximum frame size: 1 MiB. Frames exceeding this are rejected
/// to prevent memory exhaustion attacks (CWE-770).
pub const MAX_FRAME_SIZE: usize = 1_048_576;

/// Read a complete frame from an async reader.
///
/// Returns the raw JSON payload bytes. The caller is responsible for
/// deserializing into a `Message`.
///
/// # Errors
///
/// Returns `io::Error` if:
/// - The connection is closed before a complete frame is read
/// - The length prefix exceeds `MAX_FRAME_SIZE`
/// - The payload length doesn't match the prefix
pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Vec<u8>> {
    // Read 4-byte length prefix
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Frame size {} exceeds maximum {} bytes",
                len, MAX_FRAME_SIZE
            ),
        ));
    }

    // Read payload
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;
    Ok(payload)
}

/// Write a complete frame to an async writer.
///
/// Encodes the payload with a 4-byte big-endian length prefix.
pub async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, payload: &[u8]) -> io::Result<()> {
    let len = payload.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn roundtrip_small_frame() {
        let payload = br#"{"op":"HEARTBEAT","node_state":"Sovereign"}"#;
        let mut buf = Vec::new();

        write_frame(&mut buf, payload).await.unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let result = read_frame(&mut cursor).await.unwrap();

        assert_eq!(result, payload);
    }

    #[tokio::test]
    async fn roundtrip_empty_payload() {
        let payload = b"{}";
        let mut buf = Vec::new();

        write_frame(&mut buf, payload).await.unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let result = read_frame(&mut cursor).await.unwrap();

        assert_eq!(result, payload);
    }

    #[tokio::test]
    async fn roundtrip_large_payload() {
        let payload = vec![b'x'; 64_000];
        let mut buf = Vec::new();

        write_frame(&mut buf, &payload).await.unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let result = read_frame(&mut cursor).await.unwrap();

        assert_eq!(result, payload);
    }

    #[tokio::test]
    async fn rejects_oversized_frame() {
        // Create a valid frame header claiming a huge payload
        let huge_len = (MAX_FRAME_SIZE + 1) as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(&huge_len.to_be_bytes());
        buf.extend_from_slice(&[0u8; 10]); // insufficient payload

        let mut cursor = std::io::Cursor::new(buf);
        let result = read_frame(&mut cursor).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("exceeds maximum"));
    }

    #[tokio::test]
    async fn multiple_frames_in_stream() {
        let frames: Vec<&[u8]> = vec![b"frame1", b"frame2_data", b"f3"];
        let mut buf = Vec::new();

        for payload in &frames {
            write_frame(&mut buf, payload).await.unwrap();
        }

        let mut cursor = std::io::Cursor::new(buf);
        for expected in &frames {
            let result = read_frame(&mut cursor).await.unwrap();
            assert_eq!(&result, expected);
        }
    }
}
