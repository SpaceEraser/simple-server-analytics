use salvo::hyper::{
    service::Service as HyperService, Request as HyperRequest, Response as HyperResponse,
};
use simple_id::chrono_id::Id as ChronoId;

use salvo::http::body::HyperBody;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Deref)]
pub struct ConnId(pub ChronoId);

pub struct SimpleAnalyticsService<T> {
    inner: T,
    conn_id: Option<ChronoId>,
}

impl<T> SimpleAnalyticsService<T> {
    pub fn new(service: T, conn_id: Option<ChronoId>) -> Self {
        Self {
            inner: service,
            conn_id,
        }
    }
}

impl<T, B> HyperService<HyperRequest<HyperBody>> for SimpleAnalyticsService<T>
where
    T: HyperService<HyperRequest<HyperBody>, Response = HyperResponse<B>> + Send,
    T::Future: Send + 'static,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    B: salvo::http::Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = T::Response;
    type Error = T::Error;
    type Future = T::Future;

    fn call(&self, mut req: HyperRequest<HyperBody>) -> Self::Future {
        if let Some(conn_id) = self.conn_id {
            req.extensions_mut().insert(ConnId(conn_id));
        }
        self.inner.call(req)
    }
}
