use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Proxy {
    pub id: i64,
    pub ip: String,
    pub port: u16,
    pub protocol: String,        // http, https, socks4, socks5
    pub anonymity: String,       // transparent, anonymous, elite
    pub country: String,
    pub score: f64,
    pub is_alive: bool,
    pub success_count: i64,
    pub fail_count: i64,
    pub consecutive_fails: i64,
    pub avg_latency_ms: f64,
    pub last_check_at: Option<NaiveDateTime>,
    pub last_success_at: Option<NaiveDateTime>,
    pub next_check_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyResponse {
    pub proxy: String,
    pub protocol: String,
    pub country: String,
    pub anonymity: String,
    pub score: f64,
    pub latency_ms: f64,
    pub is_alive: bool,
}

impl From<Proxy> for ProxyResponse {
    fn from(p: Proxy) -> Self {
        Self {
            proxy: format!("{}:{}", p.ip, p.port),
            protocol: p.protocol,
            country: p.country,
            anonymity: p.anonymity,
            score: p.score,
            latency_ms: p.avg_latency_ms,
            is_alive: p.is_alive,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStats {
    pub total_proxies: i64,
    pub alive_proxies: i64,
    pub dead_proxies: i64,
    pub avg_score: f64,
    pub avg_latency_ms: f64,
    pub country_distribution: Vec<CountryCount>,
    pub latency_distribution: Vec<LatencyBucket>,
    pub protocol_distribution: Vec<ProtocolCount>,
    pub score_distribution: Vec<ScoreBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CountryCount {
    pub country: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyBucket {
    pub range: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProtocolCount {
    pub protocol: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBucket {
    pub range: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct CheckLog {
    pub id: i64,
    pub proxy_id: i64,
    pub target: String,
    pub success: bool,
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
    pub checked_at: NaiveDateTime,
}
