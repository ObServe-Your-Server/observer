use log::info;
use sysinfo::{Networks, System};

pub async fn collect() {
    let mut sys = System::new_all();
    sys.refresh_all();

    // --- CPU ---
    let cpu_count = sys.cpus().len();
    let cpu_usage: f32 = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / cpu_count as f32;
    info!("CPU: {:.1}% across {} cores", cpu_usage, cpu_count);

    // --- Memory ---
    let total_mb = sys.total_memory() / 1024 / 1024;
    let used_mb = sys.used_memory() / 1024 / 1024;
    info!("RAM: {}MB used / {}MB total", used_mb, total_mb);

    // --- Network ---
    let networks = Networks::new_with_refreshed_list();
    for (interface, data) in &networks {
        info!(
            "NET [{}]: down={}B up={}B",
            interface,
            data.total_received(),
            data.total_transmitted(),
        );
    }
}
