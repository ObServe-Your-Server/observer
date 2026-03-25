/*
use criterion::{Criterion, criterion_group, criterion_main};
use observer::client::host::system_metric_collection::{Metrics, collection_job};
use std::time::Duration;

fn bench_metrics_collect(c: &mut Criterion) {
    c.bench_function("metrics_collect", |b| {
        // not the right job because it only spans the thread Metrics::do_collect() or so is right
        //b.iter(|| collection_job())
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
*/