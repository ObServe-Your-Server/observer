use log::debug;
use reqwest::Client;

use crate::client::metric_collection_errors::CollectionError;
use crate::system_health::HostSytemHealth;

// TODO: Error handling
pub async fn collect(host_sytem_health: HostSytemHealth) -> Result<(), CollectionError> {
    /*let containers = match list_containers(host_sytem_health).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(()),
        Err(e) => return Err(e),
    };
    debug!("Collected {} docker containers", containers.len());

    let client = Client::new();
    match send(&client, &containers).await {
        Ok(()) => Ok(()),
        Err(e) => Err(e),
    }*/
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(ignore)]
    #[tokio::test]
    async fn test_collect_once() {
        let containers = list_containers().await.unwrap();
        println!("collected {} containers", containers.len());
        for c in &containers {
            println!("---");
            println!("  id:              {}", c.id);
            println!("  name:            {}", c.host_name);
            println!("  image:           {}", c.image_name);
            println!("  status:          {}", c.status);
            println!("  running:         {}", c.running);
            println!("  uptime (s):      {}", c.running_for_seconds);
            println!("  created_at:      {}", c.created_at);
            println!("  networks:        {}", c.networks.join(", "));
            println!("  cpu %:           {:.2}", c.cpu_usage_percent);
            println!("  memory (bytes):  {}", c.memory_usage_bytes);
        }
    }
}
