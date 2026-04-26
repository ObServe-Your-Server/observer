use std::time::Duration;
use tonic::transport::{Channel, ClientTlsConfig};
use crate::grpc::metrics_proto::{Metrics, metrics_service_client::MetricsServiceClient};

pub struct Sender {
    url: &'static str,
    connection_retries: u8,
    channel: Channel,
}

impl Sender {
    pub async fn new(url: &'static str, connection_retries: Option<u8>) -> Result<Self, tonic::transport::Error> {
        let connection_retries = connection_retries.unwrap_or(5);
        let mut last_err = None;
        for attempt in 1..=connection_retries {
            match Self::connect_socket(url).await {
                Ok(channel) => return Ok(Self { url, channel, connection_retries }),
                Err(e) => {
                    eprintln!("Connection attempt {attempt} failed: {e}");
                    last_err = Some(e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
        Err(last_err.unwrap())
    }

    pub async fn reconnect(&mut self) -> Result<(), tonic::transport::Error> {
        let mut last_err = None;
        for attempt in 1..=self.connection_retries {
            match Self::connect_socket(self.url).await {
                Ok(channel) => {
                    self.channel = channel;
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Reconnect attempt {attempt} failed: {e}");
                    last_err = Some(e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
        Err(last_err.unwrap())
    }

    pub async fn push_metrics(&self, metrics: Metrics) -> Result<(), tonic::Status> {
        let mut client = MetricsServiceClient::new(self.channel.clone());
        client.push_metrics(metrics).await.map(|_| ())
    }

    async fn connect_socket(url: &'static str) -> Result<Channel, tonic::transport::Error> {
        let endpoint = Channel::from_static(url)
            .keep_alive_while_idle(true)
            .http2_keep_alive_interval(Duration::from_secs(15))
            .keep_alive_timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(15));

        let endpoint = if url.starts_with("https://") {
            endpoint.tls_config(ClientTlsConfig::new().with_native_roots())?
        } else {
            endpoint
        };

        endpoint.connect().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore = "requires a grpc server running"]
    #[tokio::test]
    async fn test_connection() {
        let sender = Sender::new("http://localhost:50051", Some(1)).await;
        assert!(sender.is_ok(), "failed to connect: {:?}", sender.err());
    }

    #[ignore = "requires a grpc server running"]
    #[tokio::test]
    async fn test_push_metrics() {
        let sender = Sender::new("http://localhost:50051", Some(1)).await
            .expect("failed to connect");

        let metrics = Metrics {
            metrics: vec![],
            memory: vec![],
            disk: vec![],
            network: vec![],
            system_stats: vec![],
            recorded_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        };

        let result = sender.push_metrics(metrics).await;
        assert!(result.is_ok(), "failed to push metrics: {:?}", result.err());
    }
}