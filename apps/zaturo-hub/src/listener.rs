//! TCP admission bounds apply before HTTP headers, authentication, or WebSocket upgrade.

use axum::serve::Listener;
use std::{
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
    sync::{OwnedSemaphorePermit, Semaphore},
    time::{Instant, Sleep},
};

const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// Limits accepted sockets, including idle keep-alive and partial-header connections.
pub struct BoundedListener {
    inner: TcpListener,
    permits: Arc<Semaphore>,
    idle_timeout: Duration,
}

impl BoundedListener {
    pub fn new(inner: TcpListener, max_connections: usize) -> io::Result<Self> {
        Self::with_idle_timeout(inner, max_connections, DEFAULT_IDLE_TIMEOUT)
    }

    pub fn with_idle_timeout(
        inner: TcpListener,
        max_connections: usize,
        idle_timeout: Duration,
    ) -> io::Result<Self> {
        if !(1..=1024).contains(&max_connections) || idle_timeout.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "TCP capacity must be 1..1024 and idle timeout must be positive",
            ));
        }
        Ok(Self {
            inner,
            permits: Arc::new(Semaphore::new(max_connections)),
            idle_timeout,
        })
    }
}

impl Listener for BoundedListener {
    type Io = LimitedStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        // This private semaphore is never closed. Reserve before accepting so no
        // unauthenticated socket bypasses the application-level connection bound.
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .expect("private listener semaphore cannot close");
        let (stream, address) = Listener::accept(&mut self.inner).await;
        (
            LimitedStream {
                stream,
                _permit: permit,
                idle_timeout: self.idle_timeout,
                idle: Box::pin(tokio::time::sleep(self.idle_timeout)),
            },
            address,
        )
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

/// A connection retains its admission permit through HTTP upgrade until it is dropped.
pub struct LimitedStream {
    stream: TcpStream,
    _permit: OwnedSemaphorePermit,
    idle_timeout: Duration,
    idle: Pin<Box<Sleep>>,
}

impl LimitedStream {
    fn poll_idle(&mut self, context: &mut Context<'_>) -> io::Result<()> {
        if self.idle.as_mut().poll(context).is_ready() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "connection idle timeout",
            ));
        }
        Ok(())
    }

    fn activity(&mut self) {
        self.idle.as_mut().reset(Instant::now() + self.idle_timeout);
    }
}

impl AsyncRead for LimitedStream {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.poll_idle(context)?;
        let before = buffer.filled().len();
        let outcome = Pin::new(&mut this.stream).poll_read(context, buffer);
        if matches!(outcome, Poll::Ready(Ok(()))) && buffer.filled().len() > before {
            this.activity();
        }
        outcome
    }
}

impl AsyncWrite for LimitedStream {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        this.poll_idle(context)?;
        let outcome = Pin::new(&mut this.stream).poll_write(context, buffer);
        if matches!(outcome, Poll::Ready(Ok(written)) if written > 0) {
            this.activity();
        }
        outcome
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.poll_idle(context)?;
        Pin::new(&mut this.stream).poll_flush(context)
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_shutdown(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn sockets_hold_capacity_until_drop() {
        let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = tcp.local_addr().unwrap();
        let mut listener = BoundedListener::new(tcp, 1).unwrap();
        let _first_client = TcpStream::connect(address).await.unwrap();
        let (first, _) = listener.accept().await;
        let _second_client = TcpStream::connect(address).await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(25), listener.accept())
                .await
                .is_err()
        );
        drop(first);
        let (second, _) = tokio::time::timeout(Duration::from_secs(1), listener.accept())
            .await
            .unwrap();
        assert_eq!(listener.permits.available_permits(), 0);
        drop(second);
        assert_eq!(listener.permits.available_permits(), 1);
    }

    #[tokio::test]
    async fn idle_socket_times_out_and_releases_capacity_when_closed() {
        let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = tcp.local_addr().unwrap();
        let mut listener =
            BoundedListener::with_idle_timeout(tcp, 1, Duration::from_millis(20)).unwrap();
        let _client = TcpStream::connect(address).await.unwrap();
        let (mut stream, _) = listener.accept().await;
        let mut byte = [0];
        let error = tokio::time::timeout(Duration::from_secs(1), stream.read(&mut byte))
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        drop(stream);
        assert_eq!(listener.permits.available_permits(), 1);
    }

    #[tokio::test]
    async fn actual_io_refreshes_idle_deadline() {
        let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = tcp.local_addr().unwrap();
        let mut listener =
            BoundedListener::with_idle_timeout(tcp, 1, Duration::from_millis(200)).unwrap();
        let mut client = TcpStream::connect(address).await.unwrap();
        let (mut stream, _) = listener.accept().await;
        let mut byte = [0];
        for _ in 0..3 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            client.write_all(&[7]).await.unwrap();
            stream.read_exact(&mut byte).await.unwrap();
            assert_eq!(byte, [7]);
        }
        stream.write_all(&[9]).await.unwrap();
        client.read_exact(&mut byte).await.unwrap();
        assert_eq!(byte, [9]);
    }
}
