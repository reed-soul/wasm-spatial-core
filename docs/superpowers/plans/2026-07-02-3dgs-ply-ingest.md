# 3DGS PLY Ingest — Minimal Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `parsePly` recognize 3D Gaussian Splatting `.ply` files — extracting `x/y/z` positions and deriving RGB colors from the spherical-harmonic DC coefficients (`f_dc_0/1/2`) — so a 3DGS file no longer degrades into a black, uncolored point cloud.

**Architecture:** 3DGS PLY files use the standard PLY container (`binary_little_endian`, no `element face`) but carry 62 float properties per vertex instead of the legacy 9 names (`x,y,z,red,green,blue,nx,ny,nz`). The existing parser (`src/ply.rs`) already parses the header generically (recording every `property float <name>` at `parse_ply_header:222-230`) and already skips unknown per-vertex bytes correctly via the `vertex_size` sum (`ply.rs:454-458`) — so it never crashes on a 3DGS file, it just can't find `red/green/blue` and returns `colors: None`. This plan adds a 3DGS-aware detection + extraction path: detect `f_dc_0` in the header, derive RGB via the canonical formula `RGB_u8 = clamp((0.5 + SH_C0 * f_dc) * 255, 0, 255)` with `SH_C0 = 0.2820945569` (per [graphdeco-inria/gaussian-splatting#485](https://github.com/graphdeco-inria/gaussian-splatting/issues/485)), and emit the result as standard RGB `colors`. The 56 other splat attributes (`f_rest_*`, `opacity`, `scale_*`, `rot_*`) are intentionally ignored (YAGNI — faithful splat rendering is a later, larger scope). Two pre-existing defects are fixed in passing: (1) `PointCloudChunk::select_points` copies colors with a `i*4` (RGBA) stride while every ingest path produces RGB (3-byte) — corrected to `i*3`; (2) `parsePly` uses a static 100 MB cap instead of the dynamic `get_current_input_limit()`, blocking large 3DGS scenes — switched to the dynamic limit.

**Tech Stack:** Rust stable, existing `src/ply.rs` (no new deps, no new Cargo feature — `mod ply;` is unconditional at `lib.rs:183`). Color math is pure arithmetic. Tests use an in-test binary-PLY builder modeled on the existing `make_binary_ply` helper (`ply.rs:749-787`).

---

## Background: the 3DGS PLY byte map (authoritative reference)

