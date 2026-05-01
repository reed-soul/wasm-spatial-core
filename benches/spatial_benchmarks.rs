//! Criterion benchmarks for wasm-spatial-core.
//!
//! Measures native (non-WASM) performance of the core algorithms.
//! For WASM vs JS comparison, see `bench/browser/index.html`.
//!
//! Run: `cargo bench`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::Rng;

// ---------------------------------------------------------------------------
// We re-implement the core algorithms here to benchmark them without
// wasm-bindgen / js-sys dependencies (which aren't available in native).
// ---------------------------------------------------------------------------

const A: f64 = 6378245.0;
const EE: f64 = 0.006_693_421_62;
const EARTH_RADIUS: f64 = 6378137.0;
const X_PI: f64 = std::f64::consts::PI * 3000.0 / 180.0;

fn transform_lat(x: f64, y: f64) -> f64 {
    use std::f64::consts::PI;
    let mut lat =
        -100.0 + 2.0 * x + 3.0 * y + 0.2 * y * y + 0.1 * x * y + 0.2 * x.abs().sqrt();
    lat += (20.0 * (6.0 * x * PI).sin() + 20.0 * (2.0 * x * PI).sin()) * 2.0 / 3.0;
    lat += (20.0 * (y * PI).sin() + 40.0 * (y / 3.0 * PI).sin()) * 2.0 / 3.0;
    lat += (160.0 * (y / 12.0 * PI).sin() + 320.0 * (y * PI / 30.0).sin()) * 2.0 / 3.0;
    lat
}

fn transform_lng(x: f64, y: f64) -> f64 {
    use std::f64::consts::PI;
    let mut lng =
        300.0 + x + 2.0 * y + 0.1 * x * x + 0.1 * x * y + 0.1 * x.abs().sqrt();
    lng += (20.0 * (6.0 * x * PI).sin() + 20.0 * (2.0 * x * PI).sin()) * 2.0 / 3.0;
    lng += (20.0 * (x * PI).sin() + 40.0 * (x / 3.0 * PI).sin()) * 2.0 / 3.0;
    lng += (150.0 * (x / 12.0 * PI).sin() + 300.0 * (x / 30.0 * PI).sin()) * 2.0 / 3.0;
    lng
}

fn out_of_china(lng: f64, lat: f64) -> bool {
    !(73.66..=135.05).contains(&lng) || !(3.86..=53.55).contains(&lat)
}

fn wgs84_to_gcj02(lng: f64, lat: f64) -> (f64, f64) {
    if out_of_china(lng, lat) {
        return (lng, lat);
    }
    let mut d_lat = transform_lat(lng - 105.0, lat - 35.0);
    let mut d_lng = transform_lng(lng - 105.0, lat - 35.0);
    let rad_lat = lat.to_radians();
    let magic = 1.0 - EE * rad_lat.sin() * rad_lat.sin();
    let sqrt_magic = magic.sqrt();
    d_lat = (d_lat * 180.0) / ((A * (1.0 - EE)) / (magic * sqrt_magic) * std::f64::consts::PI);
    d_lng = (d_lng * 180.0) / (A / sqrt_magic * rad_lat.cos() * std::f64::consts::PI);
    (lng + d_lng, lat + d_lat)
}

fn gcj02_to_bd09(lng: f64, lat: f64) -> (f64, f64) {
    let z = (lng * lng + lat * lat).sqrt() + 0.00002 * (lat * X_PI).sin();
    let theta = lat.atan2(lng) + 0.000003 * (lng * X_PI).cos();
    (z * theta.cos() + 0.0065, z * theta.sin() + 0.006)
}

fn wgs84_to_mercator(lng: f64, lat: f64) -> (f64, f64) {
    let x = lng.to_radians() * EARTH_RADIUS;
    let y =
        ((std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0).tan()).ln() * EARTH_RADIUS;
    (x, y)
}

// ---------------------------------------------------------------------------
// Data generators
// ---------------------------------------------------------------------------

