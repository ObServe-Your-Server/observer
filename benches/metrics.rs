use criterion::{Criterion, criterion_group, criterion_main};
use observer::client::metric_collection::Metrics;
use std::hint::black_box;
use std::time::Duration;

fn bench_metrics_collect(c: &mut Criterion) {
    c.bench_function("metrics_collect", |b| {
        b.iter(|| black_box(Metrics::collect()))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(20));
    targets = bench_metrics_collect
}
criterion_main!(benches);
