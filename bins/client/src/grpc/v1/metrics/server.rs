use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{async_trait, Request, Response, Status, Streaming};
use crate::grpc::v1::metrics_tunnel_server::MetricsTunnel;
use crate::grpc::v1::{MetricsRequest, MetricsResponse};

// What external code receives per connected client.
// cmd_tx  → push MetricsRequests to this client
// resp_rx → read MetricsResponses back from this client
pub struct ConnectionHandle {
    pub client_id: String,
    pub cmd_tx: mpsc::Sender<MetricsRequest>,
    pub resp_rx: mpsc::Receiver<MetricsResponse>,
}

pub struct ClientServer {
    new_conn_tx: mpsc::Sender<ConnectionHandle>,
}

impl ClientServer {
    pub fn new() -> (Self, mpsc::Receiver<ConnectionHandle>) {
        let (new_conn_tx, new_conn_rx) = mpsc::channel(16);
        (Self { new_conn_tx }, new_conn_rx)
    }
}

#[async_trait]
impl MetricsTunnel for ClientServer {
    type BaseTransferStream = ReceiverStream<Result<MetricsRequest, Status>>;

    async fn base_transfer(
        &self,
        request: Request<Streaming<MetricsResponse>>,
    ) -> Result<Response<Self::BaseTransferStream>, Status> {
        let client_id = request
            .metadata()
            .get("api_key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        // all three channel pairs are per-connection
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<MetricsRequest>(16);
        let (resp_tx, resp_rx) = mpsc::channel::<MetricsResponse>(16);
        let (stream_tx, stream_rx) = mpsc::channel::<Result<MetricsRequest, Status>>(16);

        // hand the outward-facing ends to whoever is managing connections
        self.new_conn_tx
            .send(ConnectionHandle { client_id, cmd_tx, resp_rx })
            .await
            .ok();

        // bridge inbound wire -> resp_tx (responses from client to external handler)
        let mut inbound = request.into_inner();
        tokio::spawn(async move {
            while let Some(Ok(resp)) = inbound.next().await {
                if resp_tx.send(resp).await.is_err() {
                    break;
                }
            }
        });

        // bridge cmd_rx -> outbound wire (commands from external code to client)
        tokio::spawn(async move {
            while let Some(req) = cmd_rx.recv().await {
                if stream_tx.send(Ok(req)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(stream_rx)))
    }
}