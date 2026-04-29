use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{metadata::MetadataValue, transport::{Channel, ClientTlsConfig}, Request};

use crate::grpc::connection_proto::{
    MetricsResponse,
    metrics_service_client::MetricsServiceClient,
};


pub struct Client {
    url: &'static str,
    api_key: String,
    connection_retries: u8,
}

impl Client {
    pub fn new(url: &'static str, api_key: String) -> Self {
        Self { url, api_key, connection_retries: 5 }
    }

    pub async fn run(&self) -> Result<(), tonic::Status> {
        let channel = self.connect_with_retries().await
            .map_err(|e| tonic::Status::unavailable(e.to_string()))?;

        let mut client = MetricsServiceClient::new(channel);

        // creates the tx and rx for the metrics
        let (tx, rx) = mpsc::channel::<MetricsResponse>(16);
        // wrappes it into a stream to hand to watch-tower
        let outbound = ReceiverStream::new(rx);

        // creates the request with the api key
        let mut request = Request::new(outbound);
        request.metadata_mut().insert(
            "api_key",
            MetadataValue::try_from(self.api_key.as_str())
                .map_err(|e| tonic::Status::invalid_argument(format!("invalid api_key: {e}")))?,
        );

        // send our receiver stream info to watch-tower
        let response = client.base_transfer(request).await?;
        // get the inner stream to receive the request data
        let mut inbound = response.into_inner();

        while let Some(result) = inbound.next().await{
            match result {
                Ok(req_data) => {
                    log::debug!("received request: {:?}", req_data);
                    if tx.send(MetricsResponse{
                        request_id: req_data.request_id.clone(),
                        cpu_usage_percent: 10.0
                    }).await.is_err() {
                        log::error!("response channel failed to send data");
                        break;
                    }
                }
                Err(e) => {
                    log::error!("Error receiving metrics response: {}", e);
                    return Err(tonic::Status::internal(e.to_string()))
                },
            }
        }

        // return because server closed the stream (for now) should retry to establish the connection and start the server again
        Ok(())
    }
    
    async fn connect_with_retries(&self) -> Result<Channel, tonic::transport::Error> {
        let mut last_err = None;
        for _attempt in 1..=self.connection_retries {
            match Self::connect_socket(self.url).await {
                Ok(channel) => {
                    log::info!("Connected to the server at {}", self.url);
                    return Ok(channel);
                }
                Err(e) => {
                    log::error!("Error connecting to grpc server: {} with error: {}", self.url, e);
                    last_err = Some(e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
        Err(last_err.unwrap())
    }

    async fn connect_socket(url: &'static str) -> Result<Channel, tonic::transport::Error> {
        let endpoint = Channel::from_static(url)
            .keep_alive_while_idle(true)
            .http2_keep_alive_interval(Duration::from_secs(15))
            .keep_alive_timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(15))
            .buffer_size(256);

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
    use std::time::SystemTime;
    use super::*;
    use tokio::sync::mpsc;
    use tokio_stream::{wrappers::ReceiverStream, StreamExt};
    use tonic::{metadata::MetadataValue, Request};
    use crate::grpc::connection_proto::{
        MetricsResponse,
        metrics_service_client::MetricsServiceClient,
    };
    use crate::logging::init_logging;

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

    #[tokio::test]
    async fn run_grpc_client() {
        init_logging();
        let system_time = SystemTime::now();
        let client = Client::new(SERVER_URL, "KNlj1kJhA1pa8cj6DAmlLyvO4XRhOY2tC0HwNeKMh4o".to_string());
        let time = system_time.elapsed().unwrap();
        let res = client.run().await;
        let time = system_time.elapsed().unwrap();
        log::info!("elapsed time: {:?}", time);
        assert!(res.is_ok());
    }
}
