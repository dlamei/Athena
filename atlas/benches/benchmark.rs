use std::time::Duration;

use atlas::{
    iso,
    iso2::{self, Iso2DConfig},
    iso3,
};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

fn tree_graph_2d_build(c: &mut Criterion) {
    // c.bench_function("tree_graph_2d_build_no_simd", |b| {
    //     b.iter(|| iso2::TreeGraph::build(black_box(config)))
    // });
    let mut group = c.benchmark_group("tree_graph_2d_build");
    group.measurement_time(Duration::from_secs(10));

    for depth in [6, 7, 8, 9, 10].iter() {
        group.throughput(Throughput::Bytes(*depth as u64));

        let config = Iso2DConfig {
            min: (-1.0, -1.0).into(),
            max: (1.0, 1.0).into(),
            intrvl_depth: *depth as u32,
            simd: false,
            program: iso2::Program::Dense2,
            ..Default::default()
        };

        group.bench_with_input(BenchmarkId::new("default", depth), depth, |b, &_| {
            b.iter(|| iso2::bench::build_tree_graph(black_box(config)));
        });
    }

    group.finish();
}

fn tree_graph_2d_collapse(c: &mut Criterion) {
    // c.bench_function("tree_graph_2d_build_no_simd", |b| {
    //     b.iter(|| iso2::TreeGraph::build(black_box(config)))
    // });
    let mut group = c.benchmark_group("tree_graph_2d_collapse");

    for depth in [6, 7, 8, 9, 10].iter() {
        group.throughput(Throughput::Bytes(*depth as u64));

        let mut config = Iso2DConfig {
            min: (-1.0, -1.0).into(),
            max: (1.0, 1.0).into(),
            intrvl_depth: *depth as u32,
            simd: false,
            program: iso2::Program::Dense2,
            ..Default::default()
        };

        let tg = iso2::bench::build_tree_graph(config);

        group.bench_with_input(BenchmarkId::new("no simd", depth), depth, |b, &_| {
            b.iter(|| iso2::bench::collapse_tree(tg.clone(), black_box(config)));
        });
        config.simd = true;
        group.bench_with_input(BenchmarkId::new("simd", depth), depth, |b, &_| {
            b.iter(|| iso2::bench::collapse_tree(tg.clone(), black_box(config)));
        });
    }

    group.finish();
}

fn extract_iso_line(c: &mut Criterion) {
    // c.bench_function("tree_graph_2d_build_no_simd", |b| {
    //     b.iter(|| iso2::TreeGraph::build(black_box(config)))
    // });
    let mut group = c.benchmark_group("extract_iso_line");

    for depth in [6, 7, 8, 9, 10].iter() {
        group.throughput(Throughput::Bytes(*depth as u64));

        let mut config = Iso2DConfig {
            min: (-1.0, -1.0).into(),
            max: (1.0, 1.0).into(),
            intrvl_depth: *depth as u32,
            simd: true,
            program: iso2::Program::Dense2,
            ..Default::default()
        };

        group.bench_with_input(BenchmarkId::new("iso_2", depth), depth, |b, &_| {
            b.iter(|| iso2::bench::extract_iso_line(black_box(config)));
        });
        config.simd = true;
        group.bench_with_input(BenchmarkId::new("iso_1", depth), depth, |b, &_| {
            b.iter(|| iso::bench::extract_iso_line(black_box(config)));
        });
        group.bench_with_input(BenchmarkId::new("iso_3", depth), depth, |b, &_| {
            b.iter(|| iso3::bench::extract_iso_line(black_box(config)));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    tree_graph_2d_build,
    // tree_graph_2d_collapse,
);
criterion_main!(benches);
