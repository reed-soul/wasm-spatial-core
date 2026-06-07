//! SVD-based 3D point-set alignment (Wave 2.7).
//!
//! Estimates a similarity transform (rotation + uniform scale + translation) from
//! corresponding 3D control points — typical for photogrammetry ↔ GIS registration
//! when paired with [`crate::enu_frame::EnuFrame`] survey coordinates.

use wasm_bindgen::prelude::*;

use crate::errors::SpatialError;

type Point3 = [f64; 3];
type PointSet = Vec<Point3>;

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

/// Per-point residual report for an alignment solve.
#[derive(Debug, Clone, PartialEq)]
pub struct ResidualReport {
    /// Euclidean residual per control point (same order as input).
    pub residuals: Vec<f64>,
    pub max_residual: f64,
    /// Indices of points used for the final fit (all points when RANSAC is off).
    pub inlier_indices: Vec<u32>,
    pub outlier_count: u32,
}

/// Result of an SVD alignment solve.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignmentResult {
    pub transform: SimilarityTransform,
    /// RMS residual over inliers used for the final fit.
    pub rms_error: f64,
    pub report: ResidualReport,
}

/// RANSAC options for robust alignment with outlier rejection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RansacConfig {
    /// Max residual (target units) to count a point as an inlier.
    pub inlier_threshold: f64,
    /// Iteration cap; `0` picks an adaptive default from point count.
    pub max_iterations: u32,
    /// LCG seed; `0` derives a deterministic seed from coordinates.
    pub seed: u64,
}

impl Default for RansacConfig {
    fn default() -> Self {
        Self {
            inlier_threshold: 0.05,
            max_iterations: 0,
            seed: 0,
        }
    }
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
    compute_similarity_transform_weighted(source, target, allow_scale, None)
}

