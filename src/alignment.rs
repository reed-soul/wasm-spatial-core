//! SVD-based 3D rigid / similarity alignment (Wave 2.7).
//!
//! Estimates rotation, translation, and optional uniform scale from ≥3 point
//! correspondences (Kabsch / Umeyama). Typical use: register photogrammetry
//! local coordinates to survey / ENU / GIS control points.

use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};

type Vec3 = [f64; 3];
type Mat3 = [[f64; 3]; 3];

/// Rigid or similarity transform fitted from point correspondences.
#[derive(Debug, Clone)]
pub struct RigidAlignment {
    /// Column-major 3×3 rotation (scale applied separately).
    pub rotation: [f64; 9],
    pub translation: [f64; 3],
    /// Uniform scale (1.0 for pure rigid).
    pub scale: f64,
    /// RMS residual after fit.
    pub rmse: f64,
}

impl RigidAlignment {
    /// Column-major 4×4 transform suitable for rendering pipelines.
    pub fn to_mat4(&self) -> [f64; 16] {
        let r = &self.rotation;
        let s = self.scale;
        let t = &self.translation;
        [
            r[0] * s,
            r[1] * s,
            r[2] * s,
            0.0,
            r[3] * s,
            r[4] * s,
            r[5] * s,
            0.0,
            r[6] * s,
            r[7] * s,
            r[8] * s,
            0.0,
            t[0],
            t[1],
            t[2],
            1.0,
        ]
    }

    fn apply_f64(&self, p: Vec3) -> Vec3 {
        let r = &self.rotation;
        let s = self.scale;
        let t = &self.translation;
        [
            s * (r[0] * p[0] + r[3] * p[1] + r[6] * p[2]) + t[0],
            s * (r[1] * p[0] + r[4] * p[1] + r[7] * p[2]) + t[1],
            s * (r[2] * p[0] + r[5] * p[1] + r[8] * p[2]) + t[2],
        ]
    }
}

