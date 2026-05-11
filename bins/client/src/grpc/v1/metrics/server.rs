use std::sync::{Arc, Mutex};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{async_trait, Request, Response, Status, Streaming};
use tonic::metadata::{Ascii, MetadataValue};
use tonic::transport::Server;
use crate::grpc::v1::metrics_tunnel_server::MetricsTunnel;
use crate::grpc::v1::{MetricsRequest, MetricsResponse, MetricsType};

#[derive(Debug)]
pub struct ClientServer{
    address: String,
    cmd_tx: tokio::sync::mpsc::Sender<MetricsRequest>,
    resp_tx:
}

#[async_trait]
impl MetricsTunnel for ClientServer {
    type BaseTransferStream = ReceiverStream<Result<MetricsRequest, Status>>;

    async fn base_transfer(
        &self,
        request: Request<Streaming<MetricsResponse>>,
    ) -> Result<Response<Self::BaseTransferStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);

        let mut inbound = request.into_inner();
        // receive metrics coming back from the client
        tokio::spawn(async move {
            while let Some(result) = inbound.next().await {
                match result {
                    Ok(response) => { /* store, forward, log, etc. */ }
                    Err(e) => break,
                }
            }
        });

        tokio::spawn(async move {
            // send a metrics request
            tx.send(Ok(MetricsRequest {
                request_id: "test-request-id".to_string(),
                r#type: MetricsType::Cpu as i32,
                requested_at: Some(prost_types::Timestamp::date_time(2000, 1, 1,1,1, 1).unwrap()),
            }))
            .await
            .ok();
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

async fn create_grpc_server(url: &'static str) -> tokio::task::JoinHandle<()> {
    use crate::grpc::v1::metrics_tunnel_server::MetricsTunnelServer;
    use crate::grpc::v1::metrics::server::ClientServer;

    tokio::spawn(async {
        Server::builder()
            .add_service(MetricsTunnelServer::with_interceptor(ClientServer, auth_interceptor))
            .serve(url.parse().unwrap())
            .await
            .unwrap();
    })
}

fn auth_interceptor(request: Request<()>) -> Result<Request<()>, Status> {
    match request.metadata().get("api_key") {
        Some(key) => {
            if key == "test-key" {
                return Ok(request);
            }
            Err(Status::unauthenticated("Invalid API key"))
        }
        _ => Err(Status::unauthenticated("Missing API key")),
    }
}