/// Weighted Umeyama alignment. `weights[i]` is relative precision (≥ 0); omit with `None`.
pub fn compute_similarity_transform_weighted(
    source: &[[f64; 3]],
    target: &[[f64; 3]],
    allow_scale: bool,
    weights: Option<&[f64]>,
) -> Result<SimilarityTransform, SpatialErrorDetail> {
    let n = source.len();
    if n < 3 || n != target.len() {
        return Err(SpatialError::InvalidInput.with_detail(format!(
            "need at least 3 corresponding point pairs, got {n}"
        )));
    }
    if let Some(w) = weights {
        validate_weights(w, n)?;
    }

    let (mu_src, w_src) = centroid_weighted(source, weights);
    let (mu_tgt, w_tgt) = centroid_weighted(target, weights);
    if (w_src - w_tgt).abs() > 1e-12 {
        return Err(SpatialError::InvalidInput.with_detail("weight sum mismatch"));
    }
    let wsum = w_src;

    let src_c: Vec<[f64; 3]> = source.iter().map(|p| sub(*p, mu_src)).collect();
    let tgt_c: Vec<[f64; 3]> = target.iter().map(|p| sub(*p, mu_tgt)).collect();

    let mut sigma = [[0.0_f64; 3]; 3];
    for i in 0..n {
        let w = weight_at(weights, i);
        for r in 0..3 {
            for c in 0..3 {
                sigma[r][c] += w * tgt_c[i][r] * src_c[i][c];
            }
        }
    }
    let inv_w = 1.0 / wsum;
    for row in &mut sigma {
        for v in row {
            *v *= inv_w;
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

    let variance_src = weighted_mean_squared_norm(&src_c, weights);
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
    compute_svd_alignment_weighted_core(source, target, allow_scale, None)
}

/// Weighted alignment with per-point residual report.
pub fn compute_svd_alignment_weighted_core(
    source: &[f64],
    target: &[f64],
    allow_scale: bool,
    weights: Option<&[f64]>,
) -> Result<AlignmentResult, SpatialErrorDetail> {
    let (src_pts, tgt_pts) = parse_point_pairs(source, target)?;
    let transform =
        compute_similarity_transform_weighted(&src_pts, &tgt_pts, allow_scale, weights)?;
    let inlier_indices: Vec<u32> = (0..src_pts.len() as u32).collect();
    Ok(build_alignment_result(
        transform,
        &src_pts,
        &tgt_pts,
        &inlier_indices,
    ))
}

/// RANSAC robust alignment: sample minimal sets, refit on inliers, return residual report.
pub fn compute_svd_alignment_ransac_core(
    source: &[f64],
    target: &[f64],
    allow_scale: bool,
    config: RansacConfig,
    weights: Option<&[f64]>,
) -> Result<AlignmentResult, SpatialErrorDetail> {
    if config.inlier_threshold <= 0.0 || !config.inlier_threshold.is_finite() {
        return Err(SpatialError::InvalidInput
            .with_detail("inlier_threshold must be a positive finite number"));
    }

    let (src_pts, tgt_pts) = parse_point_pairs(source, target)?;
    let n = src_pts.len();
    if n < 3 {
        return Err(SpatialError::InvalidInput
            .with_detail(format!("RANSAC needs at least 3 point pairs, got {n}")));
    }
    if let Some(w) = weights {
        validate_weights(w, n)?;
    }

    let iterations = if config.max_iterations == 0 {
        adaptive_ransac_iterations(n)
    } else {
        config.max_iterations
    };
    let mut rng = config.seed;
    if rng == 0 {
        rng = hash_coords_seed(source, target);
    }

    let mut best_inliers: Vec<u32> = Vec::new();
    let mut best_count = 0usize;

    for _ in 0..iterations {
        let [i0, i1, i2] = sample_triple(n, &mut rng);
        let sample_src = [src_pts[i0], src_pts[i1], src_pts[i2]];
        let sample_tgt = [tgt_pts[i0], tgt_pts[i1], tgt_pts[i2]];
        let Ok(candidate) = compute_similarity_transform(&sample_src, &sample_tgt, allow_scale)
        else {
            continue;
        };

        let residuals = point_residuals(&candidate, &src_pts, &tgt_pts);
        let inliers: Vec<u32> = residuals
            .iter()
            .enumerate()
            .filter(|(_, r)| **r <= config.inlier_threshold)
            .map(|(i, _)| i as u32)
            .collect();

        if inliers.len() > best_count {
            best_count = inliers.len();
            best_inliers = inliers;
        }
    }

    if best_count < 3 {
        return Err(SpatialError::GeometryError.with_detail(format!(
            "RANSAC found fewer than 3 inliers at threshold {} — try a larger threshold or check control points",
            config.inlier_threshold
        )));
    }

    let (fit_src, fit_tgt, fit_weights) =
        subset_for_fit(&src_pts, &tgt_pts, weights, &best_inliers);
    let transform = compute_similarity_transform_weighted(
        &fit_src,
        &fit_tgt,
        allow_scale,
        fit_weights.as_deref(),
    )?;

    // Re-evaluate inliers with the refit transform.
    let residuals = point_residuals(&transform, &src_pts, &tgt_pts);
    let final_inliers: Vec<u32> = residuals
        .iter()
        .enumerate()
        .filter(|(_, r)| **r <= config.inlier_threshold)
        .map(|(i, _)| i as u32)
        .collect();

    if final_inliers.len() < 3 {
        return Err(SpatialError::GeometryError.with_detail(
            "RANSAC refit produced fewer than 3 inliers — control points may be inconsistent",
        ));
    }

    let (fit_src, fit_tgt, fit_weights) =
        subset_for_fit(&src_pts, &tgt_pts, weights, &final_inliers);
    let transform = compute_similarity_transform_weighted(
        &fit_src,
        &fit_tgt,
        allow_scale,
        fit_weights.as_deref(),
    )?;

    Ok(build_alignment_result(
        transform,
        &src_pts,
        &tgt_pts,
        &final_inliers,
    ))
}

fn parse_point_pairs(source: &[f64], target: &[f64]) -> Result<(PointSet, PointSet), SpatialErrorDetail> {
    let src_pts = parse_points(source)?;
    let tgt_pts = parse_points(target)?;
    if src_pts.len() != tgt_pts.len() {
        return Err(SpatialError::InvalidInput.with_detail(format!(
            "source and target must have the same point count ({} vs {})",
            src_pts.len(),
            tgt_pts.len()
        )));
    }
    Ok((src_pts, tgt_pts))
}

fn build_alignment_result(
    transform: SimilarityTransform,
    source: &[[f64; 3]],
    target: &[[f64; 3]],
    inlier_indices: &[u32],
) -> AlignmentResult {
    let residuals = point_residuals(&transform, source, target);
    let max_residual = residuals.iter().copied().fold(0.0_f64, f64::max);
    let outlier_count = (source.len() - inlier_indices.len()) as u32;

    let inlier_src: Vec<[f64; 3]> = inlier_indices.iter().map(|&i| source[i as usize]).collect();
    let inlier_tgt: Vec<[f64; 3]> = inlier_indices.iter().map(|&i| target[i as usize]).collect();
    let rms_error = transform.rms_error(&inlier_src, &inlier_tgt);

    AlignmentResult {
        transform,
        rms_error,
        report: ResidualReport {
            residuals,
            max_residual,
            inlier_indices: inlier_indices.to_vec(),
            outlier_count,
        },
    }
}

fn point_residuals(
    transform: &SimilarityTransform,
    source: &[[f64; 3]],
    target: &[[f64; 3]],
) -> Vec<f64> {
    source
        .iter()
        .zip(target.iter())
        .map(|(s, t)| {
            let out = transform.apply_point(*s);
            let dx = out[0] - t[0];
            let dy = out[1] - t[1];
            let dz = out[2] - t[2];
            (dx * dx + dy * dy + dz * dz).sqrt()
        })
        .collect()
}

fn validate_weights(weights: &[f64], n: usize) -> Result<(), SpatialErrorDetail> {
    if weights.len() != n {
        return Err(SpatialError::InvalidInput.with_detail(format!(
            "weights length ({}) must match point count ({n})",
            weights.len()
        )));
    }
    let mut positive = false;
    for (i, &w) in weights.iter().enumerate() {
        if !w.is_finite() || w < 0.0 {
            return Err(SpatialError::InvalidInput
                .with_detail(format!("weights[{i}] must be a finite non-negative number")));
        }
        if w > 0.0 {
            positive = true;
        }
    }
    if !positive {
        return Err(SpatialError::InvalidInput.with_detail("at least one weight must be positive"));
    }
    Ok(())
}

fn weight_at(weights: Option<&[f64]>, i: usize) -> f64 {
    weights.map(|w| w[i]).unwrap_or(1.0)
}

fn centroid_weighted(pts: &[[f64; 3]], weights: Option<&[f64]>) -> ([f64; 3], f64) {
    let mut wsum = 0.0;
    let mut c = [0.0; 3];
    for (i, p) in pts.iter().enumerate() {
        let w = weight_at(weights, i);
        wsum += w;
        c[0] += w * p[0];
        c[1] += w * p[1];
        c[2] += w * p[2];
    }
    if wsum > 0.0 {
        c[0] /= wsum;
        c[1] /= wsum;
        c[2] /= wsum;
    }
    (c, wsum)
}

fn weighted_mean_squared_norm(pts: &[[f64; 3]], weights: Option<&[f64]>) -> f64 {
    if pts.is_empty() {
        return 0.0;
    }
    let mut wsum = 0.0;
    let mut sum = 0.0;
    for (i, p) in pts.iter().enumerate() {
        let w = weight_at(weights, i);
        wsum += w;
        sum += w * (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]);
    }
    if wsum > 0.0 {
        sum / wsum
    } else {
        0.0
    }
}

fn subset_for_fit(
    source: &[Point3],
    target: &[Point3],
    weights: Option<&[f64]>,
    indices: &[u32],
) -> (PointSet, PointSet, Option<Vec<f64>>) {
    let src: Vec<_> = indices.iter().map(|&i| source[i as usize]).collect();
    let tgt: Vec<_> = indices.iter().map(|&i| target[i as usize]).collect();
    let w = weights.map(|weights| indices.iter().map(|&i| weights[i as usize]).collect());
    (src, tgt, w)
}

fn adaptive_ransac_iterations(point_count: usize) -> u32 {
    // Assume ~75% inliers; 99% confidence; minimal sample size 3.
    const SAMPLE: f64 = 3.0;
    const CONFIDENCE: f64 = 0.99;
    let inlier_ratio = 0.75_f64;
    let miss_prob = 1.0 - inlier_ratio.powf(SAMPLE);
    let needed = if miss_prob <= 1e-12 {
        1.0
    } else {
        (1.0 - CONFIDENCE).ln() / miss_prob.ln()
    };
    (needed.ceil() as u32).clamp(100, 5000) + (point_count as u32).min(500)
}

fn hash_coords_seed(source: &[f64], target: &[f64]) -> u64 {
    let mut h = 0xcbf29ce484222325_u64;
    for v in source.iter().chain(target.iter()) {
        h ^= v.to_bits();
        h = h.wrapping_mul(0x100000001b3);
    }
    h | 1
}

fn rand_unit(rng: &mut u64) -> f64 {
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*rng >> 33) as f64) / (1u64 << 31) as f64
}

