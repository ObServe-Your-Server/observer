// Run benchmarks (timing only):
//   cargo bench --manifest-path bins/client/Cargo.toml --bench storage_engine
//
// Run with flamegraph (generates target/criterion/<bench>/profile/flamegraph.svg):
//   cargo bench --manifest-path bins/client/Cargo.toml --bench storage_engine -- --profile-time 10

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use observer_client::data_storage::file_format::metrics_file::MetricsFile;
use observer_client::data_storage::storage_engine::StorageEngine;
use open_eye::collector::cpu::collector::CpuStats;
use pprof::criterion::{Output, PProfProfiler};
use std::time::Duration;

// how fast is a single HashMap insert + Vec push, should be nearly free.
fn bench_add_to_queue(c: &mut Criterion) {
    let cpu_data = CpuStats::get_current_stats();

    c.bench_function("add_data_10000_entries_to_queue", |b| {
        b.iter_batched(
            // setup: fresh engine, nothing measured yet
            || StorageEngine::with_base_folder(tempfile::tempdir().unwrap().into_path()),
            // measured: pushing 100 items into the in-memory channel
            |mut engine| {
                for _ in 0..10000 {
                    engine.add_data(cpu_data.clone());
                }
            },
            BatchSize::PerIteration,
        );
    });
}

// how long does building a MetricsFile take
// serialization + checksum per entry without any file I/O.
fn bench_build_metrics_file(c: &mut Criterion) {
    let cpu_data = CpuStats::get_current_stats();

    c.bench_function("build_metrics_file_10000_entries", |b| {
        b.iter_batched(
            // setup: collect 100 entries, not measured
            || vec![cpu_data.clone(); 10000],
            // measured: turning those entries into a MetricsFile (no disk write)
            |entries| {
                MetricsFile::with_data(entries).unwrap();
            },
            BatchSize::PerIteration,
        );
    });
}

// full pipeline with a small batch
fn bench_save_normal(c: &mut Criterion) {
    let cpu_data = CpuStats::get_current_stats();

    c.bench_function("save_to_file_3_entries", |b| {
        b.iter_batched(
            // setup: engine with 3 entries ready, dir kept alive so it exists on disk
            || {
                let dir = tempfile::tempdir().unwrap();
                let mut engine = StorageEngine::with_base_folder(dir.path().to_path_buf());
                for _ in 0..3 {
                    engine.add_data(cpu_data.clone());
                }
                (engine, dir)
            },
            // measured: serialize + checksum + write to disk
            |(mut engine, _dir)| {
                engine.save_to_file::<CpuStats>().unwrap();
            },
            BatchSize::PerIteration,
        );
    });
}

// ful pipeline with a large batch
fn bench_save_big(c: &mut Criterion) {
    let cpu_data = CpuStats::get_current_stats();

    c.bench_function("save_to_file_10000_entries", |b| {
        b.iter_batched(
            // setup: engine with 100 entries ready
            || {
                let dir = tempfile::tempdir().unwrap();
                let mut engine = StorageEngine::with_base_folder(dir.path().to_path_buf());
                for _ in 0..10000 {
                    engine.add_data(cpu_data.clone());
                }
                (engine, dir)
            },
            // measured: serialize + checksum + write to disk
            |(mut engine, _dir)| {
                engine.save_to_file::<CpuStats>().unwrap();
            },
            BatchSize::PerIteration,
        );
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(10))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_add_to_queue, bench_build_metrics_file, bench_save_normal, bench_save_big
}
criterion_main!(benches);
