use std::{net::SocketAddr, path::Path, time::Duration};

use salvo::{http::uri::Scheme, hyper::Version};
use simple_id::chrono_id::Id as ChronoId;
use simple_server_analytics_db::Db;

pub mod salvo_ext;

#[derive(Debug, Clone)]
pub struct SimpleAnalytics {
    db: Db,
}

impl SimpleAnalytics {
    pub async fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        Ok(Self {
            db: Db::new(path).await?,
        })
    }

    pub async fn report_new_connection(
        &self,
        local_addr: &SocketAddr,
        remote_addr: &SocketAddr,
        http_scheme: &Scheme,
        http_version: &Version,
    ) -> sqlx::Result<ChronoId> {
        let conn_db = self
            .db
            .connection_table()
            .insert(local_addr, remote_addr, http_scheme, http_version)
            .await?;

        Ok(conn_db.id)
    }

    pub async fn report_request(
        &self,
        conn_id: Option<&ChronoId>,
        method: &str,
        path: &str,
        hostname: &str,
        user_agent: &str,
    ) -> sqlx::Result<ChronoId> {
        let req_db = self
            .db
            .request_table()
            .insert(conn_id, method, path, hostname, user_agent)
            .await?;

        Ok(req_db.id)
    }

    pub async fn report_response(
        &self,
        conn_id: Option<&ChronoId>,
        req_id: &ChronoId,
        duration: &Duration,
        status: u16,
    ) -> sqlx::Result<ChronoId> {
        let res_db = self
            .db
            .response_table()
            .insert(conn_id, req_id, duration, status)
            .await?;

        Ok(res_db.id)
    }
}
