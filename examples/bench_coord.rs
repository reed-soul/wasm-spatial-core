//! Simple manual benchmark runner for coordinate transforms.
//!
//! Run: cargo run --example bench_coord (or cargo test --example bench_coord)
//!
//! This is a lightweight alternative to `cargo bench` for environments where
//! criterion's plotters backend is unavailable (no gnuplot).

use std::time::Instant;

// Re-implement the core transforms to avoid wasm-bindgen dependency in native builds.

const A: f64 = 6378245.0;
const EE: f64 = 0.006_693_421_62;
const EARTH_RADIUS: f64 = 6378137.0;
const X_PI: f64 = std::f64::consts::PI * 3000.0 / 180.0;

#[inline(always)]
fn transform_lat(x: f64, y: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let mut lat = -100.0 + 2.0 * x + 3.0 * y + 0.2 * y * y + 0.1 * x * y
        + 0.2 * x.abs().sqrt();
    lat += (20.0 * (6.0 * x * pi).sin() + 20.0 * (2.0 * x * pi).sin()) * 2.0 / 3.0;
    lat += (20.0 * (y * pi).sin() + 40.0 * (y / 3.0 * pi).sin()) * 2.0 / 3.0;
    lat += (160.0 * (y / 12.0 * pi).sin() + 320.0 * (y * pi / 30.0).sin()) * 2.0 / 3.0;
    lat
}

#[inline(always)]
fn transform_lng(x: f64, y: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let mut lng = 300.0 + x + 2.0 * y + 0.1 * x * x + 0.1 * x * y + 0.1 * x.abs().sqrt();
    lng += (20.0 * (6.0 * x * pi).sin() + 20.0 * (2.0 * x * pi).sin()) * 2.0 / 3.0;
    lng += (20.0 * (x * pi).sin() + 40.0 * (x / 3.0 * pi).sin()) * 2.0 / 3.0;
    lng += (150.0 * (x / 12.0 * pi).sin() + 300.0 * (x / 30.0 * pi).sin()) * 2.0 / 3.0;
    lng
}

#[inline(always)]
fn out_of_china(lng: f64, lat: f64) -> bool {
    !(73.66..=135.05).contains(&lng) || !(3.86..=53.55).contains(&lat)
}

#[inline(always)]
fn wgs84_to_gcj02(lng: f64, lat: f64) -> (f64, f64) {
    if out_of_china(lng, lat) { return (lng, lat); }
    let mut d_lat = transform_lat(lng - 105.0, lat - 35.0);
    let mut d_lng = transform_lng(lng - 105.0, lat - 35.0);
    let rad_lat = lat.to_radians();
    let magic = 1.0 - EE * rad_lat.sin() * rad_lat.sin();
    let sqrt_magic = magic.sqrt();
    d_lat = (d_lat * 180.0) / ((A * (1.0 - EE)) / (magic * sqrt_magic) * std::f64::consts::PI);
    d_lng = (d_lng * 180.0) / (A / sqrt_magic * rad_lat.cos() * std::f64::consts::PI);
    (lng + d_lng, lat + d_lat)
}

#[inline(always)]
fn gcj02_to_bd09(lng: f64, lat: f64) -> (f64, f64) {
    let z = (lng * lng + lat * lat).sqrt() + 0.00002 * (lat * X_PI).sin();
    let theta = lat.atan2(lng) + 0.000003 * (lng * X_PI).cos();
    (z * theta.cos() + 0.0065, z * theta.sin() + 0.006)
}

#[inline(always)]
fn wgs84_to_mercator(lng: f64, lat: f64) -> (f64, f64) {
    let x = lng.to_radians() * EARTH_RADIUS;
    let y = ((std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0).tan()).ln() * EARTH_RADIUS;
    (x, y)
}

fn generate_china_coords(n: usize) -> Vec<f64> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut coords = Vec::with_capacity(n * 2);
    for _ in 0..n {
        coords.push(rng.gen_range(73.66..135.05));
        coords.push(rng.gen_range(3.86..53.55));
    }
    coords
}

fn bench_transform(
    name: &str,
    coords: &[f64],
    f: fn(f64, f64) -> (f64, f64),
    iterations: usize,
) {
    let mut buf = coords.to_vec();
    let n = coords.len() / 2;

    // Warmup
    for chunk in buf.chunks_exact_mut(2) {
        let (x, y) = f(chunk[0], chunk[1]);
        chunk[0] = x;
        chunk[1] = y;
    }

    let total_start = Instant::now();
    for _ in 0..iterations {
        for chunk in buf.chunks_exact_mut(2) {
            let (x, y) = f(chunk[0], chunk[1]);
            chunk[0] = x;
            chunk[1] = y;
        }
    }
    let total_elapsed = total_start.elapsed();
    let per_iter = total_elapsed / iterations as u32;
    let ops_per_sec = (n as f64) / per_iter.as_secs_f64();

    println!(
        "  {:30} {:>10} pts  | {:>8.2} ms/iter  | {:>12.1} ops/s",
        name,
        n,
        per_iter.as_secs_f64() * 1000.0,
        ops_per_sec,
    );
}

fn main() {
    println!("=== wasm-spatial-core — Coordinate Transform Benchmarks (Native aarch64) ===\n");

    for &n in &[1_000usize, 10_000, 100_000, 1_000_000] {
        println!("--- {} points ({:.1} MB) ---", n, (n * 2 * 8) as f64 / 1_048_576.0);
        let coords = generate_china_coords(n);
        let iterations = if n <= 10_000 { 100 } else if n <= 100_000 { 10 } else { 3 };

        bench_transform("WGS-84 → GCJ-02", &coords, wgs84_to_gcj02, iterations);
        bench_transform("WGS-84 → BD-09", &coords, gcj02_to_bd09, iterations);
        bench_transform("WGS-84 → Mercator", &coords, wgs84_to_mercator, iterations);
        println!();
    }
}
