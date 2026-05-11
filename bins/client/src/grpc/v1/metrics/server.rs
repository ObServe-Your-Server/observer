use tokio_stream::wrappers::ReceiverStream;
use tonic::{async_trait, Request, Response, Status, Streaming};
use crate::grpc::v1::metrics_tunnel_server::MetricsTunnel;
use crate::grpc::v1::{MetricsRequest, MetricsResponse, MetricsType};

#[derive(Debug)]
pub struct ClientServer;

#[async_trait]
impl MetricsTunnel for ClientServer {
    type BaseTransferStream = ReceiverStream<Result<MetricsRequest, Status>>;

    async fn base_transfer(
        &self,
        _request: Request<Streaming<MetricsResponse>>,
    ) -> Result<Response<Self::BaseTransferStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);
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