fn rand_below(rng: &mut u64, n: usize) -> usize {
    (rand_unit(rng) * n as f64) as usize % n
}

fn sample_triple(n: usize, rng: &mut u64) -> [usize; 3] {
    let i0 = rand_below(rng, n);
    let mut i1 = rand_below(rng, n);
    while i1 == i0 {
        i1 = rand_below(rng, n);
    }
    let mut i2 = rand_below(rng, n);
    while i2 == i0 || i2 == i1 {
        i2 = rand_below(rng, n);
    }
    [i0, i1, i2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
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

    /// Per-point Euclidean residuals (all control points, same input order).
    #[wasm_bindgen(getter)]
    pub fn residuals(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(self.inner.report.residuals.as_slice())
    }

    /// Largest per-point residual.
    #[wasm_bindgen(getter, js_name = maxResidual)]
    pub fn max_residual(&self) -> f64 {
        self.inner.report.max_residual
    }

    /// Indices of inliers used for the final fit.
    #[wasm_bindgen(getter, js_name = inlierIndices)]
    pub fn inlier_indices(&self) -> js_sys::Uint32Array {
        js_sys::Uint32Array::from(self.inner.report.inlier_indices.as_slice())
    }

    /// Number of outliers (points excluded from the final fit).
    #[wasm_bindgen(getter, js_name = outlierCount)]
    pub fn outlier_count(&self) -> u32 {
        self.inner.report.outlier_count
    }

    /// Number of inliers used for the final fit.
    #[wasm_bindgen(getter, js_name = inlierCount)]
    pub fn inlier_count(&self) -> u32 {
        self.inner.report.inlier_indices.len() as u32
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

/// Weighted alignment — `weights[i]` is relative precision for pair `i`.
#[wasm_bindgen(js_name = computeSvdAlignmentWeighted)]
pub fn compute_svd_alignment_weighted(
    source: &[f64],
    target: &[f64],
    allow_scale: bool,
    weights: &[f64],
) -> Result<WasmAlignmentResult, JsValue> {
    let inner = compute_svd_alignment_weighted_core(source, target, allow_scale, Some(weights))
        .map_err(JsValue::from)?;
    Ok(WasmAlignmentResult { inner })
}

/// RANSAC robust alignment with outlier rejection and inlier refit.
///
/// `max_iterations = 0` uses an adaptive default. `seed = 0` derives a
/// deterministic seed from coordinates (reproducible for the same inputs).
#[wasm_bindgen(js_name = computeSvdAlignmentRansac)]
pub fn compute_svd_alignment_ransac(
    source: &[f64],
    target: &[f64],
    allow_scale: bool,
    inlier_threshold: f64,
    max_iterations: u32,
    seed: u64,
) -> Result<WasmAlignmentResult, JsValue> {
    let config = RansacConfig {
        inlier_threshold,
        max_iterations,
        seed,
    };
    let inner = compute_svd_alignment_ransac_core(source, target, allow_scale, config, None)
        .map_err(JsValue::from)?;
    Ok(WasmAlignmentResult { inner })
}

/// RANSAC alignment with optional per-point weights applied during inlier refit.
#[wasm_bindgen(js_name = computeSvdAlignmentRansacWeighted)]
pub fn compute_svd_alignment_ransac_weighted(
    source: &[f64],
    target: &[f64],
    allow_scale: bool,
    inlier_threshold: f64,
    max_iterations: u32,
    seed: u64,
    weights: &[f64],
) -> Result<WasmAlignmentResult, JsValue> {
    let config = RansacConfig {
        inlier_threshold,
        max_iterations,
        seed,
    };
    let inner =
        compute_svd_alignment_ransac_core(source, target, allow_scale, config, Some(weights))
            .map_err(JsValue::from)?;
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

    #[test]
    fn test_residual_report_on_clean_data() {
        let src = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let r = rot_z(0.2);
        let tgt: Vec<_> = src
            .iter()
            .map(|p| apply_gt(&r, 1.0, [1.0, 2.0, 3.0], *p))
            .collect();
        let flat_src: Vec<f64> = src.iter().flat_map(|p| p.iter().copied()).collect();
        let flat_tgt: Vec<f64> = tgt.iter().flat_map(|p| p.iter().copied()).collect();

        let result = compute_svd_alignment_core(&flat_src, &flat_tgt, false).unwrap();
        assert_eq!(result.report.inlier_indices.len(), 4);
        assert_eq!(result.report.outlier_count, 0);
        assert_eq!(result.report.residuals.len(), 4);
        assert!(result.report.max_residual < 1e-8);
        assert!(result.rms_error < 1e-8);
    }

    #[test]
    fn test_weighted_alignment_downweights_outlier() {
        let src = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let tgt_good = [
            [5.0, 2.0, 1.0],
            [6.0, 2.0, 1.0],
            [5.0, 3.0, 1.0],
            [6.0, 3.0, 1.0],
        ];
        let mut tgt_bad = tgt_good;
        tgt_bad[3] = [100.0, 100.0, 100.0];

        // Low weight on the outlier — weighted fit should follow the first three points.
        let weights = [1.0, 1.0, 1.0, 1e-6];
        let unweighted =
            compute_similarity_transform_weighted(&src, &tgt_bad, false, None).unwrap();
        let weighted =
            compute_similarity_transform_weighted(&src, &tgt_bad, false, Some(&weights)).unwrap();

        let inlier_src = &src[..3];
        let inlier_tgt = &tgt_good[..3];
        assert!(
            weighted.rms_error(inlier_src, inlier_tgt)
                < unweighted.rms_error(inlier_src, inlier_tgt)
        );
        assert!(weighted.rms_error(inlier_src, inlier_tgt) < 0.01);
    }

    #[test]
    fn test_ransac_rejects_outliers() {
        let src = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
        ];
        let r = rot_z(0.15);
        let t = [3.0, -2.0, 5.0];
        let mut tgt: Vec<_> = src.iter().map(|p| apply_gt(&r, 1.0, t, *p)).collect();
        // Inject two gross outliers.
        tgt[2] = [999.0, -999.0, 999.0];
        tgt[6] = [-500.0, 500.0, -500.0];

        let flat_src: Vec<f64> = src.iter().flat_map(|p| p.iter().copied()).collect();
        let flat_tgt: Vec<f64> = tgt.iter().flat_map(|p| p.iter().copied()).collect();

        let naive = compute_svd_alignment_core(&flat_src, &flat_tgt, false).unwrap();
        assert!(naive.rms_error > 1.0);

        let config = RansacConfig {
            inlier_threshold: 0.01,
            max_iterations: 500,
            seed: 42,
        };
        let robust =
            compute_svd_alignment_ransac_core(&flat_src, &flat_tgt, false, config, None).unwrap();
        assert_eq!(robust.report.outlier_count, 2);
        assert!(robust.rms_error < 1e-6);
        let max_inlier_residual = robust
            .report
            .inlier_indices
            .iter()
            .map(|&i| robust.report.residuals[i as usize])
            .fold(0.0_f64, f64::max);
        assert!(max_inlier_residual < 0.01);
        assert!(robust.report.max_residual > 100.0); // outliers remain in the report
        assert!(!robust.report.inlier_indices.contains(&2));
        assert!(!robust.report.inlier_indices.contains(&6));
    }
}
