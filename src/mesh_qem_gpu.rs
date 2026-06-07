//! CPU reference paths for GPU QEM kernels (Wave 5.7).

use crate::mesh_qem_math::{
    accumulate_vertex_quadrics, build_unique_edges, evaluate_edge_costs, Quadric,
    QUADRIC_FLOAT_COUNT,
};

fn positions_f32_to_f64(positions: &[f32]) -> Vec<[f64; 3]> {
    positions
        .chunks_exact(3)
        .map(|c| [c[0] as f64, c[1] as f64, c[2] as f64])
        .collect()
}

fn quadrics_to_f32(quadrics: &[[f64; 10]]) -> Vec<f32> {
    let mut out = Vec::with_capacity(quadrics.len() * QUADRIC_FLOAT_COUNT);
    for q in quadrics {
        for &v in q {
            out.push(v as f32);
        }
    }
    out
}

fn quadrics_from_f32(flat: &[f32]) -> Vec<[f64; 10]> {
    let vertex_count = flat.len() / QUADRIC_FLOAT_COUNT;
    let mut out = Vec::with_capacity(vertex_count);
    for chunk in flat.chunks_exact(QUADRIC_FLOAT_COUNT) {
        out.push(Quadric::from_f32_slice(chunk).m);
    }
    out
}

/// CPU reference for `mesh_quadrics_v1.wgsl` parity.
pub fn accumulate_quadrics_cpu_reference(positions: &[f32], indices: &[u32]) -> Vec<f32> {
    let positions64 = positions_f32_to_f64(positions);
    let quadrics = accumulate_vertex_quadrics(&positions64, indices);
    quadrics_to_f32(&quadrics)
}

/// CPU reference for `mesh_edge_costs_v1.wgsl` parity.
pub fn evaluate_edge_costs_cpu_reference(
    positions: &[f32],
    quadrics_flat: &[f32],
    edges: &[u32],
) -> Vec<f32> {
    let positions64 = positions_f32_to_f64(positions);
    let quadrics = quadrics_from_f32(quadrics_flat);
    let edge_pairs: Vec<(u32, u32)> = edges
        .chunks_exact(2)
        .map(|pair| (pair[0], pair[1]))
        .collect();
    evaluate_edge_costs(&positions64, &quadrics, &edge_pairs)
        .into_iter()
        .map(|c| c as f32)
        .collect()
}

/// Build unique undirected edges from a triangle index list (GPU upload helper).
pub fn unique_edges_from_indices(indices: &[u32]) -> Vec<u32> {
    let pairs = build_unique_edges(indices);
    let mut flat = Vec::with_capacity(pairs.len() * 2);
    for (a, b) in pairs {
        flat.push(a);
        flat.push(b);
    }
    flat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_qem::grid_mesh;

    #[test]
    fn test_accumulate_quadrics_non_zero_on_grid() {
        let mesh = grid_mesh(8);
        let quadrics = accumulate_quadrics_cpu_reference(&mesh.positions, &mesh.indices);
        assert_eq!(quadrics.len(), mesh.vertex_count() * QUADRIC_FLOAT_COUNT);
        assert!(quadrics.iter().any(|&v| v.abs() > 0.0));
    }

    #[test]
    fn test_evaluate_edge_costs_finite() {
        let mesh = grid_mesh(8);
        let quadrics = accumulate_quadrics_cpu_reference(&mesh.positions, &mesh.indices);
        let edges = unique_edges_from_indices(&mesh.indices);
        let costs = evaluate_edge_costs_cpu_reference(&mesh.positions, &quadrics, &edges);
        assert_eq!(costs.len(), edges.len() / 2);
        assert!(costs.iter().all(|c| c.is_finite()));
    }
}
