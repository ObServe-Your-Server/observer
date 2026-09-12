use std::time::Duration;
use criterion::{criterion_group, criterion_main, Criterion};
use nix::libc::malloc_good_size;
use open_eye::collector::container_runtime::collector::{ContainerRuntime, ContainerRuntimeStats};
use open_eye::collector::cpu::collector::CpuStats;

fn bench_cpu_collector(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_collector");
    let sample_sizes = [10, 20];
    for size in sample_sizes {
        group.sample_size(size);
        group.warm_up_time(Duration::from_secs(1));
        group.bench_function(format!("{} runs", size), |b| {
            b.iter(|| CpuStats::get_current_stats());
        });
    }
}

fn bench_container_runtime_collector(c: &mut Criterion) {
    let mut group = c.benchmark_group("container_runtime_collector");
    let sample_sizes = [10, 20];

    for size in sample_sizes {
        group.sample_size(size);
        group.warm_up_time(Duration::from_secs(1));
        group.bench_function(format!("{} runs", size), |b| {
            b.iter(|| ContainerRuntimeStats::get_current_stats());
        });
    }
}

criterion_group!(benches, bench_cpu_collector, bench_container_runtime_collector);
criterion_main!(benches);