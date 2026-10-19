use bytes::Bytes;
use chrono::Utc;
use futures_util::StreamExt;
use log::debug;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, LazyLock,
};
use std::time::{Duration, Instant};

const DOWNLOAD_URL: &str = "https://speed.cloudflare.com/__down?bytes=5000000";
const UPLOAD_URL: &str = "https://speed.cloudflare.com/__up";
const PING_URL: &str = "https://speed.cloudflare.com/__down?bytes=0";

const PING_ROUNDS: u32 = 5;

// 4 parallel streams, each on its own OS thread via tokio::spawn on a multi-thread
// runtime. This saturates the link the same way a browser download does.
const PARALLEL_STREAMS: usize = 4;
const WARMUP_SECS: u64 = 4;
const MEASURE_SECS: u64 = 8;

const UPLOAD_CHUNK: usize = 5_000_000; // 5 MB per POST

// Allocated once for the whole process; `Bytes::clone` is zero-copy, so every
// POST across all streams shares this one buffer instead of allocating a fresh
// each — repeated large allocations fragment the heap and ratchet RSS upward.
static UPLOAD_PAYLOAD: LazyLock<Bytes> = LazyLock::new(|| Bytes::from(vec![0u8; UPLOAD_CHUNK]));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedtestResult {
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
    pub collected_at: chrono::DateTime<Utc>,
}

#[derive(Debug)]
pub enum SpeedtestError {
    Http(reqwest::Error),
    Stream(String),
    NoData(String),
}

impl std::fmt::Display for SpeedtestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeedtestError::Http(e) => write!(f, "HTTP error: {}", e),
            SpeedtestError::Stream(s) => write!(f, "Stream error: {}", s),
            SpeedtestError::NoData(s) => write!(f, "No usable measurement: {}", s),
        }
    }
}

impl std::error::Error for SpeedtestError {}

impl From<reqwest::Error> for SpeedtestError {
    fn from(e: reqwest::Error) -> Self {
        SpeedtestError::Http(e)
    }
}

/// A non-success status (notably 429) still yields `Ok` from reqwest and its
/// tiny error body would otherwise be counted as measured payload
fn is_rate_limited(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
}

async fn measure_ping(client: &Client) -> Result<f64, SpeedtestError> {
    // Warmup: establish TLS + HTTP/2 connection.
    let warmup = client.get(PING_URL).send().await?;
    if !warmup.status().is_success() {
        return Err(SpeedtestError::NoData(format!(
            "ping endpoint returned {}",
            warmup.status()
        )));
    }

    let mut total_ms = 0.0;
    for _ in 0..PING_ROUNDS {
        let start = Instant::now();
        let resp = client.get(PING_URL).send().await?;
        if !resp.status().is_success() {
            return Err(SpeedtestError::NoData(format!(
                "ping endpoint returned {}",
                resp.status()
            )));
        }
        total_ms += start.elapsed().as_secs_f64() * 1000.0;
    }
    Ok(total_ms / PING_ROUNDS as f64)
}

/// Single download stream: pulls bytes until `stop` fires, adds to `counter`.
/// Gets its own Client so it has an independent TCP connection + congestion window.
/// Only counts bodies from a 2xx response
async fn download_stream(
    stop: Arc<AtomicBool>,
    counter: Arc<AtomicU64>,
    measure_flag: Arc<AtomicBool>,
    rate_limited: Arc<AtomicBool>,
) {
    let client = match Client::builder().no_gzip().build() {
        Ok(c) => c,
        Err(_) => return,
    };
    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let resp = match client.get(DOWNLOAD_URL).send().await {
            Ok(r) => r,
            Err(e) => {
                debug!("download stream error, retrying: {e}");
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
        };
        if !resp.status().is_success() {
            //back off hard on 429: retrying immediately sustains
            //rate limit and the 1-byte error body is not payload
            let backoff = if is_rate_limited(resp.status()) {
                rate_limited.store(true, Ordering::Relaxed);
                Duration::from_secs(2)
            } else {
                Duration::from_millis(200)
            };
            debug!("download rejected with {}, backing off", resp.status());
            tokio::time::sleep(backoff).await;
            continue;
        }
        let mut stream = resp.bytes_stream();
        loop {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            match stream.next().await {
                Some(Ok(chunk)) => {
                    if measure_flag.load(Ordering::Acquire) {
                        counter.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                    }
                }
                Some(Err(e)) => {
                    debug!("download chunk error: {e}");
                    break;
                }
                None => break,
            }
        }
    }
}

