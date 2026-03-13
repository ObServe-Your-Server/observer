use futures_util::StreamExt;
use log::{debug, info, warn};
use reqwest::Client;
use serde::Serialize;
use std::sync::RwLock;
use std::time::Instant;

use crate::client::metric_collection_errors::CollectionError;

const DOWNLOAD_URL: &str = "https://speed.cloudflare.com/__down?bytes=10000000"; // 10MB
const UPLOAD_URL: &str = "https://speed.cloudflare.com/__up";
const PING_URL: &str = "https://speed.cloudflare.com/__ping";
const PING_ROUNDS: u32 = 5;
const DOWNLOAD_ROUNDS: u32 = 3;
const UPLOAD_SIZE: usize = 10_000_000; // 10MB

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedtestResult {
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub ping_ms: f64,
}

// last speedtest result - None until the first measurement completes
static LAST_RESULT: RwLock<Option<SpeedtestResult>> = RwLock::new(None);

pub fn get_last_result() -> Option<SpeedtestResult> {
    LAST_RESULT.read().unwrap().clone()
}

async fn measure_ping(client: &Client) -> Result<f64, String> {
    // warmup request to establish the TLS connection and warm up HTTP/2 multiplexing
    client
        .get(PING_URL)
        .send()
        .await
        .map_err(|e| format!("Ping warmup failed: {}", e))?;

    let mut total_ms = 0.0;
    for _ in 0..PING_ROUNDS {
        let start = Instant::now();
        client
            .get(PING_URL)
            .send()
            .await
            .map_err(|e| format!("Ping request failed: {}", e))?;
        total_ms += start.elapsed().as_secs_f64() * 1000.0;
    }

    Ok(total_ms / PING_ROUNDS as f64)
}

async fn measure_download_once(client: &Client) -> Result<f64, String> {
    let start = Instant::now();
    let response = client
        .get(DOWNLOAD_URL)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;
    let ttfb = start.elapsed();

    let mut stream = response.bytes_stream();
    let mut bytes_received: usize = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        bytes_received += chunk.len();
    }

    // subtract TTFB so we measure transfer time only, not connection overhead
    let elapsed = (start.elapsed() - ttfb).as_secs_f64();
    Ok((bytes_received as f64 * 8.0) / (elapsed * 1_000_000.0))
}

async fn measure_download(client: &Client) -> Result<f64, String> {
    let mut total = 0.0;
    for _ in 0..DOWNLOAD_ROUNDS {
        total += measure_download_once(client).await?;
    }
    Ok(total / DOWNLOAD_ROUNDS as f64)
}

async fn measure_upload(client: &Client) -> Result<f64, String> {
    // fill a buffer with zeros as upload payload
    let payload = vec![0u8; UPLOAD_SIZE];

    // reqwest streams the body while sending; elapsed until response arrives
    // is the best approximation of upload time available without raw socket access
    let start = Instant::now();
    let response = client
        .post(UPLOAD_URL)
        .body(payload)
        .send()
        .await
        .map_err(|e| format!("Upload request failed: {}", e))?;
    let elapsed = start.elapsed().as_secs_f64();

    // consume response to avoid connection issues, but don't count its time
    drop(response);

    Ok((UPLOAD_SIZE as f64 * 8.0) / (elapsed * 1_000_000.0))
}

// TODO: Add error handling
pub async fn run() -> Result<(), CollectionError> {
    info!(
        "Starting speedtest against Cloudflare ({} download rounds)...",
        DOWNLOAD_ROUNDS
    );

    let client = Client::new();

    let ping = match measure_ping(&client).await {
        Ok(v) => {
            debug!("Ping: {:.1}ms (avg over {} rounds)", v, PING_ROUNDS);
            Some(v)
        }
        Err(e) => {
            warn!("Ping failed: {}", e);
            None
        }
    };

    let download = match measure_download(&client).await {
        Ok(v) => {
            debug!("Download: {:.2} Mbps", v);
            Some(v)
        }
        Err(e) => {
            warn!("Download failed: {}", e);
            None
        }
    };

    let upload = match measure_upload(&client).await {
        Ok(v) => {
            debug!("Upload: {:.2} Mbps", v);
            Some(v)
        }
        Err(e) => {
            warn!("Upload failed: {}", e);
            None
        }
    };

    // only store if all three measurements succeeded
    if let (Some(ping_ms), Some(download_mbps), Some(upload_mbps)) = (ping, download, upload) {
        *LAST_RESULT.write().unwrap() = Some(SpeedtestResult {
            download_mbps,
            upload_mbps,
            ping_ms,
        });
    }

    Ok(())
}
