use futures_util::StreamExt;
use log::{debug, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

const DOWNLOAD_URL: &str = "https://speed.cloudflare.com/__down?bytes=90000000";
const UPLOAD_URL: &str = "https://speed.cloudflare.com/__up";
const PING_URL: &str = "https://speed.cloudflare.com/__ping";

const PING_ROUNDS: u32 = 5;

// 8 parallel streams, each on its own OS thread via tokio::spawn on a multi-thread
// runtime. This saturates the link the same way a browser download does.
const PARALLEL_STREAMS: usize = 8;
const WARMUP_SECS: u64 = 4;
const MEASURE_SECS: u64 = 8;

const UPLOAD_CHUNK: usize = 10_000_000; // 10 MB per POST

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedtestResult {
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
}

#[derive(Debug)]
pub enum SpeedtestError {
    Http(reqwest::Error),
    Stream(String),
}

impl std::fmt::Display for SpeedtestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeedtestError::Http(e) => write!(f, "HTTP error: {}", e),
            SpeedtestError::Stream(s) => write!(f, "Stream error: {}", s),
        }
    }
}

impl std::error::Error for SpeedtestError {}

impl From<reqwest::Error> for SpeedtestError {
    fn from(e: reqwest::Error) -> Self {
        SpeedtestError::Http(e)
    }
}

async fn measure_ping(client: &Client) -> Result<f64, SpeedtestError> {
    // Warmup: establish TLS + HTTP/2 connection.
    client.get(PING_URL).send().await?;

    let mut total_ms = 0.0;
    for _ in 0..PING_ROUNDS {
        let start = Instant::now();
        client.get(PING_URL).send().await?;
        total_ms += start.elapsed().as_secs_f64() * 1000.0;
    }
    Ok(total_ms / PING_ROUNDS as f64)
}

/// Single download stream: pulls bytes until `stop` fires, adds to `counter`.
/// Gets its own Client so it has an independent TCP connection + congestion window.
async fn download_stream(
    stop: Arc<AtomicBool>,
    counter: Arc<AtomicU64>,
    measure_flag: Arc<AtomicBool>,
) {
    let client = match Client::builder()
        .no_gzip()
        .pool_max_idle_per_host(0)
        .build()
    {
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
) {
    let client = match Client::builder()
        .no_gzip()
        .pool_max_idle_per_host(0)
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let payload = vec![0u8; UPLOAD_CHUNK];
        let len = payload.len() as u64;
        match client
            .post(UPLOAD_URL)
            .header("Content-Type", "application/octet-stream")
            .body(payload)
            .send()
            .await
        {
            Ok(resp) => {
                let _ = resp.bytes().await;
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
async fn run_persistent<F, Fut>(make_fut: F) -> (u64, Duration)
where
    F: Fn(Arc<AtomicBool>, Arc<AtomicU64>, Arc<AtomicBool>) -> Fut,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let counter = Arc::new(AtomicU64::new(0));
    let measure_flag = Arc::new(AtomicBool::new(false));

    let handles: Vec<_> = (0..PARALLEL_STREAMS)
        .map(|_| {
            tokio::spawn(make_fut(
                Arc::clone(&stop),
                Arc::clone(&counter),
                Arc::clone(&measure_flag),
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

    (counter.load(Ordering::Relaxed), start.elapsed())
}

async fn measure_download() -> Result<f64, SpeedtestError> {
    let (bytes, elapsed) = run_persistent(download_stream).await;
    Ok(bytes as f64 * 8.0 / (elapsed.as_secs_f64() * 1_000_000.0))
}

async fn measure_upload() -> Result<f64, SpeedtestError> {
    let (bytes, elapsed) = run_persistent(upload_stream).await;
    Ok(bytes as f64 * 8.0 / (elapsed.as_secs_f64() * 1_000_000.0))
}

pub async fn run() -> Result<SpeedtestResult, SpeedtestError> {
    let client = Client::new();

    let ping_ms = measure_ping(&client).await?;
    debug!("Speedtest: ping {:.2} ms", ping_ms);

    let download_mbps = measure_download().await?;
    debug!("Speedtest: download {:.2} Mbit/s", download_mbps);

    let upload_mbps = measure_upload().await?;
    debug!("Speedtest: upload {:.2} Mbit/s", upload_mbps);

    info!(
        "Speedtest: ↓ {:.1} Mbit/s  ↑ {:.1} Mbit/s  ping {:.1} ms",
        download_mbps, upload_mbps, ping_ms
    );

    Ok(SpeedtestResult {
        download_mbps,
        upload_mbps,
        ping_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