/// Single upload stream: POSTs chunks until `stop` fires, adds to `counter`.
async fn upload_stream(
    stop: Arc<AtomicBool>,
    counter: Arc<AtomicU64>,
    measure_flag: Arc<AtomicBool>,
    rate_limited: Arc<AtomicBool>,
) {
    let client = match Client::builder().no_gzip().build() {
        Ok(c) => c,
        Err(_) => return,
    };
    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let payload = UPLOAD_PAYLOAD.clone();
        let len = payload.len() as u64;
        match client
            .post(UPLOAD_URL)
            .header("Content-Type", "application/octet-stream")
            .body(payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let _ = resp.bytes().await;
                if !status.is_success() {
                    let backoff = if is_rate_limited(status) {
                        rate_limited.store(true, Ordering::Relaxed);
                        Duration::from_secs(2)
                    } else {
                        Duration::from_millis(200)
                    };
                    debug!("upload rejected with {status}, backing off");
                    tokio::time::sleep(backoff).await;
                    continue;
                }
                if measure_flag.load(Ordering::Acquire) {
                    counter.fetch_add(len, Ordering::Relaxed);
                }
            }
            Err(e) => {
                debug!("stream error, retrying: {e}");
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
        }
    }
}

/// Spawn `PARALLEL_STREAMS` tasks, let them run for `duration`, then stop and collect bytes.
/// Uses `tokio::spawn` so each stream runs on its own OS thread (requires multi-thread runtime).
async fn run_persistent<F, Fut>(make_fut: F) -> (u64, Duration, bool)
where
    F: Fn(Arc<AtomicBool>, Arc<AtomicU64>, Arc<AtomicBool>, Arc<AtomicBool>) -> Fut,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let counter = Arc::new(AtomicU64::new(0));
    let measure_flag = Arc::new(AtomicBool::new(false));
    let rate_limited = Arc::new(AtomicBool::new(false));

    let handles: Vec<_> = (0..PARALLEL_STREAMS)
        .map(|_| {
            tokio::spawn(make_fut(
                Arc::clone(&stop),
                Arc::clone(&counter),
                Arc::clone(&measure_flag),
                Arc::clone(&rate_limited),
            ))
        })
        .collect();

    tokio::time::sleep(Duration::from_secs(WARMUP_SECS)).await;

    counter.store(0, Ordering::Release);
    let start = Instant::now();
    measure_flag.store(true, Ordering::Release);
    tokio::time::sleep(Duration::from_secs(MEASURE_SECS)).await;

    stop.store(true, Ordering::Relaxed);

    for h in handles {
        let _ = h.await;
    }

    (
        counter.load(Ordering::Relaxed),
        start.elapsed(),
        rate_limited.load(Ordering::Relaxed),
    )
}

/// minimum bytes a window must collect to be a plausible sample. Below this the
/// streams were effectively staled, and dividing by the window would report a
/// near-zero rate as though it were a real measurement
const MIN_SAMPLE_BYTES: u64 = 1_000_000;

fn to_mbps(
    label: &str,
    bytes: u64,
    elapsed: Duration,
    rate_limited: bool,
) -> Result<f64, SpeedtestError> {
    if rate_limited && bytes < MIN_SAMPLE_BYTES {
        return Err(SpeedtestError::NoData(format!(
            "{label} rate-limited by server (collected {bytes} bytes)"
        )));
    }
    if bytes < MIN_SAMPLE_BYTES {
        return Err(SpeedtestError::NoData(format!(
            "{label} collected only {bytes} bytes in {:.1}s",
            elapsed.as_secs_f64()
        )));
    }
    Ok(bytes as f64 * 8.0 / (elapsed.as_secs_f64() * 1_000_000.0))
}

async fn measure_download() -> Result<f64, SpeedtestError> {
    let (bytes, elapsed, rate_limited) = run_persistent(download_stream).await;
    to_mbps("download", bytes, elapsed, rate_limited)
}

async fn measure_upload() -> Result<f64, SpeedtestError> {
    let (bytes, elapsed, rate_limited) = run_persistent(upload_stream).await;
    to_mbps("upload", bytes, elapsed, rate_limited)
}

pub async fn run() -> Result<SpeedtestResult, SpeedtestError> {
    let client = Client::new();

    let ping_ms = measure_ping(&client).await?;
    debug!("Speedtest: ping {:.2} ms", ping_ms);

    let download_mbps = measure_download().await?;
    debug!("Speedtest: download {:.2} Mbit/s", download_mbps);

    let upload_mbps = measure_upload().await?;
    debug!("Speedtest: upload {:.2} Mbit/s", upload_mbps);

    debug!(
        "Speedtest: ↓ {:.1} Mbit/s  ↑ {:.1} Mbit/s  ping {:.1} ms",
        download_mbps, upload_mbps, ping_ms
    );

    Ok(SpeedtestResult {
        download_mbps,
        upload_mbps,
        ping_ms,
        collected_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore = "due to external network dependency"]
    // could fail due to external network
    // multi_thread is required: tokio::spawn needs real OS threads so each
    // download stream has its own thread and doesn't starve the others.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_full_speedtest() {
        let result = run().await.expect("speedtest failed");
        println!(
            "Download: {:.2} Mbit/s | Upload: {:.2} Mbit/s | Ping: {:.2} ms",
            result.download_mbps, result.upload_mbps, result.ping_ms
        );
        assert!(
            result.download_mbps > 1.0,
            "got {:.2}",
            result.download_mbps
        );
        assert!(result.upload_mbps > 1.0, "got {:.2}", result.upload_mbps);
        assert!(result.ping_ms > 0.0 && result.ping_ms < 2000.0);
    }
}
