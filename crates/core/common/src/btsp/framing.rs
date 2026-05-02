// SPDX-License-Identifier: AGPL-3.0-or-later
//! Length-prefixed BTSP frame codec.
//!
//! All BTSP frames use the same wire format regardless of cipher suite:
//!
//! ```text
//! [ Length: 4 bytes BE u32 ][ Payload: Length bytes ]
//! ```
//!
//! Max frame size: 16 MiB (`MAX_FRAME_SIZE`).
//!
//! In dev mode (NDJSON), framing uses newlines. In production (BTSP),
//! framing uses length-prefixed frames — this module handles the latter.

use std::io;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::types::MAX_FRAME_SIZE;

/// Wraps a stream, prepending a single already-consumed byte.
///
/// Used by BTSP auto-detect: one byte is read to distinguish binary length-prefixed
/// framing from text (`{` / HTTP), then the handshake reader sees the full frame.
#[derive(Debug)]
pub struct PrependByte<S> {
    first: Option<u8>,
    inner: S,
}

impl<S> PrependByte<S> {
    /// Wrap `inner`, yielding `first` as the first byte of the read stream.
    #[must_use]
    pub const fn new(first: u8, inner: S) -> Self {
        Self {
            first: Some(first),
            inner,
        }
    }

    /// Unwrap the inner stream (drops any not-yet-read prepended byte).
    #[must_use]
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for PrependByte<S> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if let Some(b) = self.first.take() {
            buf.put_slice(&[b]);
            return std::task::Poll::Ready(Ok(()));
        }
        std::pin::Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for PrependByte<S> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Read a single length-prefixed frame from the stream.
///
/// Returns the frame payload (without the length header).
///
/// # Errors
///
/// - `UnexpectedEof` if the stream closes before a complete frame.
/// - `InvalidData` if the frame exceeds `MAX_FRAME_SIZE`.
pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);

    if len > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("BTSP frame too large: {len} bytes (max {MAX_FRAME_SIZE})"),
        ));
    }

    let mut payload = vec![0u8; len as usize];
    reader.read_exact(&mut payload).await?;
    Ok(payload)
}

/// Write a single length-prefixed frame to the stream.
///
/// # Errors
///
/// - `InvalidData` if the payload exceeds `MAX_FRAME_SIZE`.
/// - I/O errors from the underlying stream.
pub async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, payload: &[u8]) -> io::Result<()> {
    let len: u32 = payload.len().try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "BTSP frame payload too large: {} bytes (max {})",
                payload.len(),
                MAX_FRAME_SIZE
            ),
        )
    })?;

    if len > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("BTSP frame too large: {len} bytes (max {MAX_FRAME_SIZE})"),
        ));
    }

    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;
    Ok(())
}

/// Buffered frame reader that wraps an async stream.
pub struct BtspFrameReader<R> {
    inner: R,
}

impl<R: AsyncRead + Unpin> BtspFrameReader<R> {
    /// Create a new frame reader.
    pub const fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Read the next frame.
    ///
    /// # Errors
    ///
    /// Returns I/O errors or `InvalidData` for oversized frames.
    pub async fn read(&mut self) -> io::Result<Vec<u8>> {
        read_frame(&mut self.inner).await
    }
}

/// Buffered frame writer that wraps an async stream.
pub struct BtspFrameWriter<W> {
    inner: W,
}

impl<W: AsyncWrite + Unpin> BtspFrameWriter<W> {
    /// Create a new frame writer.
    pub const fn new(inner: W) -> Self {
        Self { inner }
    }

    /// Write a frame.
    ///
    /// # Errors
    ///
    /// Returns I/O errors or `InvalidData` for oversized payloads.
    pub async fn write(&mut self, payload: &[u8]) -> io::Result<()> {
        write_frame(&mut self.inner, payload).await
    }
}

/// Read an encrypted frame: `[4B len BE u32][len bytes: nonce + ciphertext + tag]`.
///
/// Decrypts the payload using the provided session keys and returns the plaintext.
///
/// # Errors
///
/// - `UnexpectedEof` if the stream closes before a complete frame.
/// - `InvalidData` if the frame exceeds `MAX_FRAME_SIZE` or decryption fails.
pub async fn read_encrypted_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
    keys: &super::phase3::Phase3SessionKeys,
) -> io::Result<Vec<u8>> {
    let encrypted = read_frame(reader).await?;
    keys.decrypt(&encrypted).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("BTSP Phase 3 decrypt: {e}"),
        )
    })
}

/// Write an encrypted frame: `[4B len BE u32][12B nonce][ciphertext + tag]`.
///
/// Encrypts the plaintext using the provided session keys and writes the frame.
///
/// # Errors
///
/// - `InvalidData` if encryption fails or the frame exceeds `MAX_FRAME_SIZE`.
/// - I/O errors from the underlying stream.
pub async fn write_encrypted_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    keys: &super::phase3::Phase3SessionKeys,
    plaintext: &[u8],
) -> io::Result<()> {
    let encrypted = keys.encrypt(plaintext).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("BTSP Phase 3 encrypt: {e}"),
        )
    })?;
    write_frame(writer, &encrypted).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip_single_frame() {
        let payload = b"hello BTSP";
        let mut buf = Vec::new();

        write_frame(&mut buf, payload).await.expect("write");
        assert_eq!(buf.len(), 4 + payload.len());

        let mut cursor = io::Cursor::new(buf);
        let read_back = read_frame(&mut cursor).await.expect("read");
        assert_eq!(read_back, payload);
    }

    #[tokio::test]
    async fn round_trip_empty_frame() {
        let mut buf = Vec::new();
        write_frame(&mut buf, b"").await.expect("write");

        let mut cursor = io::Cursor::new(buf);
        let read_back = read_frame(&mut cursor).await.expect("read");
        assert!(read_back.is_empty());
    }

    #[tokio::test]
    async fn round_trip_multiple_frames() {
        let frames: Vec<&[u8]> = vec![b"frame-1", b"frame-2-longer", b"f3"];
        let mut buf = Vec::new();

        for f in &frames {
            write_frame(&mut buf, f).await.expect("write");
        }

        let mut cursor = io::Cursor::new(buf);
        for expected in &frames {
            let got = read_frame(&mut cursor).await.expect("read");
            assert_eq!(got.as_slice(), *expected);
        }
    }

    #[tokio::test]
    async fn rejects_oversized_frame_on_read() {
        let bad_len = (MAX_FRAME_SIZE + 1).to_be_bytes();
        let mut cursor = io::Cursor::new(bad_len.to_vec());
        let err = read_frame(&mut cursor).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("too large"));
    }

    #[tokio::test]
    async fn frame_reader_writer_types() {
        let payload = b"typed-frame";
        let mut buf = Vec::new();

        let mut writer = BtspFrameWriter::new(&mut buf);
        writer.write(payload).await.expect("write");

        let mut reader = BtspFrameReader::new(io::Cursor::new(buf));
        let got = reader.read().await.expect("read");
        assert_eq!(got, payload);
    }

    #[tokio::test]
    async fn read_frame_eof_returns_error() {
        let mut cursor = io::Cursor::new(vec![0u8, 0, 0]);
        let err = read_frame(&mut cursor).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }
}