fn parse_points_xyz(flat: &[f64]) -> Result<Vec<Vec3>, SpatialErrorDetail> {
    if flat.len() < 9 || !flat.len().is_multiple_of(3) {
        return Err(SpatialError::InvalidInput
            .with_detail("point buffer must hold at least 3 points (9 floats) as [x,y,z,...]"));
    }
    Ok(flat.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

fn centroid(points: &[Vec3]) -> Vec3 {
    let n = points.len() as f64;
    let mut c = [0.0; 3];
    for p in points {
        c[0] += p[0];
        c[1] += p[1];
        c[2] += p[2];
    }
    [c[0] / n, c[1] / n, c[2] / n]
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn mat3_to_nalgebra(h: &Mat3) -> nalgebra::Matrix3<f64> {
    nalgebra::Matrix3::new(
        h[0][0], h[0][1], h[0][2], h[1][0], h[1][1], h[1][2], h[2][0], h[2][1], h[2][2],
    )
}

fn nalgebra_to_mat3(m: &nalgebra::Matrix3<f64>) -> Mat3 {
    [
        [m[(0, 0)], m[(0, 1)], m[(0, 2)]],
        [m[(1, 0)], m[(1, 1)], m[(1, 2)]],
        [m[(2, 0)], m[(2, 1)], m[(2, 2)]],
    ]
}

/// Kabsch rotation from cross-covariance matrix H via 3×3 SVD.
fn rotation_from_covariance(h: &Mat3) -> Mat3 {
    let svd = mat3_to_nalgebra(h).svd(true, true);
    let mut u = svd.u.unwrap();
    let v_t = svd.v_t.unwrap();

    if u.determinant() * v_t.determinant() < 0.0 {
        u.set_column(2, &-u.column(2));
    }

    nalgebra_to_mat3(&(u * v_t))
}

/// Compute optimal rigid or similarity transform mapping `source` → `target`.
///
/// Both slices must have equal length ≥ 3. Points are `[x,y,z]` tuples in the
/// same units (meters recommended for GIS / ENU registration).
pub fn compute_rigid_alignment(
    source: &[Vec3],
    target: &[Vec3],
    allow_scale: bool,
) -> Result<RigidAlignment, SpatialErrorDetail> {
    if source.len() != target.len() {
        return Err(SpatialError::InvalidInput
            .with_detail("source and target must have the same number of points"));
    }
    if source.len() < 3 {
        return Err(
            SpatialError::InvalidInput.with_detail("at least 3 point correspondences are required")
        );
    }

    let cs = centroid(source);
    let ct = centroid(target);

    let mut h = [[0.0; 3]; 3];
    let mut var_src = 0.0;
    for (s, t) in source.iter().zip(target.iter()) {
        let ps = sub(*s, cs);
        let pt = sub(*t, ct);
        var_src += ps[0] * ps[0] + ps[1] * ps[1] + ps[2] * ps[2];
        for r in 0..3 {
            for c in 0..3 {
                h[r][c] += ps[r] * pt[c];
            }
        }
    }

    // Umeyama convention: SVD on dst^T * src (transpose of src^T * dst).
    let h_t = [
        [h[0][0], h[1][0], h[2][0]],
        [h[0][1], h[1][1], h[2][1]],
        [h[0][2], h[1][2], h[2][2]],
    ];
    let r = rotation_from_covariance(&h_t);

    let scale = if allow_scale && var_src > 1e-24 {
        let svd = mat3_to_nalgebra(&h).svd(true, false);
        let singular_sum: f64 = svd.singular_values.iter().sum();
        singular_sum / var_src
    } else {
        1.0
    };

    let rotated_c = [
        r[0][0] * cs[0] + r[0][1] * cs[1] + r[0][2] * cs[2],
        r[1][0] * cs[0] + r[1][1] * cs[1] + r[1][2] * cs[2],
        r[2][0] * cs[0] + r[2][1] * cs[1] + r[2][2] * cs[2],
    ];
    let translation = [
        ct[0] - scale * rotated_c[0],
        ct[1] - scale * rotated_c[1],
        ct[2] - scale * rotated_c[2],
    ];

    let rotation_col = [
        r[0][0], r[1][0], r[2][0], r[0][1], r[1][1], r[2][1], r[0][2], r[1][2], r[2][2],
    ];

    let alignment = RigidAlignment {
        rotation: rotation_col,
        translation,
        scale,
        rmse: 0.0,
    };

    let mut sum_sq = 0.0;
    for (s, t) in source.iter().zip(target.iter()) {
        let mapped = alignment.apply_f64(*s);
        let dx = mapped[0] - t[0];
        let dy = mapped[1] - t[1];
        let dz = mapped[2] - t[2];
        sum_sq += dx * dx + dy * dy + dz * dz;
    }
    let rmse = (sum_sq / source.len() as f64).sqrt();

    Ok(RigidAlignment { rmse, ..alignment })
}

/// Apply a fitted alignment to a flat `[x,y,z,...]` position buffer.
pub fn apply_rigid_alignment(positions: &[f32], alignment: &RigidAlignment) -> Vec<f32> {
    if !positions.len().is_multiple_of(3) {
        return Vec::new();
    }
    positions
        .chunks_exact(3)
        .flat_map(|c| {
            let mapped = alignment.apply_f64([c[0] as f64, c[1] as f64, c[2] as f64]);
            [mapped[0] as f32, mapped[1] as f32, mapped[2] as f32]
        })
        .collect()
}

// ===========================================================================
// WASM API
// ===========================================================================

#[wasm_bindgen(js_name = "RigidAlignment")]
pub struct WasmRigidAlignment {
    inner: RigidAlignment,
}

#[wasm_bindgen(js_class = "RigidAlignment")]
impl WasmRigidAlignment {
    #[wasm_bindgen(getter)]
    pub fn scale(&self) -> f64 {
        self.inner.scale
    }

    #[wasm_bindgen(getter)]
    pub fn rmse(&self) -> f64 {
        self.inner.rmse
    }

    #[wasm_bindgen(js_name = "rotationMatrix")]
    pub fn rotation_matrix(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(&self.inner.rotation[..])
    }

    #[wasm_bindgen(js_name = "translation")]
    pub fn translation(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(&self.inner.translation[..])
    }

    #[wasm_bindgen(js_name = "toMat4")]
    pub fn to_mat4(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(&self.inner.to_mat4()[..])
    }
}

/// Fit rigid/similarity transform from corresponding 3D points (flat `[x,y,z,...]`).
#[wasm_bindgen(js_name = "computeRigidAlignment")]
pub fn compute_rigid_alignment_js(
    source: &js_sys::Float64Array,
    target: &js_sys::Float64Array,
    allow_scale: bool,
) -> Result<WasmRigidAlignment, JsValue> {
    if source.length() != target.length() {
        return Err(SpatialError::InvalidInput
            .with_detail("source and target must have equal length")
            .into());
    }
    let mut src = vec![0.0f64; source.length() as usize];
    let mut tgt = vec![0.0f64; target.length() as usize];
    source.copy_to(&mut src);
    target.copy_to(&mut tgt);

    let src_pts = parse_points_xyz(&src)?;
    let tgt_pts = parse_points_xyz(&tgt)?;
    compute_rigid_alignment(&src_pts, &tgt_pts, allow_scale)
        .map(|inner| WasmRigidAlignment { inner })
        .map_err(Into::into)
}

/// Apply a fitted alignment to positions (flat `[x,y,z,...]`).
#[wasm_bindgen(js_name = "applyRigidAlignment")]
pub fn apply_rigid_alignment_js(
    positions: &js_sys::Float32Array,
    alignment: &WasmRigidAlignment,
) -> js_sys::Float32Array {
    let mut buf = vec![0.0f32; positions.length() as usize];
    positions.copy_to(&mut buf);
    let out = apply_rigid_alignment(&buf, &alignment.inner);
    let arr = js_sys::Float32Array::new_with_length(out.len() as u32);
    arr.copy_from(&out);
    arr
}

/// Whether SVD 3D alignment (Wave 2.7) is available.
#[wasm_bindgen(js_name = "supportsSvdAlignment")]
pub fn supports_svd_alignment() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rot_z(theta: f64) -> Mat3 {
        let (s, c) = theta.sin_cos();
        [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
    }

    fn apply_mat3(m: &Mat3, p: Vec3) -> Vec3 {
        [
            m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2],
            m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2],
            m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2],
        ]
    }

    #[test]
    fn test_recover_known_rigid_transform() {
        let src = vec![
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, 0.0, 10.0],
        ];
        let r = rot_z(0.3);
        let t = [100.0, -50.0, 25.0];
        let tgt: Vec<Vec3> = src
            .iter()
            .map(|p| {
                let rp = apply_mat3(&r, *p);
                [rp[0] + t[0], rp[1] + t[1], rp[2] + t[2]]
            })
            .collect();

        let fit = compute_rigid_alignment(&src, &tgt, false).unwrap();
        assert!((fit.scale - 1.0).abs() < 1e-9);
        assert!(fit.rmse < 1e-9);

        let mapped = apply_rigid_alignment(
            &src.iter()
                .flat_map(|p| p.iter().map(|v| *v as f32))
                .collect::<Vec<_>>(),
            &fit,
        );
        for (i, p) in tgt.iter().enumerate() {
            let b = i * 3;
            assert!((mapped[b] as f64 - p[0]).abs() < 1e-5);
            assert!((mapped[b + 1] as f64 - p[1]).abs() < 1e-5);
            assert!((mapped[b + 2] as f64 - p[2]).abs() < 1e-5);
        }
    }

    #[test]
    fn test_similarity_with_scale() {
        let src = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let scale = 2.5;
        let tgt: Vec<Vec3> = src
            .iter()
            .map(|p| [p[0] * scale + 5.0, p[1] * scale - 3.0, p[2] * scale + 1.0])
            .collect();

        let fit = compute_rigid_alignment(&src, &tgt, true).unwrap();
        assert!((fit.scale - scale).abs() < 1e-6);
        assert!(fit.rmse < 1e-9);
    }

    #[test]
    fn test_too_few_points_errors() {
        let src = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let tgt = src.clone();
        assert!(compute_rigid_alignment(&src, &tgt, false).is_err());
    }
}
