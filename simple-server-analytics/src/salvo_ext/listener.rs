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
use simple_id::chrono_id::Id as ChronoId;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::sync::CancellationToken;

use crate::SimpleAnalytics;

use super::service::SimpleAnalyticsService;

pub struct SimpleAnalyticsListener<T> {
    inner: T,
    sa: SimpleAnalytics,
}

impl<T> SimpleAnalyticsListener<T> {
    pub fn new(inner: T, sa: &SimpleAnalytics) -> Self {
        Self {
            inner,
            sa: sa.clone(),
        }
    }
}

#[async_trait]
impl<T> Listener for SimpleAnalyticsListener<T>
where
    T: Listener + Send + Unpin + 'static,
    T::Acceptor: Acceptor + Send + Unpin + 'static,
{
    type Acceptor = SimpleAnalyticsAcceptor<T::Acceptor>;

    async fn try_bind(self) -> IoResult<Self::Acceptor> {
        let bound = self.inner.try_bind().await?;
        Ok(SimpleAnalyticsAcceptor {
            inner: bound,
            sa: self.sa.clone(),
        })
    }
}

pub struct SimpleAnalyticsAcceptor<T> {
    inner: T,
    sa: SimpleAnalytics,
}

#[async_trait]
impl<T> Acceptor for SimpleAnalyticsAcceptor<T>
where
    T: Acceptor + Send + Unpin + 'static,
    T::Conn: HttpConnection + AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Conn = SimpleAnalyticsStream<T::Conn>;

    #[inline]
    fn holdings(&self) -> &[Holding] {
        self.inner.holdings()
    }

    #[inline]
    async fn accept(&mut self) -> IoResult<Accepted<Self::Conn>> {
        let accepted = self.inner.accept().await?;
        let reported_conn = self
            .sa
            .report_new_connection(
                &accepted.local_addr.clone().into_std().unwrap(),
                &accepted.remote_addr.clone().into_std().unwrap(),
                &accepted.http_scheme,
                &accepted.http_version,
            )
            .await;

        Ok(accepted.map_conn(|conn| SimpleAnalyticsStream {
            inner: conn,
            conn_id: reported_conn.ok().map(|id| id),
        }))
    }
}

#[pin_project]
pub struct SimpleAnalyticsStream<T> {
    #[pin]
    inner: T,
    conn_id: Option<ChronoId>,
}

impl<T> AsyncRead for SimpleAnalyticsStream<T>
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

impl<T> AsyncWrite for SimpleAnalyticsStream<T>
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
impl<T> HttpConnection for SimpleAnalyticsStream<T>
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
        let service = SimpleAnalyticsService::new(handler, self.conn_id);

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
    pub id: ChronoId,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub http_scheme: Scheme,
    pub http_version: Version,
}
