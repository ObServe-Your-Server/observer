use serde::{Deserialize, Serialize};
use sysinfo::{ProcessesToUpdate, System};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessStats {
    pub pid: u32,
    pub name: String,
    pub user: String,
    pub status: String,
    pub cpu_usage_percent: f32,
    pub memory_usage_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessesStats {
    pub collected_at: chrono::DateTime<chrono::Utc>,
    pub top_cpu: Vec<ProcessStats>,
    pub top_memory: Vec<ProcessStats>,
}

impl ProcessStats {
    pub fn get_current_stats(top_n: Option<u8>) -> ProcessesStats {
        let top_n = top_n.unwrap_or(10) as usize;
        let mut sys = System::new_all();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let mut processes: Vec<ProcessStats> = sys
            .processes()
            .values()
            .map(|process| ProcessStats {
                pid: process.pid().as_u32(),
                name: process.name().to_string_lossy().to_string(),
                user: process
                    .user_id()
                    .map_or("unknown".to_string(), |u| u.to_string()),
                status: format!("{:?}", process.status()),
                cpu_usage_percent: process.cpu_usage(),
                memory_usage_bytes: process.memory(),
            })
            .collect();

        processes.sort_by(|a, b| b.cpu_usage_percent.total_cmp(&a.cpu_usage_percent));
        let top_cpu = processes.iter().take(top_n).cloned().collect::<Vec<_>>();

        processes.sort_by(|a, b| b.memory_usage_bytes.cmp(&a.memory_usage_bytes));
        let top_memory = processes.iter().take(top_n).cloned().collect::<Vec<_>>();

        ProcessesStats {
            collected_at: chrono::Utc::now(),
            top_cpu,
            top_memory,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn returns_non_empty_process_lists() {
        let stats = ProcessStats::get_current_stats(None);

        assert!(!stats.top_cpu.is_empty(), "top_cpu should not be empty");
        assert!(
            !stats.top_memory.is_empty(),
            "top_memory should not be empty"
        );
    }

    #[test]
    fn top_cpu_is_sorted_descending() {
        let stats = ProcessStats::get_current_stats(None);

        for window in stats.top_cpu.windows(2) {
            assert!(
                window[0].cpu_usage_percent >= window[1].cpu_usage_percent,
                "top_cpu not sorted: {} < {}",
                window[0].cpu_usage_percent,
                window[1].cpu_usage_percent,
            );
        }
    }

    #[test]
    fn top_memory_is_sorted_descending() {
        let stats = ProcessStats::get_current_stats(None);

        for window in stats.top_memory.windows(2) {
            assert!(
                window[0].memory_usage_bytes >= window[1].memory_usage_bytes,
                "top_memory not sorted: {} < {}",
                window[0].memory_usage_bytes,
                window[1].memory_usage_bytes,
            );
        }
    }
}
