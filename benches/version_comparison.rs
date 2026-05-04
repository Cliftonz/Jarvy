//! Benchmarks for version comparison operations
//!
//! Measures the performance of parsing and comparing semantic versions.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use semver::{Version, VersionReq};
use std::hint::black_box;

fn bench_version_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_parsing");

    let versions = [
        ("simple", "1.0.0"),
        ("with_patch", "1.2.3"),
        ("with_pre", "1.0.0-alpha.1"),
        ("with_build", "1.0.0+build.123"),
        ("complex", "1.2.3-beta.4+build.567"),
    ];

    for (name, version) in versions {
        group.bench_with_input(BenchmarkId::new("parse", name), version, |b, version| {
            b.iter(|| Version::parse(black_box(version)));
        });
    }

    group.finish();
}

fn bench_version_req_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_req_parsing");

    let requirements = [
        ("exact", "=1.0.0"),
        ("caret", "^1.2.3"),
        ("tilde", "~1.2.3"),
        ("range", ">=1.0.0, <2.0.0"),
        ("wildcard", "1.2.*"),
        ("complex", ">=1.0.0, <2.0.0, !=1.5.0"),
    ];

    for (name, req) in requirements {
        group.bench_with_input(BenchmarkId::new("parse", name), req, |b, req| {
            b.iter(|| VersionReq::parse(black_box(req)));
        });
    }

    group.finish();
}

fn bench_version_comparison(c: &mut Criterion) {
    let v1 = Version::parse("1.2.3").unwrap();
    let v2 = Version::parse("1.2.4").unwrap();
    let v3 = Version::parse("2.0.0").unwrap();

    let mut group = c.benchmark_group("version_comparison");

    group.bench_function("equal_versions", |b| {
        let v1_clone = v1.clone();
        b.iter(|| black_box(&v1) == black_box(&v1_clone));
    });

    group.bench_function("less_than", |b| {
        b.iter(|| black_box(&v1) < black_box(&v2));
    });

    group.bench_function("greater_than", |b| {
        b.iter(|| black_box(&v3) > black_box(&v1));
    });

    group.bench_function("cmp_ordering", |b| {
        b.iter(|| black_box(&v1).cmp(black_box(&v2)));
    });

    group.finish();
}

fn bench_version_matching(c: &mut Criterion) {
    let versions: Vec<Version> = vec![
        Version::parse("1.0.0").unwrap(),
        Version::parse("1.2.3").unwrap(),
        Version::parse("1.5.0").unwrap(),
        Version::parse("2.0.0").unwrap(),
        Version::parse("2.1.0").unwrap(),
    ];

    let req_caret = VersionReq::parse("^1.0.0").unwrap();
    let req_range = VersionReq::parse(">=1.0.0, <2.0.0").unwrap();

    let mut group = c.benchmark_group("version_matching");

    group.bench_function("caret_match_single", |b| {
        let version = &versions[1];
        b.iter(|| req_caret.matches(black_box(version)));
    });

    group.bench_function("range_match_single", |b| {
        let version = &versions[1];
        b.iter(|| req_range.matches(black_box(version)));
    });

    group.bench_function("filter_matching_versions", |b| {
        b.iter(|| {
            let matching: Vec<_> = versions.iter().filter(|v| req_caret.matches(v)).collect();
            black_box(matching);
        });
    });

    group.bench_function("find_latest_matching", |b| {
        b.iter(|| {
            let latest = versions.iter().filter(|v| req_caret.matches(v)).max();
            black_box(latest);
        });
    });

    group.finish();
}

fn bench_latest_version_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("latest_detection");

    // Simulate checking if version string is "latest"
    let version_strings = ["latest", "1.2.3", "20.11.0", "LATEST", "Latest"];

    for version in version_strings {
        group.bench_with_input(
            BenchmarkId::new("is_latest", version),
            version,
            |b, version| {
                b.iter(|| black_box(version).eq_ignore_ascii_case("latest"));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_version_parsing,
    bench_version_req_parsing,
    bench_version_comparison,
    bench_version_matching,
    bench_latest_version_detection
);
criterion_main!(benches);
