//! SVD-based 3D point-set alignment (Wave 2.7).
//!
//! Estimates a similarity transform (rotation + uniform scale + translation) from
//! corresponding 3D control points — typical for photogrammetry ↔ GIS registration
//! when paired with [`crate::enu_frame::EnuFrame`] survey coordinates.

use wasm_bindgen::prelude::*;

use crate::errors::SpatialError;

/// Similarity transform `target ≈ scale * rotation * source + translation`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimilarityTransform {
    /// Row-major 3×3 rotation matrix.
    pub rotation: [[f64; 3]; 3],
    pub translation: [f64; 3],
    pub scale: f64,
}

impl SimilarityTransform {
    /// Identity transform (scale = 1, zero translation).
    pub fn identity() -> Self {
        Self {
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: [0.0, 0.0, 0.0],
            scale: 1.0,
        }
    }

    /// Apply the transform to a single point.
    pub fn apply_point(&self, p: [f64; 3]) -> [f64; 3] {
        let r = &self.rotation;
        let s = self.scale;
        let t = self.translation;
        [
            s * (r[0][0] * p[0] + r[0][1] * p[1] + r[0][2] * p[2]) + t[0],
            s * (r[1][0] * p[0] + r[1][1] * p[1] + r[1][2] * p[2]) + t[1],
            s * (r[2][0] * p[0] + r[2][1] * p[1] + r[2][2] * p[2]) + t[2],
        ]
    }

    /// Root-mean-square residual over paired points after applying this transform.
    pub fn rms_error(&self, source: &[[f64; 3]], target: &[[f64; 3]]) -> f64 {
        debug_assert_eq!(source.len(), target.len());
        if source.is_empty() {
            return 0.0;
        }
        let mut sum_sq = 0.0;
        for (s, t) in source.iter().zip(target.iter()) {
            let out = self.apply_point(*s);
            let dx = out[0] - t[0];
            let dy = out[1] - t[1];
            let dz = out[2] - t[2];
            sum_sq += dx * dx + dy * dy + dz * dz;
        }
        (sum_sq / source.len() as f64).sqrt()
    }

    /// Column-major 4×4 matrix (WebGL / `transformPointCloud` convention).
    pub fn to_mat4_f32(&self) -> [f32; 16] {
        let m = self.to_mat4_f64();
        m.map(|v| v as f32)
    }

    /// Column-major 4×4 matrix (WebGL convention).
    pub fn to_mat4_f64(&self) -> [f64; 16] {
        let r = &self.rotation;
        let s = self.scale;
        let t = self.translation;
        [
            s * r[0][0],
            s * r[1][0],
            s * r[2][0],
            0.0,
            s * r[0][1],
            s * r[1][1],
            s * r[2][1],
            0.0,
            s * r[0][2],
            s * r[1][2],
            s * r[2][2],
            0.0,
            t[0],
            t[1],
            t[2],
            1.0,
        ]
    }
}

/// Result of an SVD alignment solve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignmentResult {
    pub transform: SimilarityTransform,
    pub rms_error: f64,
}

