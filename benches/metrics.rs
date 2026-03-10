use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::Duration;
use observer::client::host::system_metric_collection::{Metrics, collection_job};

fn bench_metrics_collect(c: &mut Criterion) {
    c.bench_function("metrics_collect", |b| {
        b.iter(|| black_box(Metrics::do_collect()))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .measurement_time(Duration::from_secs(30));
    targets = bench_metrics_collect
}
criterion_main!(benches);