/// Generate random China-region coordinate pairs as flat `[lng, lat, …]`.
fn generate_china_coords(n_pairs: usize) -> Vec<f64> {
    let mut rng = rand::thread_rng();
    let mut coords = Vec::with_capacity(n_pairs * 2);
    for _ in 0..n_pairs {
        coords.push(rng.gen_range(73.66..135.05)); // lng
        coords.push(rng.gen_range(3.86..53.55)); // lat
    }
    coords
}

/// Generate a synthetic GeoJSON FeatureCollection with `n` Point features.
fn generate_geojson_points(n: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut features = Vec::with_capacity(n);
    for _ in 0..n {
        let lng = rng.gen_range(73.66_f64..135.05);
        let lat = rng.gen_range(3.86_f64..53.55);
        features.push(format!(
            r#"{{"type":"Feature","geometry":{{"type":"Point","coordinates":[{lng:.6},{lat:.6}]}},"properties":{{}}}}"#
        ));
    }
    format!(
        r#"{{"type":"FeatureCollection","features":[{}]}}"#,
        features.join(",")
    )
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_wgs84_to_gcj02(c: &mut Criterion) {
    let mut group = c.benchmark_group("wgs84_to_gcj02");

    for n in [1_000, 10_000, 100_000, 1_000_000] {
        let coords = generate_china_coords(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("batch", n), &coords, |b, data| {
            b.iter(|| {
                let mut buf = data.clone();
                for i in (0..buf.len()).step_by(2) {
                    let (lng, lat) = wgs84_to_gcj02(buf[i], buf[i + 1]);
                    buf[i] = black_box(lng);
                    buf[i + 1] = black_box(lat);
                }
            });
        });
    }
    group.finish();
}

fn bench_wgs84_to_mercator(c: &mut Criterion) {
    let mut group = c.benchmark_group("wgs84_to_mercator");

    for n in [1_000, 10_000, 100_000, 1_000_000] {
        let coords = generate_china_coords(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("batch", n), &coords, |b, data| {
            b.iter(|| {
                let mut buf = data.clone();
                for i in (0..buf.len()).step_by(2) {
                    let (x, y) = wgs84_to_mercator(buf[i], buf[i + 1]);
                    buf[i] = black_box(x);
                    buf[i + 1] = black_box(y);
                }
            });
        });
    }
    group.finish();
}

fn bench_gcj02_to_bd09(c: &mut Criterion) {
    let mut group = c.benchmark_group("gcj02_to_bd09");

    for n in [1_000, 10_000, 100_000, 1_000_000] {
        let coords = generate_china_coords(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("batch", n), &coords, |b, data| {
            b.iter(|| {
                let mut buf = data.clone();
                for i in (0..buf.len()).step_by(2) {
                    let (lng, lat) = gcj02_to_bd09(buf[i], buf[i + 1]);
                    buf[i] = black_box(lng);
                    buf[i + 1] = black_box(lat);
                }
            });
        });
    }
    group.finish();
}

fn bench_geojson_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("geojson_parse");
    group.sample_size(20); // Fewer samples for large inputs

    for n in [100, 1_000, 10_000, 100_000] {
        let geojson_str = generate_geojson_points(n);
        let size_mb = geojson_str.len() as f64 / 1_048_576.0;
        group.throughput(Throughput::Bytes(geojson_str.len() as u64));
        group.bench_with_input(
            BenchmarkId::new(format!("{n}_features_{size_mb:.1}MB"), n),
            &geojson_str,
            |b, data| {
                b.iter(|| {
                    let geojson: geojson::GeoJson = data.parse().unwrap();
                    let mut coords = Vec::with_capacity(n * 2);
                    if let geojson::GeoJson::FeatureCollection(fc) = geojson {
                        for feat in &fc.features {
                            if let Some(geom) = &feat.geometry {
                                if let geojson::Value::Point(pos) = &geom.value {
                                    coords.push(black_box(pos[0]));
                                    coords.push(black_box(pos[1]));
                                }
                            }
                        }
                    }
                    coords
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_wgs84_to_gcj02,
    bench_wgs84_to_mercator,
    bench_gcj02_to_bd09,
    bench_geojson_parse,
);
criterion_main!(benches);
