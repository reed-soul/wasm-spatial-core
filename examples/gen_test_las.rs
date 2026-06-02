//! Generate a synthetic LAS 1.2 file with N random points for testing.
//!
//! Usage: cargo run --example gen_test_las -- [num_points] [output.las]
//! Defaults: 500000 points, test-data/large/synthetic_500k.las

use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};

/// Write the fixed header fields at their standard offsets, then point data.
fn main() {
    let args: Vec<String> = env::args().collect();
    let n: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(500_000);
    let out_path = args.get(2).cloned().unwrap_or_else(|| "test-data/large/synthetic_500k.las".into());

    println!("Generating {n} points → {out_path}");

    let mut rng = XorShift64::new(42);

    // Generate random points and compute bounds
    let mut xs = Vec::with_capacity(n as usize);
    let mut ys = Vec::with_capacity(n as usize);
    let mut zs = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let x = (rng.next() as f64 / u32::MAX as f64) * 1000.0;
        let y = (rng.next() as f64 / u32::MAX as f64) * 1000.0;
        let z = (rng.next() as f64 / u32::MAX as f64) * 200.0 - 50.0;
        xs.push(x);
        ys.push(y);
        zs.push(z);
    }

    let min_x = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_x = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_y = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_z = zs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_z = zs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let scale: f64 = 0.001;
    let offset_x = min_x;
    let offset_y = min_y;
    let offset_z = min_z;

    // Build header as a 227-byte buffer, write at exact standard offsets
    let mut hdr = vec![0u8; 227];

    // 0: Magic
    hdr[0..4].copy_from_slice(b"LASF");
    // 24: Version major/minor
    hdr[24] = 1;
    hdr[25] = 2;
    // 26..58: System ID
    hdr[26..31].copy_from_slice(b"GENLS");
    // 58..90: Generating Software
    hdr[58..73].copy_from_slice(b"gen_test_las.rs");
    // 90..92: File Creation Day of Year
    hdr[90..92].copy_from_slice(&153u16.to_le_bytes());
    // 92..94: File Creation Year
    hdr[92..94].copy_from_slice(&2026u16.to_le_bytes());
    // 94..96: Header Size
    hdr[94..96].copy_from_slice(&227u16.to_le_bytes());
    // 96..100: Offset to Point Data
    hdr[96..100].copy_from_slice(&227u32.to_le_bytes());
    // 100..104: Number of VLRs = 0
    // 104: Point Data Format ID = 1 (with GPS time)
    hdr[104] = 1;
    // 105..107: Point Data Record Length = 28
    hdr[105..107].copy_from_slice(&28u16.to_le_bytes());
    // 107..111: Number of Point Records
    hdr[107..111].copy_from_slice(&n.to_le_bytes());
    // 111..131: Number of Points by Return (5 × u32)
    let by_return = [0u32, 0, n, 0, 0];
    for (i, r) in by_return.iter().enumerate() {
        let off = 111 + i * 4;
        hdr[off..off + 4].copy_from_slice(&r.to_le_bytes());
    }
    // 131..155: Scale factors (3 × f64)
    hdr[131..139].copy_from_slice(&scale.to_le_bytes());
    hdr[139..147].copy_from_slice(&scale.to_le_bytes());
    hdr[147..155].copy_from_slice(&scale.to_le_bytes());
    // 155..179: Offsets (3 × f64)
    hdr[155..163].copy_from_slice(&offset_x.to_le_bytes());
    hdr[163..171].copy_from_slice(&offset_y.to_le_bytes());
    hdr[171..179].copy_from_slice(&offset_z.to_le_bytes());
    // 179..227: Max/Min (X max, X min, Y max, Y min, Z max, Z min — each f64)
    hdr[179..187].copy_from_slice(&max_x.to_le_bytes());
    hdr[187..195].copy_from_slice(&min_x.to_le_bytes());
    hdr[195..203].copy_from_slice(&max_y.to_le_bytes());
    hdr[203..211].copy_from_slice(&min_y.to_le_bytes());
    hdr[211..219].copy_from_slice(&max_z.to_le_bytes());
    hdr[219..227].copy_from_slice(&min_z.to_le_bytes());

    assert_eq!(hdr.len(), 227);

    let mut f = BufWriter::new(File::create(&out_path).unwrap());
    f.write_all(&hdr).unwrap();

    // Point Data Records (format 1 = 28 bytes each)
    for i in 0..n as usize {
        let ix = ((xs[i] - offset_x) / scale).round() as i32;
        let iy = ((ys[i] - offset_y) / scale).round() as i32;
        let iz = ((zs[i] - offset_z) / scale).round() as i32;

        // X (4B) + Y (4B) + Z (4B)
        f.write_all(&ix.to_le_bytes()).unwrap();
        f.write_all(&iy.to_le_bytes()).unwrap();
        f.write_all(&iz.to_le_bytes()).unwrap();
        // Intensity (2B)
        let intensity = (rng.next() % 65000 + 500) as u16;
        f.write_all(&intensity.to_le_bytes()).unwrap();
        // Return Number (3-bit) | Number of Returns (3-bit) — return 1 of 1
        f.write_all(&[0x11]).unwrap();
        // Scan Angle Rank (1B)
        f.write_all(&[0u8]).unwrap();
        // Classification (1B)
        let cls = match i % 10 {
            0..=1 => 2u8,  // ground
            2..=5 => 5u8,  // high vegetation
            _ => 1u8,       // unclassified
        };
        f.write_all(&[cls]).unwrap();
        // User Data (1B) + Point Source ID (2B)
        f.write_all(&[0u8]).unwrap();
        f.write_all(&100u16.to_le_bytes()).unwrap();
        // GPS Time (8B, double)
        let gps_time = i as f64 * 0.001;
        f.write_all(&gps_time.to_le_bytes()).unwrap();
    }

    f.flush().unwrap();
    let size = File::open(&out_path).unwrap().metadata().unwrap().len();
    println!("Done: {size} bytes ({:.1} MB, {n} points)", size as f64 / 1_048_576.0);
}

struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 1 } else { seed })
    }
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }
}
