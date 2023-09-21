use salvo::hyper::{
    service::Service as HyperService, Request as HyperRequest, Response as HyperResponse,
};
use simple_id::random_id::Id as RandomId;

use salvo::http::body::HyperBody;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnId(RandomId);

pub struct ConnIdService<T> {
    inner: T,
    conn_id: ConnId,
}

impl<T> ConnIdService<T> {
    pub fn new(service: T, id: RandomId) -> Self {
        Self {
            inner: service,
            conn_id: ConnId(id),
        }
    }
}

impl<T, B> HyperService<HyperRequest<HyperBody>> for ConnIdService<T>
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
        req.extensions_mut().insert::<ConnId>(self.conn_id);
        self.inner.call(req)
    }
}
