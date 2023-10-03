use salvo::{
    async_trait,
    hyper::header::{HOST, USER_AGENT},
    Depot, FlowCtrl, Handler, Request, Response,
};
use tracing::*;

use crate::SimpleAnalytics;

use super::service::ConnId;

pub struct SimpleAnalyticsHandler {
    sa: SimpleAnalytics,
}

#[async_trait]
impl Handler for SimpleAnalyticsHandler {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        let conn_id = req.extensions().get::<ConnId>().cloned();
        let started = std::time::Instant::now();

        let req_id = self
            .sa
            .report_request(
                conn_id.map(|ci| ci.0).as_ref(),
                req.method().as_str(),
                req.uri().path(),
                req.headers()
                    .get(HOST)
                    .map(|v| v.to_str().ok())
                    .flatten()
                    .unwrap_or_default(),
                req.headers()
                    .get(USER_AGENT)
                    .map(|v| v.to_str().ok())
                    .flatten()
                    .unwrap_or_default(),
            )
            .await;
        if let Err(ref e) = req_id {
            error!("Failed to report request: {e:?}");
        }

        ctrl.call_next(req, depot, res).await;

        let duration = started.elapsed();
        let status = res.status_code.unwrap_or_default().as_u16();

        if let Ok(req_id) = req_id {
            let res_id = self
                .sa
                .report_response(conn_id.map(|ci| ci.0).as_ref(), &req_id, &duration, status)
                .await;

            if let Err(ref e) = res_id {
                error!("Failed to report request: {e:?}");
            }
        }
    }
}
