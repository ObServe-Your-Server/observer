use crate::entities::prelude::TunnelAccessLog;

pub trait AccessLogStore{
    fn save_tunnel_access_log(tunnel_access_log: TunnelAccessLog) -> anyhow::Result<()>;
}