/// Parse flat `[x,y,z,...]` into point triples.
fn parse_points(coords: &[f64]) -> Result<Vec<[f64; 3]>, SpatialErrorDetail> {
    if coords.len() < 9 || !coords.len().is_multiple_of(3) {
        return Err(SpatialError::InvalidInput.with_detail(
            "point arrays must contain at least 3 points (9 floats) with length multiple of 3",
        ));
    }
    Ok(coords.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

type SpatialErrorDetail = crate::errors::SpatialErrorDetail;

/// Umeyama closed-form similarity alignment (SVD / Kabsch).
///
/// `source[i]` maps to `target[i]`. Requires ≥ 3 non-collinear pairs for a stable solve.
pub fn compute_similarity_transform(
    source: &[[f64; 3]],
    target: &[[f64; 3]],
    allow_scale: bool,
) -> Result<SimilarityTransform, SpatialErrorDetail> {
    let n = source.len();
    if n < 3 || n != target.len() {
        return Err(SpatialError::InvalidInput.with_detail(format!(
            "need at least 3 corresponding point pairs, got {n}"
        )));
    }

    let mu_src = centroid(source);
    let mu_tgt = centroid(target);

    let src_c: Vec<[f64; 3]> = source.iter().map(|p| sub(*p, mu_src)).collect();
    let tgt_c: Vec<[f64; 3]> = target.iter().map(|p| sub(*p, mu_tgt)).collect();

    let mut sigma = [[0.0_f64; 3]; 3];
    for i in 0..n {
        for r in 0..3 {
            for c in 0..3 {
                sigma[r][c] += tgt_c[i][r] * src_c[i][c];
            }
        }
    }
    let inv_n = 1.0 / n as f64;
    for row in &mut sigma {
        for v in row {
            *v *= inv_n;
        }
    }

    let (u, singular, vt) = svd_3x3(&sigma);

    let mut d = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    // vt = Vᵀ, so det(U Vᵀ) fixes improper rotations (Umeyama Eq. 40).
    let det = det3(&mat_mul(&u, &vt));
    if det < 0.0 {
        d[2][2] = -1.0;
    }

    let ud = mat_mul(&u, &d);
    let rotation = mat_mul(&ud, &vt);

    let variance_src = mean_squared_norm(&src_c);
    if variance_src < 1e-24 {
        return Err(SpatialError::GeometryError
            .with_detail("source control points are degenerate (zero variance)"));
    }

    let scale = if allow_scale {
        (singular[0] * d[0][0] + singular[1] * d[1][1] + singular[2] * d[2][2]) / variance_src
    } else {
        1.0
    };

    if !scale.is_finite() || scale <= 0.0 {
        return Err(SpatialError::GeometryError
            .with_detail("alignment produced invalid scale — check control point distribution"));
    }

    let r_mu = mat_vec_mul(&rotation, &mu_src);
    let translation = [
        mu_tgt[0] - scale * r_mu[0],
        mu_tgt[1] - scale * r_mu[1],
        mu_tgt[2] - scale * r_mu[2],
    ];

    Ok(SimilarityTransform {
        rotation,
        translation,
        scale,
    })
}

/// Full alignment solve from flat coordinate arrays.
pub fn compute_svd_alignment_core(
    source: &[f64],
    target: &[f64],
    allow_scale: bool,
) -> Result<AlignmentResult, SpatialErrorDetail> {
    let src_pts = parse_points(source)?;
    let tgt_pts = parse_points(target)?;
    if src_pts.len() != tgt_pts.len() {
        return Err(SpatialError::InvalidInput.with_detail(format!(
            "source and target must have the same point count ({} vs {})",
            src_pts.len(),
            tgt_pts.len()
        )));
    }
    let transform = compute_similarity_transform(&src_pts, &tgt_pts, allow_scale)?;
    let rms_error = transform.rms_error(&src_pts, &tgt_pts);
    Ok(AlignmentResult {
        transform,
        rms_error,
    })
}

fn centroid(pts: &[[f64; 3]]) -> [f64; 3] {
    let n = pts.len() as f64;
    let mut c = [0.0; 3];
    for p in pts {
        c[0] += p[0];
        c[1] += p[1];
        c[2] += p[2];
    }
    c[0] /= n;
    c[1] /= n;
    c[2] /= n;
    c
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn mean_squared_norm(pts: &[[f64; 3]]) -> f64 {
    if pts.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0;
    for p in pts {
        sum += p[0] * p[0] + p[1] * p[1] + p[2] * p[2];
    }
    sum / pts.len() as f64
}

fn transpose(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ]
}

fn mat_mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut out = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    out
}

fn mat_vec_mul(m: &[[f64; 3]; 3], v: &[f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn det3(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// SVD of a 3×3 matrix: `a ≈ u * diag(s) * vt` (vt = Vᵀ).
fn svd_3x3(a: &[[f64; 3]; 3]) -> ([[f64; 3]; 3], [f64; 3], [[f64; 3]; 3]) {
    let ata = mat_mul(&transpose(a), a);
    let (v, eigenvalues) = jacobi_eigen_symmetric_3x3(&ata);

    let mut singular = [0.0_f64; 3];
    for i in 0..3 {
        singular[i] = eigenvalues[i].max(0.0).sqrt();
    }

    let mut u = [[0.0_f64; 3]; 3];
    for i in 0..3 {
        if singular[i] > 1e-15 {
            let col = [
                a[0][0] * v[0][i] + a[0][1] * v[1][i] + a[0][2] * v[2][i],
                a[1][0] * v[0][i] + a[1][1] * v[1][i] + a[1][2] * v[2][i],
                a[2][0] * v[0][i] + a[2][1] * v[1][i] + a[2][2] * v[2][i],
            ];
            let inv = 1.0 / singular[i];
            u[0][i] = col[0] * inv;
            u[1][i] = col[1] * inv;
            u[2][i] = col[2] * inv;
        } else {
            u[i][i] = 1.0;
        }
    }

    // Re-orthogonalize U via Gram-Schmidt when singular values are tiny.
    for i in 0..3 {
        for j in 0..i {
            let dot = u[0][i] * u[0][j] + u[1][i] * u[1][j] + u[2][i] * u[2][j];
            u[0][i] -= dot * u[0][j];
            u[1][i] -= dot * u[1][j];
            u[2][i] -= dot * u[2][j];
        }
        let len = (u[0][i] * u[0][i] + u[1][i] * u[1][i] + u[2][i] * u[2][i]).sqrt();
        if len > 1e-15 {
            u[0][i] /= len;
            u[1][i] /= len;
            u[2][i] /= len;
        }
    }

    let vt = transpose(&v);
    (u, singular, vt)
}

/// Jacobi eigen-decomposition for a symmetric 3×3 matrix.
#[allow(clippy::needless_range_loop)]
fn jacobi_eigen_symmetric_3x3(a: &[[f64; 3]; 3]) -> ([[f64; 3]; 3], [f64; 3]) {
    let mut m = *a;
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    for _ in 0..50 {
        let mut max_off = 0.0_f64;
        let mut p = 0usize;
        let mut q = 1usize;
        for i in 0..3 {
            for j in (i + 1)..3 {
                let off = m[i][j].abs();
                if off > max_off {
                    max_off = off;
                    p = i;
                    q = j;
                }
            }
        }
        if max_off < 1e-15 {
            break;
        }

        let app = m[p][p];
        let aqq = m[q][q];
        let apq = m[p][q];
        let phi = 0.5 * (2.0 * apq).atan2(aqq - app);
        let c = phi.cos();
        let s = phi.sin();

        let new_app = c * c * app - 2.0 * s * c * apq + s * s * aqq;
        let new_aqq = s * s * app + 2.0 * s * c * apq + c * c * aqq;
        m[p][p] = new_app;
        m[q][q] = new_aqq;
        m[p][q] = 0.0;
        m[q][p] = 0.0;

        for r in 0..3 {
            if r == p || r == q {
                continue;
            }
            let mrp = m[r][p];
            let mrq = m[r][q];
            let new_mrp = c * mrp - s * mrq;
            let new_mrq = s * mrp + c * mrq;
            m[r][p] = new_mrp;
            m[p][r] = new_mrp;
            m[r][q] = new_mrq;
            m[q][r] = new_mrq;
        }

        for r in 0..3 {
            let vrp = v[r][p];
            let vrq = v[r][q];
            v[r][p] = c * vrp - s * vrq;
            v[r][q] = s * vrp + c * vrq;
        }
    }

    let eigenvalues = [m[0][0], m[1][1], m[2][2]];
    (v, eigenvalues)
}

// ===========================================================================
// WASM API
// ===========================================================================

/// WASM result of `computeSvdAlignment`.
#[wasm_bindgen]
pub struct WasmAlignmentResult {
    inner: AlignmentResult,
}

#[wasm_bindgen]
impl WasmAlignmentResult {
    /// Uniform scale factor.
    #[wasm_bindgen(getter)]
    pub fn scale(&self) -> f64 {
        self.inner.transform.scale
    }

    /// RMS residual in target coordinate units.
    #[wasm_bindgen(getter, js_name = rmsError)]
    pub fn rms_error(&self) -> f64 {
        self.inner.rms_error
    }

    /// Column-major 4×4 matrix for `transformPointCloud` (f32).
    #[wasm_bindgen(getter)]
    pub fn matrix(&self) -> js_sys::Float32Array {
        let m = self.inner.transform.to_mat4_f32();
        js_sys::Float32Array::from(m.as_slice())
    }

    /// Row-major 3×3 rotation (f64).
    #[wasm_bindgen(getter)]
    pub fn rotation(&self) -> js_sys::Float64Array {
        let r = &self.inner.transform.rotation;
        let flat = [
            r[0][0], r[0][1], r[0][2], r[1][0], r[1][1], r[1][2], r[2][0], r[2][1], r[2][2],
        ];
        js_sys::Float64Array::from(flat.as_slice())
    }

    /// Translation vector `[tx, ty, tz]` (f64).
    #[wasm_bindgen(getter)]
    pub fn translation(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(self.inner.transform.translation.as_slice())
    }
}

/// Estimate a similarity transform aligning `source` control points to `target`.
///
/// Both arrays are flat `[x,y,z,...]` with equal length (≥ 9 elements).
/// Set `allow_scale` false for a rigid (Kabsch) solve.
#[wasm_bindgen(js_name = computeSvdAlignment)]
pub fn compute_svd_alignment(
    source: &[f64],
    target: &[f64],
    allow_scale: bool,
) -> Result<WasmAlignmentResult, JsValue> {
    let inner = compute_svd_alignment_core(source, target, allow_scale).map_err(JsValue::from)?;
    Ok(WasmAlignmentResult { inner })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rot_z(theta: f64) -> [[f64; 3]; 3] {
        let (c, s) = (theta.cos(), theta.sin());
        [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
    }

    fn apply_gt(r: &[[f64; 3]; 3], scale: f64, t: [f64; 3], p: [f64; 3]) -> [f64; 3] {
        let rotated = [
            r[0][0] * p[0] + r[0][1] * p[1] + r[0][2] * p[2],
            r[1][0] * p[0] + r[1][1] * p[1] + r[1][2] * p[2],
            r[2][0] * p[0] + r[2][1] * p[1] + r[2][2] * p[2],
        ];
        [
            scale * rotated[0] + t[0],
            scale * rotated[1] + t[1],
            scale * rotated[2] + t[2],
        ]
    }

    #[test]
    fn test_identity_alignment() {
        let src = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let tf = compute_similarity_transform(&src, &src, true).unwrap();
        assert!((tf.scale - 1.0).abs() < 1e-10);
        assert!(tf.rms_error(&src, &src) < 1e-12);
    }

    #[test]
    fn test_recover_similarity_transform() {
        let src = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
        ];
        let r = rot_z(std::f64::consts::FRAC_PI_4);
        let scale = 2.5;
        let t = [10.0, -3.0, 5.0];
        let tgt: Vec<[f64; 3]> = src.iter().map(|p| apply_gt(&r, scale, t, *p)).collect();

        let est = compute_similarity_transform(&src, &tgt, true).unwrap();
        assert!((est.scale - scale).abs() < 1e-9);
        assert!((est.translation[0] - t[0]).abs() < 1e-9);
        assert!((est.translation[1] - t[1]).abs() < 1e-9);
        assert!((est.translation[2] - t[2]).abs() < 1e-9);
        assert!(est.rms_error(&src, &tgt) < 1e-9);
    }

    #[test]
    fn test_rigid_no_scale() {
        let src = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let r = rot_z(0.3);
        let t = [5.0, 2.0, -1.0];
        let tgt: Vec<[f64; 3]> = src.iter().map(|p| apply_gt(&r, 1.0, t, *p)).collect();

        let est = compute_similarity_transform(&src, &tgt, false).unwrap();
        assert!((est.scale - 1.0).abs() < 1e-10);
        assert!(est.rms_error(&src, &tgt) < 1e-9);
    }

    #[test]
    fn test_mat4_applies_transform() {
        let src = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let tgt = [[1.0, 2.0, 3.0], [2.0, 2.0, 3.0], [1.0, 3.0, 3.0]];
        let est = compute_similarity_transform(&src, &tgt, false).unwrap();
        let out = est.apply_point(src[0]);
        assert!((out[0] - tgt[0][0]).abs() < 1e-9);
        assert!((out[1] - tgt[0][1]).abs() < 1e-9);
        assert!((out[2] - tgt[0][2]).abs() < 1e-9);
    }

    #[test]
    fn test_rejects_mismatched_counts() {
        let src = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let tgt = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        assert!(compute_svd_alignment_core(&src, &tgt, true).is_err());
    }

    #[test]
    fn test_rejects_degenerate_source() {
        let src = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        let tgt = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert!(compute_similarity_transform(&src, &tgt, true).is_err());
    }

    #[test]
    fn test_jacobi_eigen_symmetric_3x3() {
        let a = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 10.0]];
        let ata = mat_mul(&transpose(&a), &a);
        let (v, evals) = jacobi_eigen_symmetric_3x3(&ata);
        let mut reconstructed = [[0.0; 3]; 3];
        for i in 0..3 {
            let vi = [v[0][i], v[1][i], v[2][i]];
            for r in 0..3 {
                for c in 0..3 {
                    reconstructed[r][c] += evals[i] * vi[r] * vi[c];
                }
            }
        }
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (reconstructed[i][j] - ata[i][j]).abs() < 1e-8,
                    "eigen recon[{i}][{j}]={} ata={}",
                    reconstructed[i][j],
                    ata[i][j]
                );
            }
        }
    }

    #[test]
    fn test_svd_3x3_reconstruction_asymmetric() {
        let a = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 10.0]];
        let (u, s, vt) = svd_3x3(&a);
        let recon = mat_mul(&u, &mat_mul(&diag3(&s), &vt));
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (recon[i][j] - a[i][j]).abs() < 1e-8,
                    "recon[{i}][{j}]={} expected {}",
                    recon[i][j],
                    a[i][j]
                );
            }
        }
    }

    #[test]
    fn test_svd_3x3_identity() {
        let id = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let (u, s, vt) = svd_3x3(&id);
        let recon = mat_mul(&u, &mat_mul(&diag3(&s), &vt));
        for i in 0..3 {
            for j in 0..3 {
                assert!((recon[i][j] - id[i][j]).abs() < 1e-10);
            }
        }
    }

    fn diag3(s: &[f64; 3]) -> [[f64; 3]; 3] {
        [[s[0], 0.0, 0.0], [0.0, s[1], 0.0], [0.0, 0.0, s[2]]]
    }
}
