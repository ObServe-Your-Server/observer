use std::{cell::LazyCell, sync::Arc};

use tokio::sync::RwLock;

pub mod collector;
pub mod docker_metric_sender;
pub mod docker_job;
/*
 * enum DockerCollectorStatus {
     Started,
     Stopped,
 }
 
 struct DockerCollectorStatus {
     
 }
 
 static DOCKER_COLLECTOR_STATUS: LazyCell<Arc<RwLock<>>>
 */
// TODO