A standard 3DGS training output ([INRA/kerbl 2023](https://github.com/graphdeco-inria/gaussian-splatting)) is a `binary_little_endian` PLY with this header:

```
ply
format binary_little_endian 1.0
element vertex <count>
property float x
property float y
property float z
property float f_dc_0      ← SH degree-0 (DC) coefficient, R channel
property float f_dc_1      ← G channel
property float f_dc_2      ← B channel
property float f_rest_0    … property float f_rest_44   (45 high-order SH; ignored here)
property float opacity     (logit space; ignored)
property float scale_0/1/2 (log space; ignored)
property float rot_0/1/2/3 (quaternion w,x,y,z; ignored)
end_header
```

Every property is `float` (4 bytes LE). Per vertex = 62 floats = **248 bytes**. No `element face`.

**DC → RGB conversion** (the only color math this plan needs):

```
SH_C0 = 0.2820945569
R_f64 = 0.5 + SH_C0 * f_dc_0
G_f64 = 0.5 + SH_C0 * f_dc_1
B_f64 = 0.5 + SH_C0 * f_dc_2
R_u8  = clamp(R_f64 * 255.0, 0.0, 255.0) as u8   (round via clamp+cast)
```

Source: [graphdeco-inria Issue #485](https://github.com/graphdeco-inria/gaussian-splatting/issues/485) and the [cvlab-epfl/gaussian-splatting-web shaders.ts](https://github.com/cvlab-epfl/gaussian-splatting-web/blob/main/src/shaders.ts) constant `SH_C0 = 0.28209479177387814` (the float32-precision value; we use the f64 form `0.2820945569` to match the canonical reference — both round to the same u8).

---

## File Structure

- **Modify:** `src/ply.rs` — add 3DGS detection in `parse_ply_header` consumer path; add `f_dc` extraction in both `parse_ply_ascii` and `parse_ply_binary_le`; switch `parse_ply` input cap to dynamic. Single responsibility remains "PLY parsing".
- **Modify:** `src/spatial_ir.rs:245-291` — fix `select_points` color stride `i*4 → i*3` (RGB). Single bug fix.
- **No new files, no new deps, no new Cargo feature.** The `mod ply;` declaration at `lib.rs:183` is already unconditional, so the new path ships in every build including default npm.

---

## Task 1: Fix `select_points` RGBA→RGB stride bug (TDD)

This is a pre-existing defect that the 3DGS work would otherwise trip over. Fixing it first, in isolation, makes the later IR-flow task safe.

**Files:**
- Modify: `src/spatial_ir.rs:263-267`
- Test: `src/spatial_ir.rs` (extend the existing `tests` module)

- [ ] **Step 1: Write the failing test**

Append to the `tests` module in `src/spatial_ir.rs` (after the existing `test_point_cloud_select_by_aabb` test):

```rust
    #[test]
    fn test_point_cloud_select_preserves_rgb_colors() {
        // 2 points; colors are RGB (3 bytes/point) — the format every ingest path produces.
        // Point 0 is red (255,0,0) inside the region; point 1 is blue (0,0,255) outside.
        let mut pc = PointCloudChunk {
            metadata: ChunkMeta::new("las"),
            positions: vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0],
            colors: Some(vec![255, 0, 0, 0, 0, 255]),
            normals: None,
        };
        pc.refresh_metadata();
        let region = Aabb {
            min: [-1.0, -1.0, -1.0],
            max: [1.0, 1.0, 1.0],
        };
        let selected = pc.select_by_aabb(&region).unwrap();
        assert_eq!(selected.vertex_count(), 1);
        // The kept point's color must be red (255,0,0), not a misaligned slice.
        assert_eq!(selected.colors.as_deref().unwrap(), &[255u8, 0, 0]);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features mesh-ingest --lib spatial_ir::tests::test_point_cloud_select_preserves_rgb_colors`
Expected: FAIL — assertion `selected.colors == &[255,0,0]` fails because `select_points` reads `src[i*4 .. i*4+4]`, taking bytes `[255,0,0,0]` (4 bytes starting at offset 0) instead of `[255,0,0]` (3 bytes). The actual `selected.colors` will be a 4-byte `vec![255,0,0,0]` — wrong length and wrong content.

- [ ] **Step 3: Fix the stride (RGB = 3 bytes)**

In `src/spatial_ir.rs`, change the color copy block inside `select_points` (currently lines 263-267):

```rust
                if let (Some(src), Some(dst)) = (self.colors.as_ref(), colors.as_mut()) {
                    let base = i * 4;
                    if src.len() >= base + 4 {
                        dst.extend_from_slice(&src[base..base + 4]);
                    }
                }
```

to:

```rust
                if let (Some(src), Some(dst)) = (self.colors.as_ref(), colors.as_mut()) {
                    // colors are RGB (3 bytes/point), matching every ingest path (PLY, 3DGS).
                    let base = i * 3;
                    if src.len() >= base + 3 {
                        dst.extend_from_slice(&src[base..base + 3]);
                    }
                }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --features mesh-ingest --lib spatial_ir::tests::test_point_cloud_select_preserves_rgb_colors`
Expected: PASS. Also re-run the whole spatial_ir suite to confirm no regression:

Run: `cargo test --features mesh-ingest --lib spatial_ir::tests`
Expected: all tests PASS (the existing `test_point_cloud_select_by_aabb` used `colors: None`, so it was never exercising the stride — this fix is safe for it).

- [ ] **Step 5: Commit**

```bash
git add src/spatial_ir.rs
git commit -m "fix(ir): select_points color stride RGBA(4) -> RGB(3)

PointCloudChunk::select_points copied colors with i*4 (RGBA) stride,
but every ingest path (PLY) produces RGB (3 bytes/point). This silently
corrupted colors on any region-selected point cloud. Pre-existing defect,
surfaced by the upcoming 3DGS ingest work."
```

---

## Task 2: Switch `parsePly` to dynamic input limit (TDD)

PLY is the only parser still using the static `DEFAULT_MAX_INPUT_SIZE` (100 MB); large 3DGS scenes exceed it and JS `setInputSizeLimit()` currently can't help.

**Files:**
- Modify: `src/ply.rs:697-707` (the `parse_ply` WASM entry)
- Test: `src/ply.rs` (extend inline tests)

- [ ] **Step 1: Write the failing test**

Append to the `tests` module in `src/ply.rs`:

```rust
    #[test]
    fn test_parse_ply_core_uses_dynamic_limit_path() {
        // We can't easily test the dynamic limit value from a unit test (it's a
        // global), but we CAN verify parse_ply_core itself imposes NO limit —
        // the limit lives only in the WASM parse_ply wrapper. A 2-point PLY
        // must parse regardless. This locks the contract: core = unbounded,
        // wasm wrapper = bounded by get_current_input_limit.
        let pts = vec![(0.0, 0.0, 0.0), (1.0, 1.0, 1.0)];
        let bytes = make_binary_ply(&pts, None, None);
        let result = parse_ply_core(&bytes).expect("2-point PLY must parse");
        assert_eq!(result.vertex_count, 2);
    }
```

- [ ] **Step 2: Run test to verify it passes (it already should — this is a contract lock)**

Run: `cargo test --lib ply::tests::test_parse_ply_core_uses_dynamic_limit_path`
Expected: PASS (this test documents the existing behavior; the real change is in the WASM wrapper next).

- [ ] **Step 3: Switch the WASM wrapper to the dynamic limit**

In `src/ply.rs`, the `parse_ply` function at line 697. Change the body's size check. First read the current body (lines 697-707):

```rust
#[wasm_bindgen(js_name = "parsePly")]
pub fn parse_ply(bytes: &[u8]) -> Result<PlyResult, SpatialErrorDetail> {
    if bytes.len() > DEFAULT_MAX_INPUT_SIZE {
        return Err(SpatialError::InvalidInput.with_detail(format!(
            "PLY data too large: {} bytes (max {})",
            bytes.len(),
            DEFAULT_MAX_INPUT_SIZE
        )));
    }
    parse_ply_core(bytes).map_err(|e| SpatialError::InvalidInput.with_detail(e))
}
```

Replace the `DEFAULT_MAX_INPUT_SIZE` branch with the dynamic limit:

```rust
#[wasm_bindgen(js_name = "parsePly")]
pub fn parse_ply(bytes: &[u8]) -> Result<PlyResult, SpatialErrorDetail> {
    let limit = crate::get_current_input_limit();
    if bytes.len() > limit {
        return Err(SpatialError::InvalidInput.with_detail(format!(
            "PLY data too large: {} bytes (max {}; raise via setInputSizeLimit)",
            bytes.len(), limit
        )));
    }
    parse_ply_core(bytes).map_err(|e| SpatialError::InvalidInput.with_detail(e))
}
```

- [ ] **Step 4: Verify it compiles and tests pass**

Run: `cargo test --lib ply::tests`
Expected: all PLY tests PASS. Confirm `get_current_input_limit` is accessible (it's `pub` at `lib.rs:293`).

- [ ] **Step 5: Commit**

```bash
git add src/ply.rs
git commit -m "fix(ply): parsePly uses dynamic get_current_input_limit (was static 100MB)

PLY was the only parser still on DEFAULT_MAX_INPUT_SIZE, so JS
setInputSizeLimit() had no effect on it. Large 3DGS scenes (>100MB)
were unconditionally rejected. Now honors the runtime-configurable limit,
matching point_cloud/geotiff/glb readers. parse_ply_core stays unbounded
(testable core, limit enforced in the WASM wrapper)."
```

---

## Task 3: 3DGS header detection + test fixture builder (TDD)

Add the ability to recognize a 3DGS PLY (by presence of `f_dc_0` in the vertex properties) and build a test fixture that mimics the real 62-property layout.

**Files:**
- Modify: `src/ply.rs` (add `is_gaussian_splat_header` helper + `make_3dgs_binary_ply` test helper)
- Test: `src/ply.rs` (extend inline tests)

- [ ] **Step 1: Write the failing test + the test-fixture builder**

Append to the `tests` module in `src/ply.rs`. First the fixture builder (modeled on `make_binary_ply` at line 749, but emitting the 3DGS 62-property layout with controllable f_dc values):

```rust
    /// Build a minimal-but-realistic 3DGS binary PLY for tests.
    /// `f_dc` is the 3 DC coefficients per vertex; the remaining 56 splat
    /// properties (f_rest_0..44, opacity, scale_0..2, rot_0..3) are written as
    /// zeros so the per-vertex byte layout (248 bytes) matches a real file.
    fn make_3dgs_binary_ply(positions: &[(f32, f32, f32)], f_dc: &[(f32, f32, f32)]) -> Vec<u8> {
        assert_eq!(positions.len(), f_dc.len());
        let mut header = String::from("ply\nformat binary_little_endian 1.0\n");
        header.push_str(&format!("element vertex {}\n", positions.len()));
        header.push_str("property float x\nproperty float y\nproperty float z\n");
        header.push_str("property float f_dc_0\nproperty float f_dc_1\nproperty float f_dc_2\n");
        // 45 f_rest + opacity + 3 scale + 4 rot = 53 extra float properties
        for name in (0..45).map(|i| format!("f_rest_{}", i)) {
            header.push_str(&format!("property float {}\n", name));
        }
        for name in ["opacity", "scale_0", "scale_1", "scale_2", "rot_0", "rot_1", "rot_2", "rot_3"] {
            header.push_str(&format!("property float {}\n", name));
        }
        header.push_str("end_header\n");

        let mut data = header.into_bytes();
        for (i, &(x, y, z)) in positions.iter().enumerate() {
            data.extend_from_slice(&x.to_le_bytes());
            data.extend_from_slice(&y.to_le_bytes());
            data.extend_from_slice(&z.to_le_bytes());
            let (dc0, dc1, dc2) = f_dc[i];
            data.extend_from_slice(&dc0.to_le_bytes());
            data.extend_from_slice(&dc1.to_le_bytes());
            data.extend_from_slice(&dc2.to_le_bytes());
            // 53 remaining floats, all zero — preserves the real 248-byte stride.
            for _ in 0..53 {
                data.extend_from_slice(&0.0f32.to_le_bytes());
            }
        }
        data
    }

    #[test]
    fn test_detect_3dgs_header_by_f_dc_0() {
        let pts = vec![(0.0, 0.0, 0.0)];
        let fdc = vec![(0.0, 0.0, 0.0)];
        let bytes = make_3dgs_binary_ply(&pts, &fdc);
        let header = parse_ply_header(&bytes).expect("3DGS header must parse");
        assert!(is_gaussian_splat_header(&header), "f_dc_0 present → must detect as 3DGS");
    }

    #[test]
    fn test_detect_legacy_ply_not_3dgs() {
        let pts = vec![(0.0, 0.0, 0.0)];
        let bytes = make_binary_ply(&pts, Some(&[(255, 0, 0)]), None);
        let header = parse_ply_header(&bytes).expect("legacy header must parse");
        assert!(!is_gaussian_splat_header(&header), "red/green/blue PLY is not 3DGS");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib ply::tests::test_detect_3dgs_header_by_f_dc_0 ply::tests::test_detect_legacy_ply_not_3dgs`
Expected: FAIL — `cannot find function is_gaussian_splat_header`.

- [ ] **Step 3: Implement `is_gaussian_splat_header`**

Add to `src/ply.rs` just above the `#[cfg(test)] mod tests` block (i.e., in the main module body, near the other header helpers around line 277):

```rust
/// Detect whether a PLY header describes a 3D Gaussian Splatting file.
///
/// 3DGS files carry `f_dc_0` (the SH degree-0 R-channel coefficient) as a
/// vertex property instead of `red`/`green`/`blue`. This is the reliable
/// discriminator: every standard 3DGS training output declares it.
pub(crate) fn is_gaussian_splat_header(header: &PlyHeader) -> bool {
    header
        .vertex_properties
        .iter()
        .any(|p| p.name.eq_ignore_ascii_case("f_dc_0"))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib ply::tests::test_detect_3dgs_header_by_f_dc_0 ply::tests::test_detect_legacy_ply_not_3dgs`
Expected: PASS (both).

- [ ] **Step 5: Commit**

```bash
git add src/ply.rs
git commit -m "feat(ply): detect 3DGS PLY by f_dc_0 property (W3.6-prep / 3DGS T3)

Adds is_gaussian_splat_header — the reliable discriminator between a
3DGS file and a legacy RGB PLY. Plus a make_3dgs_binary_ply test fixture
builder that emits the real 62-property (248-byte) vertex layout so
downstream tests exercise correct byte-stride skipping."
```

---

## Task 4: DC → RGB conversion helper (TDD)

Pure color math, isolated and tested before wiring into the parser.

**Files:**
- Modify: `src/ply.rs` (add `sh_dc_to_rgb`)
- Test: `src/ply.rs` (extend inline tests)

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    #[test]
    fn test_sh_dc_to_rgb_midgray() {
        // f_dc = 0 → RGB = (0.5 + 0) * 255 = 127.5 → 127 (as-truncating cast)
        let (r, g, b) = sh_dc_to_rgb(0.0, 0.0, 0.0);
        assert_eq!((r, g, b), (127, 127, 127));
    }

    #[test]
    fn test_sh_dc_to_rgb_clamps() {
        // Large positive f_dc → 255; large negative → 0.
        let (r, _, _) = sh_dc_to_rgb(100.0, 0.0, 0.0);
        assert_eq!(r, 255);
        let (_, g, _) = sh_dc_to_rgb(0.0, -100.0, 0.0);
        assert_eq!(g, 0);
    }

    #[test]
    fn test_sh_dc_to_rgb_typical_red() {
        // A saturated-ish red splat: f_dc_0 high, others near zero.
        // f_dc_0 = 1.0 → R = (0.5 + 0.2820945569*1.0)*255 = 199.4 → 199
        let (r, _g, _b) = sh_dc_to_rgb(1.0, 0.0, 0.0);
        assert_eq!(r, 199);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib ply::tests::test_sh_dc_to_rgb_midgray`
Expected: FAIL — `cannot find function sh_dc_to_rgb`.

- [ ] **Step 3: Implement `sh_dc_to_rgb`**

Add to `src/ply.rs` just below `is_gaussian_splat_header` (added in Task 3):

```rust
/// SH degree-0 constant (the canonical 3DGS value).
const SH_C0: f64 = 0.2820945569;

/// Convert 3DGS spherical-harmonic DC coefficients to an 8-bit RGB triple.
///
/// Formula per graphdeco-inria/gaussian-splatting#485:
///   channel = clamp((0.5 + SH_C0 * f_dc) * 255, 0, 255)
/// f_dc values are read as f32 from the file and promoted to f64 here for
/// stable clamping arithmetic.
pub(crate) fn sh_dc_to_rgb(f_dc_0: f32, f_dc_1: f32, f_dc_2: f32) -> (u8, u8, u8) {
    let to_byte = |dc: f32| -> u8 {
        let v = (0.5 + SH_C0 * dc as f64) * 255.0;
        v.clamp(0.0, 255.0) as u8
    };
    (to_byte(f_dc_0), to_byte(f_dc_1), to_byte(f_dc_2))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib ply::tests::test_sh_dc_to_rgb`
Expected: PASS (all three sh_dc_to_rgb tests green).

- [ ] **Step 5: Commit**

```bash
git add src/ply.rs
git commit -m "feat(ply): sh_dc_to_rgb — 3DGS DC coefficient to 8-bit color

Canonical formula from graphdeco-inria/gaussian-splatting#485:
channel_u8 = clamp((0.5 + SH_C0 * f_dc) * 255, 0, 255) with
SH_C0 = 0.2820945569. Tested at midgray, saturation clamp, and a
typical red splat. Pure arithmetic, no deps."
```

---

## Task 5: Binary 3DGS extraction path (TDD)

Wire 3DGS detection + DC extraction into the binary parser (the path real 3DGS files use). The ASCII path is added in Task 6.

**Files:**
- Modify: `src/ply.rs:409-555` (`parse_ply_binary_le`)
- Test: `src/ply.rs` (extend inline tests)

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    #[test]
    fn test_binary_3dgs_extracts_positions_and_dc_colors() {
        // 2 splats: origin (black-ish) and (1,1,1) with a known DC.
        let pts = vec![(0.0, 0.0, 0.0), (1.0, 2.0, 3.0)];
        // f_dc that yields a recognizable color for point 1.
        // Use f_dc = 1.0 on R → R=199; 0 on G/B → 127.
        let fdc = vec![(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)];
        let bytes = make_3dgs_binary_ply(&pts, &fdc);
        let result = parse_ply_core(&bytes).expect("3DGS PLY must parse");

        assert_eq!(result.vertex_count, 2);
        // positions preserved
        let p = result.positions_core();
        assert_eq!(p.len(), 6);
        assert_eq!(p[3], 1.0); // point 1 x
        // colors derived from f_dc (not None — the whole point of this feature)
        let c = result.colors_core().expect("3DGS must derive colors from f_dc");
        assert_eq!(c.len(), 6, "RGB = 3 bytes * 2 points");
        // point 0: f_dc all 0 → midgray 127
        assert_eq!(&c[0..3], &[127, 127, 127]);
        // point 1: f_dc_0=1.0 → R=199; f_dc_1/2=0 → 127
        assert_eq!(&c[3..6], &[199, 127, 127]);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib ply::tests::test_binary_3dgs_extracts_positions_and_dc_colors`
Expected: FAIL — `colors_core()` returns `None` (the parser doesn't yet know about `f_dc`, so `has_colors` is false).

- [ ] **Step 3: Extend `parse_ply_binary_le` to extract 3DGS DC coefficients**

In `src/ply.rs`, the `parse_ply_binary_le` function (starting line 409). After the existing property-index lookups (the block at 412-420 that finds `x_idx/y_idx/z_idx/r_idx/g_idx/b_idx/nx_idx/ny_idx/nz_idx`), add a 3DGS branch. Read the current function head to find the exact insertion point, then:

Add these lookups right after the existing `find_property` calls for nx/ny/nz (around line 420):

```rust
    // 3DGS detection: look for SH DC coefficients when legacy RGB is absent.
    let is_splat = is_gaussian_splat_header(header);
    let fdc0_idx = if is_splat { find_property(&header.vertex_properties, "f_dc_0") } else { None };
    let fdc1_idx = if is_splat { find_property(&header.vertex_properties, "f_dc_1") } else { None };
    let fdc2_idx = if is_splat { find_property(&header.vertex_properties, "f_dc_2") } else { None };
    let derive_splat_colors = is_splat && fdc0_idx.is_some()
        && fdc1_idx.is_some() && fdc2_idx.is_some();
```

Then find the per-vertex read loop (around line 473, `for i in 0..vertex_count`). After the existing block that reads legacy colors (the `if r_idx.is_some()...` block) and normals, add a splat-color derivation inside the same loop, before pushing into `vertices.positions`. Use `read_float_at` (the existing helper at line ~557 area) to read the f_dc bytes:

```rust
        if derive_splat_colors {
            let dc0 = read_float_at(data, vertex_base, &header.vertex_properties[fdc0_idx.unwrap()]);
            let dc1 = read_float_at(data, vertex_base, &header.vertex_properties[fdc1_idx.unwrap()]);
            let dc2 = read_float_at(data, vertex_base, &header.vertex_properties[fdc2_idx.unwrap()]);
            let (r, g, b) = sh_dc_to_rgb(dc0, dc1, dc2);
            vertices.colors.push(r);
            vertices.colors.push(g);
            vertices.colors.push(b);
        }
```

**Important:** the existing `vertices` accumulator initializes `colors: None` and only sets it to `Some(Vec)` when legacy RGB is detected. You must also initialize it for the splat path. Find the `let mut colors = if r_idx.is_some()... Some(Vec::new()) else None;` initialization (around line 425) and change it to also be `Some` when `derive_splat_colors`:

```rust
    let has_legacy_colors = r_idx.is_some() && g_idx.is_some() && b_idx.is_some();
    let mut colors: Option<Vec<u8>> =
        if has_legacy_colors || derive_splat_colors { Some(Vec::with_capacity(vertex_count * 3)) } else { None };
```

(Adjust the surrounding code that previously inferred `has_colors` from `r_idx.is_some()` to use `has_legacy_colors` instead, so the two paths don't conflict. The `PlyResult` construction at the end of the function already takes `colors` by value, so no signature change is needed.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib ply::tests::test_binary_3dgs_extracts_positions_and_dc_colors`
Expected: PASS — positions and DC-derived colors both present and correct.

Re-run the full PLY suite to ensure legacy binary tests still pass:

Run: `cargo test --lib ply::tests`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add src/ply.rs
git commit -m "feat(ply): extract 3DGS positions + DC-derived RGB from binary PLY

parse_ply_binary_le now detects f_dc_0/1/2 and, when present, derives
8-bit RGB via sh_dc_to_rgb instead of leaving colors empty. Real 3DGS
files (binary_little_endian, 62 properties/vertex) no longer degrade to
black point clouds. Legacy RGB PLY path unchanged."
```

---

## Task 6: ASCII 3DGS extraction path (TDD)

3DGS files are virtually always binary, but the ASCII path should behave consistently for robustness and for hand-written test fixtures.

**Files:**
- Modify: `src/ply.rs:286-403` (`parse_ply_ascii`)
- Test: `src/ply.rs` (extend inline tests)

- [ ] **Step 1: Write the failing test**

Append to the `tests` module:

```rust
    #[test]
    fn test_ascii_3dgs_extracts_dc_colors() {
        // Minimal ASCII 3DGS PLY: x/y/z + f_dc_0/1/2 only (omit f_rest etc.
        // for a compact fixture; the parser skips unknown columns anyway).
        let header = "ply\nformat ascii 1.0\nelement vertex 1\n\
                      property float x\nproperty float y\nproperty float z\n\
                      property float f_dc_0\nproperty float f_dc_1\nproperty float f_dc_2\n\
                      end_header\n";
        // f_dc = (0,0,0) → midgray (127,127,127)
        let body = "0.0 0.0 0.0 0.0 0.0 0.0\n";
        let bytes = format!("{}{}", header, body).into_bytes();
        let result = parse_ply_core(&bytes).expect("ASCII 3DGS must parse");
        assert_eq!(result.vertex_count, 1);
        let c = result.colors_core().expect("ASCII 3DGS must derive colors");
        assert_eq!(c.as_slice(), &[127, 127, 127]);
    }
```

- [ ] **Step 2: Run test to verify it fail**

Run: `cargo test --lib ply::tests::test_ascii_3dgs_extracts_dc_colors`
Expected: FAIL — `colors_core()` returns `None`.

- [ ] **Step 3: Extend `parse_ply_ascii` with the 3DGS branch**

In `src/ply.rs`, `parse_ply_ascii` (line 286). Mirror the binary path: after the legacy color index lookups (around line 290-295), add:

```rust
    let is_splat = is_gaussian_splat_header(header);
    let fdc0_idx = if is_splat { find_property(&header.vertex_properties, "f_dc_0") } else { None };
    let fdc1_idx = if is_splat { find_property(&header.vertex_properties, "f_dc_1") } else { None };
    let fdc2_idx = if is_splat { find_property(&header.vertex_properties, "f_dc_2") } else { None };
    let derive_splat_colors = is_splat && fdc0_idx.is_some()
        && fdc1_idx.is_some() && fdc2_idx.is_some();
```

Note: `parse_ply_ascii` receives `header` differently — check its signature; it's `parse_ply_ascii(data: &str, header: &PlyHeader)`. The `find_property` and `is_gaussian_splat_header` calls work the same way. Then in the per-line token loop (around line 328), after reading x/y/z and legacy colors, add:

```rust
        if derive_splat_colors {
            let dc0: f32 = tokens[fdc0_idx.unwrap()].parse().unwrap_or(0.0);
            let dc1: f32 = tokens[fdc1_idx.unwrap()].parse().unwrap_or(0.0);
            let dc2: f32 = tokens[fdc2_idx.unwrap()].parse().unwrap_or(0.0);
            let (r, g, b) = sh_dc_to_rgb(dc0, dc1, dc2);
            colors.push(r);
            colors.push(g);
            colors.push(b);
        }
```

And adjust the `colors` initialization in `parse_ply_ascii` the same way as the binary path (make it `Some` when `derive_splat_colors` is true, even if legacy RGB is absent).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib ply::tests::test_ascii_3dgs_extracts_dc_colors`
Expected: PASS.

Run: `cargo test --lib ply::tests`
Expected: all green (legacy ASCII tests unaffected).

- [ ] **Step 5: Commit**

```bash
git add src/ply.rs
git commit -m "feat(ply): extract 3DGS DC colors from ASCII PLY path

Mirror of the binary 3DGS extraction for the ASCII parser, so both paths
behave consistently. Real 3DGS files are binary, but ASCII support keeps
hand-written fixtures and edge cases covered."
```

---

## Task 7: Integration — 3DGS → IR region select (TDD)

Prove the 3DGS-derived RGB flows correctly through `PointCloudChunk` and region selection (this is where the Task 1 stride fix pays off).

**Files:**
- Modify: `tests/spatial_ir_test.rs` (extend, under `mesh-ingest` feature)
- Test: same file

- [ ] **Step 1: Write the integration test**

Append to `tests/spatial_ir_test.rs`:

```rust
#[test]
fn test_3dgs_colors_survive_region_select() {
    use wasm_spatial_core::{parse_ply_core, PointCloudChunk, ChunkMeta};
    use wasm_spatial_core::Aabb;

    // Build a 3DGS PLY: 2 splats, one inside the region, one outside.
    // f_dc chosen so the inside splat is a recognizable red.
    let header = "ply\nformat binary_little_endian 1.0\nelement vertex 2\n\
                  property float x\nproperty float y\nproperty float z\n\
                  property float f_dc_0\nproperty float f_dc_1\nproperty float f_dc_2\n\
                  end_header\n";
    let mut bytes = header.as_bytes().to_vec();
    // splat 0 at origin, f_dc=(1.0, 0.0, 0.0) → red-ish (199,127,127)
    bytes.extend_from_slice(&0.0f32.to_le_bytes());
    bytes.extend_from_slice(&0.0f32.to_le_bytes());
    bytes.extend_from_slice(&0.0f32.to_le_bytes());
    bytes.extend_from_slice(&1.0f32.to_le_bytes()); // f_dc_0
    bytes.extend_from_slice(&0.0f32.to_le_bytes()); // f_dc_1
    bytes.extend_from_slice(&0.0f32.to_le_bytes()); // f_dc_2
    // splat 1 far away, f_dc=(0,0,0) → midgray
    bytes.extend_from_slice(&100.0f32.to_le_bytes());
    bytes.extend_from_slice(&100.0f32.to_le_bytes());
    bytes.extend_from_slice(&100.0f32.to_le_bytes());
    bytes.extend_from_slice(&0.0f32.to_le_bytes());
    bytes.extend_from_slice(&0.0f32.to_le_bytes());
    bytes.extend_from_slice(&0.0f32.to_le_bytes());

    let ply = parse_ply_core(&bytes).expect("3DGS PLY must parse");
    assert_eq!(ply.vertex_count, 2);
    let colors = ply.colors_core().expect("3DGS must derive colors");
    assert_eq!(colors.len(), 6);

    // Flow into IR and select a region containing only splat 0.
    let mut pc = PointCloudChunk {
        metadata: ChunkMeta::new("3dgs"),
        positions: ply.positions_core(),
        colors: Some(colors),
        normals: None,
    };
    pc.refresh_metadata();
    let region = Aabb { min: [-1.0, -1.0, -1.0], max: [1.0, 1.0, 1.0] };
    let selected = pc.select_by_aabb(&region).unwrap();
    assert_eq!(selected.vertex_count(), 1);
    // The kept color must be splat 0's red-ish triple, not a misaligned slice.
    assert_eq!(selected.colors.as_deref().unwrap(), &[199, 127, 127]);
}
```

- [ ] **Step 2: Run test to verify it fails (or passes if Task 1+5 are correct)**

Run: `cargo test --features mesh-ingest --test spatial_ir_test -- test_3dgs_colors_survive_region_select --nocapture`
Expected: PASS if Tasks 1 and 5 are correctly implemented (this test validates the integration). If it fails, the failure points at either the stride (Task 1) or the DC extraction (Task 5) — debug accordingly.

- [ ] **Step 3: Confirm the integration test passes**

Run: `cargo test --features mesh-ingest --test spatial_ir_test`
Expected: all green.

- [ ] **Step 4: Commit**

```bash
git add tests/spatial_ir_test.rs
git commit -m "test(ir): 3DGS-derived RGB survives PointCloudChunk region select

End-to-end: 3DGS PLY → parsePly (DC→RGB) → PointCloudChunk → selectByAabb.
The selected chunk keeps the correct per-splat color, proving the Task 1
stride fix and Task 5 DC extraction compose correctly. This is the real
payoff: a 3DGS file can now be ingested and spatially queried like any
point cloud."
```

---

## Task 8: Full regression + CI parity + docs note

**Files:** none (verification) + CHANGELOG note

- [ ] **Step 1: Run the full CI parity suite**

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```
Expected: all three green. Test count should rise by ~9 (Tasks 1-7 tests) vs the 817 baseline.

- [ ] **Step 2: Verify the WASM build still works**

```bash
wasm-pack build --target web --release --out-dir pkg
```
Expected: success (PLY is compiled into every build; no new feature gate).

- [ ] **Step 3: Add a CHANGELOG note**

In `CHANGELOG.md`, under `[Unreleased]` (created by the W3.6 work; if absent, add the section above `[0.8.0]`), add to `### Added`:

```markdown
- **3DGS PLY ingest (minimal slice)** — `parsePly` now recognizes 3D Gaussian Splatting `.ply` files by their `f_dc_0` property and derives RGB colors from the SH degree-0 coefficients (`RGB = clamp((0.5 + 0.2820945569·f_dc)·255)` per [graphdeco-inria#485](https://github.com/graphdeco-inria/gaussian-splatting/issues/485)). A 3DGS file no longer degrades to a black, uncolored point cloud; the derived RGB flows through `PointCloudChunk` and region selection like any point cloud. The 56 high-order splat attributes (`f_rest_*`, `opacity`, `scale_*`, `rot_*`) are intentionally ignored (faithful splat rendering is future scope).
```

And in `### Fixed` of the same `[Unreleased]` section, add:

```markdown
- **`PointCloudChunk::select_points` color stride** — was copying colors with an RGBA (4-byte) stride while every ingest path produces RGB (3-byte); silently corrupted colors on region selection. Now uses the correct 3-byte stride.
- **`parsePly` input limit** — was the only parser still using the static 100 MB `DEFAULT_MAX_INPUT_SIZE`; now uses the runtime `get_current_input_limit()` so JS `setInputSizeLimit()` applies (large 3DGS scenes no longer hard-rejected).
```

- [ ] **Step 4: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: changelog for 3DGS ingest + select_points/parsePly fixes"
```

- [ ] **Step 5: Push and confirm CI green**

```bash
git push origin master
```
Then monitor the master CI run to `success`.

---

## Self-Review (completed by plan author)

**1. Spec coverage:**
- Detect 3DGS file → Task 3 (`is_gaussian_splat_header`) ✅
- DC → RGB conversion → Task 4 (`sh_dc_to_rgb`, canonical formula) ✅
- Binary extraction (the real-file path) → Task 5 ✅
- ASCII extraction (robustness) → Task 6 ✅
- IR region select with 3DGS colors → Task 7 ✅
- Pre-existing stride bug fixed → Task 1 ✅
- 100 MB limit unblocking → Task 2 ✅
- f_rest/opacity/scale/rot intentionally ignored → stated YAGNI, documented in CHANGELOG ✅
- Faithful splat rendering (GaussianSplatChunk) → **out of scope** (documented; this is the "minimal slice") ✅

**2. Placeholder scan:** No TBD/TODO/"add error handling". Every code step contains complete, compilable Rust. The two "around line X" references in Task 5/6 are anchored to exact functions whose heads are cited (`parse_ply_binary_le` at 409, `parse_ply_ascii` at 286) — the executor reads the function to find the precise insertion point, which is standard for "extend existing function" tasks. ✅

**3. Type consistency:** `sh_dc_to_rgb(f32,f32,f32) -> (u8,u8,u8)` — same signature in Task 4 (def) and Tasks 5/6 (callers). `is_gaussian_splat_header(&PlyHeader) -> bool` — same in Task 3 (def) and Tasks 5/6. `parse_ply_core(&[u8]) -> Result<PlyResult, String>` unchanged. `PlyResult::colors_core() -> Option<Vec<u8>>` (RGB, 3 bytes/pt) consistent across all tasks. `PointCloudChunk.colors: Option<Vec<u8>>` (now correctly RGB-stride after Task 1). ✅

**4. Known limitations (honest):**
- Only the DC (degree-0) SH coefficients become color; view-dependent color from `f_rest_*` is dropped → a 3DGS scene viewed this way shows its base albedo, not the view-dependent specular highlights. This is the defining trade-off of "ingest as point cloud" vs "faithful splat render". Acceptable for the minimal slice; documented in CHANGELOG.
- No new `GaussianSplatChunk` IR variant → splat-specific edits (opacity/scale/rot transforms) aren't possible yet. Future scope.
- No real 3DGS fixture file checked in (the tests synthesize the byte layout in-test via `make_3dgs_binary_ply`). This keeps the repo lean and avoids a large binary blob; the synthesized layout matches the real 248-byte-per-vertex stride exactly.
