use std::{net::SocketAddr, path::Path, time::Duration};

use chrono::{DateTime, Utc};
use http::uri::Scheme;
use human_readable_duration::HumanReadableDuration;
use serde::{Deserialize, Serialize};
use simple_id::chrono_id::Id as ChronoId;
use sqlx::SqlitePool;

mod human_readable_duration;

#[derive(Debug, Clone, derive_more::Deref)]
pub struct Db(SqlitePool);

impl Db {
    pub async fn new<P: AsRef<Path>>(path: P) -> sqlx::Result<Self> {
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .create_if_missing(true)
            .pragma("cache_size", "-10000")
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .optimize_on_close(true, None)
            .filename(path);
        let pool = sqlx::Pool::connect_with(options).await?;

        sqlx::migrate!().run(&pool).await?;

        Ok(Self(pool.clone()))
    }

    pub fn connection_table(&self) -> ConnectionTable {
        ConnectionTable(self.0.clone())
    }

    pub fn request_table(&self) -> RequestTable {
        RequestTable(self.0.clone())
    }

    pub fn response_table(&self) -> ResponseTable {
        ResponseTable(self.0.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Connection {
    pub id: ChronoId,
    pub created_at: DateTime<Utc>,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub http_scheme: String,
    pub http_version: HttpVersion,
}

impl Connection {
    pub fn to_stored(self) -> StoredConnection {
        StoredConnection {
            id: self.id,
            created_at: self.created_at,
            local_addr: self.local_addr.to_string(),
            remote_addr: self.remote_addr.to_string(),
            http_scheme: self.http_scheme,
            http_version: self.http_version.to_string(),
        }
    }

    pub fn from_stored(stored: StoredConnection) -> Self {
        Connection {
            id: stored.id,
            created_at: stored.created_at,
            local_addr: stored.local_addr.parse().unwrap(),
            remote_addr: stored.remote_addr.parse().unwrap(),
            http_scheme: stored.http_scheme,
            http_version: stored.http_version.parse().unwrap(),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum HttpVersion {
    Http09,
    Http10,
    Http11,
    Http2,
    Http3,
    Unknown,
}

impl From<http::Version> for HttpVersion {
    fn from(value: http::Version) -> Self {
        use http::Version;
        match value {
            Version::HTTP_09 => Self::Http09,
            Version::HTTP_10 => Self::Http10,
            Version::HTTP_11 => Self::Http11,
            Version::HTTP_2 => Self::Http2,
            Version::HTTP_3 => Self::Http3,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Http09 => "HTTP/0.9",
                Self::Http10 => "HTTP/1.0",
                Self::Http11 => "HTTP/1.1",
                Self::Http2 => "HTTP/2",
                Self::Http3 => "HTTP/3",
                Self::Unknown => "Unknown",
            }
        )
    }
}

impl std::str::FromStr for HttpVersion {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "HTTP/0.9" | "0.9" => Self::Http09,
            "HTTP/1" | "1" | "HTTP/1.0" | "1.0" => Self::Http10,
            "HTTP/1.1" | "1.1" => Self::Http11,
            "HTTP/2" | "2" | "HTTP/2.0" | "2.0" => Self::Http2,
            "HTTP/3" | "3" | "HTTP/3.0" | "3.0" => Self::Http3,
            _ => Self::Unknown,
        })
    }
}

pub struct StoredConnection {
    pub id: ChronoId,
    pub created_at: DateTime<Utc>,
    pub local_addr: String,
    pub remote_addr: String,
    pub http_scheme: String,
    pub http_version: String,
}

#[derive(Debug, Clone)]
pub struct ConnectionTable(sqlx::Pool<sqlx::Sqlite>);

impl ConnectionTable {
    pub async fn insert(
        &self,
        local_addr: &SocketAddr,
        remote_addr: &SocketAddr,
        http_scheme: &Scheme,
        http_version: &http::Version,
    ) -> sqlx::Result<Connection> {
        let e = Connection {
            id: ChronoId::new(),
            created_at: Utc::now(),
            local_addr: local_addr.clone(),
            remote_addr: remote_addr.clone(),
            http_scheme: http_scheme.to_string(),
            http_version: http_version.clone().into(),
        };
        let stored = e.clone().to_stored();

        sqlx::query(
            "
            INSERT INTO sa_connection (
                id,
                created_at,
                local_addr,
                remote_addr,
                http_scheme,
                http_version
            ) VALUES (?, ?, ?, ?, ?, ?)
        ",
        )
        .bind(&stored.id)
        .bind(&stored.created_at)
        .bind(&stored.local_addr)
        .bind(&stored.remote_addr)
        .bind(&stored.http_scheme)
        .bind(&stored.http_version)
        .execute(&self.0)
        .await?;

        Ok(e)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Request {
    pub id: ChronoId,
    pub created_at: DateTime<Utc>,
    pub conn_id: Option<ChronoId>,
    pub method: String,
    pub path: String,
    pub hostname: String,
    pub user_agent: String,
}

#[derive(Debug, Clone)]
pub struct RequestTable(sqlx::Pool<sqlx::Sqlite>);

impl RequestTable {
    pub async fn insert(
        &self,
        conn_id: Option<&ChronoId>,
        method: &str,
        path: &str,
        hostname: &str,
        user_agent: &str,
    ) -> sqlx::Result<Request> {
        let e = Request {
            id: ChronoId::new(),
            created_at: Utc::now(),
            conn_id: conn_id.cloned(),
            method: method.to_owned(),
            path: path.to_owned(),
            hostname: hostname.to_owned(),
            user_agent: user_agent.to_owned(),
        };

        sqlx::query(
            "
            INSERT INTO sa_request (
                id,
                created_at,
                conn_id,
                method,
                path,
                hostname,
                user_agent
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
        ",
        )
        .bind(&e.id)
        .bind(&e.created_at)
        .bind(&e.conn_id)
        .bind(&e.method)
        .bind(&e.path)
        .bind(&e.hostname)
        .bind(&e.user_agent)
        .execute(&self.0)
        .await?;

        Ok(e)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Response {
    pub id: ChronoId,
    pub created_at: DateTime<Utc>,
    pub conn_id: Option<ChronoId>,
    pub req_id: ChronoId,
    pub duration: Duration,
    pub status: u16,
}

pub struct StoredResponse {
    pub id: ChronoId,
    pub created_at: DateTime<Utc>,
    pub conn_id: Option<ChronoId>,
    pub req_id: ChronoId,
    pub duration: HumanReadableDuration,
    pub status: u16,
}

#[derive(Debug, Clone)]
pub struct ResponseTable(sqlx::Pool<sqlx::Sqlite>);

impl ResponseTable {
    pub async fn insert(
        &self,
        conn_id: Option<&ChronoId>,
        req_id: &ChronoId,
        duration: &Duration,
        status: u16,
    ) -> sqlx::Result<Response> {
        let e = Response {
            id: ChronoId::new(),
            created_at: Utc::now(),
            conn_id: conn_id.cloned(),
            req_id: req_id.clone(),
            duration: duration.clone(),
            status,
        };

        sqlx::query(
            "
            INSERT INTO sa_request (
                id,
                created_at,
                conn_id,
                req_id,
                elapsed,
                status
            ) VALUES (?, ?, ?, ?, ?, ?)
        ",
        )
        .bind(&e.id)
        .bind(&e.created_at)
        .bind(&e.conn_id)
        .bind(&e.req_id)
        .bind(&HumanReadableDuration(e.duration))
        .bind(&e.status)
        .execute(&self.0)
        .await?;

        Ok(e)
    }
}
