// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite dispatch for display IPC streams (Unix, TCP, test duplex).

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpStream, UnixStream};

/// Tokio streams used by [`super::DisplayClient`].
pub enum AsyncStreamDispatch {
    /// Unix domain socket stream.
    Unix(UnixStream),
    /// TCP stream (fallback).
    Tcp(TcpStream),
    /// In-memory duplex (tests).
    #[cfg(test)]
    Duplex(tokio::io::DuplexStream),
}

impl AsyncRead for AsyncStreamDispatch {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.as_mut().get_mut() {
            Self::Unix(s) => Pin::new(s).poll_read(cx, buf),
            Self::Tcp(s) => Pin::new(s).poll_read(cx, buf),
            #[cfg(test)]
            Self::Duplex(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for AsyncStreamDispatch {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.as_mut().get_mut() {
            Self::Unix(s) => Pin::new(s).poll_write(cx, buf),
            Self::Tcp(s) => Pin::new(s).poll_write(cx, buf),
            #[cfg(test)]
            Self::Duplex(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_mut().get_mut() {
            Self::Unix(s) => Pin::new(s).poll_flush(cx),
            Self::Tcp(s) => Pin::new(s).poll_flush(cx),
            #[cfg(test)]
            Self::Duplex(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_mut().get_mut() {
            Self::Unix(s) => Pin::new(s).poll_shutdown(cx),
            Self::Tcp(s) => Pin::new(s).poll_shutdown(cx),
            #[cfg(test)]
            Self::Duplex(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}
