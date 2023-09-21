use std::io::{Error as IoError, ErrorKind as IoErrorKind, Result as IoResult};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use pin_project::pin_project;
use salvo::conn::{Accepted, Acceptor, Holding, HttpBuilder, SocketAddr};
use salvo::core::HyperHandler;
use salvo::http::{uri::Scheme, HttpConnection};
use salvo::hyper::Version;
use salvo::{async_trait, Listener};
use simple_id::random_id::Id as RandomId;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::sync::CancellationToken;

pub struct SimpleListener<T> {
    inner: T,
}

#[async_trait]
impl<T> Listener for SimpleListener<T>
where
    T: Listener + Send + Unpin + 'static,
    T::Acceptor: Acceptor + Send + Unpin + 'static,
{
    type Acceptor = SimpleAcceptor<T::Acceptor>;

    async fn try_bind(self) -> IoResult<Self::Acceptor> {
        let bound = self.inner.try_bind().await?;
        Ok(SimpleAcceptor { inner: bound })
    }
}

pub struct SimpleAcceptor<T> {
    inner: T,
}

#[async_trait]
impl<T> Acceptor for SimpleAcceptor<T>
where
    T: Acceptor + Send + Unpin + 'static,
    T::Conn: HttpConnection + AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Conn = SimpleStream<T::Conn>;

    #[inline]
    fn holdings(&self) -> &[Holding] {
        self.inner.holdings()
    }

    #[inline]
    async fn accept(&mut self) -> IoResult<Accepted<Self::Conn>> {
        let accepted = self.inner.accept().await?;
        let conn_info = ConnectionInfo {
            id: RandomId::new(),
            local_addr: accepted.local_addr.clone(),
            remote_addr: accepted.remote_addr.clone(),
            http_scheme: accepted.http_scheme.clone(),
            http_version: accepted.http_version,
        };

        Ok(accepted.map_conn(|conn| SimpleStream {
            inner: conn,
            conn_info,
        }))
    }
}

#[pin_project]
pub struct SimpleStream<T> {
    #[pin]
    inner: T,
    conn_info: ConnectionInfo,
}

impl<T> SimpleStream<T> {
    pub fn new(inner: T, conn_info: ConnectionInfo) -> Self {
        Self { inner, conn_info }
    }
}

impl<T> AsyncRead for SimpleStream<T>
where
    T: AsyncRead + Send + Unpin + 'static,
{
    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        self.project().inner.poll_read(cx, buf)
    }
}

impl<T> AsyncWrite for SimpleStream<T>
where
    T: AsyncWrite + Send + Unpin + 'static,
{
    #[inline]
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        self.project().inner.poll_write(cx, buf)
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        self.project().inner.poll_flush(cx)
    }

    #[inline]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        self.project().inner.poll_shutdown(cx)
    }
}

#[async_trait]
impl<T> HttpConnection for SimpleStream<T>
where
    T: HttpConnection + AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    async fn serve(
        self,
        handler: HyperHandler,
        builder: Arc<HttpBuilder>,
        server_shutdown_token: CancellationToken,
        idle_connection_timeout: Option<Duration>,
    ) -> IoResult<()> {
        let service = super::service::ConnIdService::new(handler, self.conn_info.id);

        builder
            .serve_connection(
                self,
                service,
                server_shutdown_token,
                idle_connection_timeout,
            )
            .await
            .map_err(|e| IoError::new(IoErrorKind::Other, e.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: RandomId,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub http_scheme: Scheme,
    pub http_version: Version,
}
