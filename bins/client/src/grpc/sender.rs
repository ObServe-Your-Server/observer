use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{metadata::MetadataValue, transport::{Channel, ClientTlsConfig}, Request};

use crate::grpc::connection_proto::{
    MetricsResponse, RequestData,
    metrics_service_client::MetricsServiceClient,
};
use crate::subsystem::host_metrics_collector::HostMetrics;


pub struct Sender {
    url: &'static str,
    api_key: String,
    connection_retries: u8,
    channel: Channel,
}

impl Sender {
    pub async fn new(
        url: &'static str,
        api_key: String,
    ) -> Result<Self, tonic::transport::Error> {
        let connection_retries = 5;
        let mut last_err = None;
        
        // attempt to connect
        for attempt in 1..=connection_retries {
            match Self::connect_socket(url).await {
                Ok(channel) => {
                    log::info!("Connected to the server at {}", url);
                    return Ok(Self { url, api_key, channel, connection_retries })
                },
                Err(e) => {
                    log::error!("Error connecting to grpc server: {} with error: {}", url, e);
                    last_err = Some(e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
        Err(last_err.unwrap())
    }
    
    pub async fn run(&self) -> Result<(), tonic::Status> {
        // create a new client instance
        let mut client = MetricsServiceClient::new(self.channel.clone());

        let (tx, rx) = mpsc::channel::<MetricsResponse>(16);
        let outbound = ReceiverStream::new(rx);

        let mut request = Request::new(outbound);
        request.metadata_mut().insert(
            "api_key",
            MetadataValue::try_from(self.api_key.as_str())
                .map_err(|e| tonic::Status::invalid_argument(format!("invalid api_key: {e}")))?,
        );

        let response = client.base_transfer(request).await?;
        let mut inbound = response.into_inner();

        Ok(())
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
    use tokio::sync::mpsc;
    use tokio_stream::{wrappers::ReceiverStream, StreamExt};
    use tonic::{metadata::MetadataValue, Request};
    use crate::grpc::connection_proto::{
        MetricsResponse,
        metrics_service_client::MetricsServiceClient,
    };

    const SERVER_URL: &str = "http://localhost:50051";

    #[ignore = "requires a grpc server running"]
    #[tokio::test]
    async fn test_base_transfer_with_server() {
        let channel = tonic::transport::Channel::from_static(SERVER_URL)
            .keep_alive_while_idle(true)
            .connect()
            .await
            .expect("failed to connect to server");

        let mut client = MetricsServiceClient::new(channel);

        let (resp_tx, resp_rx) = mpsc::channel(4);
        let outbound = ReceiverStream::new(resp_rx);

        let mut request = Request::new(outbound);
        request.metadata_mut().insert(
            "api_key",
            MetadataValue::try_from("test-key").unwrap(),
        );

        let mut inbound = client
            .base_transfer(request)
            .await
            .expect("base_transfer call failed")
            .into_inner();

        let request_data = inbound
            .next()
            .await
            .expect("server closed stream without sending")
            .expect("stream error");

        println!("Received RequestData: id={:?}", request_data.request_id);

        resp_tx
            .send(MetricsResponse {
                request_id: request_data.request_id.clone(),
                cpu_usage_percent: 12.3,
            })
            .await
            .expect("failed to send response");

        tokio::time::sleep(Duration::from_secs(1)).await;
        println!("Sent MetricsResponse for request_id={}", request_data.request_id);
    }
}
