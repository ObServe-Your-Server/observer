use std::time::Duration;
use tonic::transport::{Channel, ClientTlsConfig};

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
    #[test]
    fn test_connection() {

    }
}