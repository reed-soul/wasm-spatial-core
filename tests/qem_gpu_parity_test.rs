//! CPU reference parity for GPU QEM kernels (W5.7).

#[cfg(all(feature = "mesh-edit", feature = "webgpu"))]
#[test]
fn test_qem_accumulate_quadrics_reference() {
    use wasm_spatial_core::{grid_mesh, test_exports::qem_accumulate_quadrics_reference};

    let mesh = grid_mesh(12);
    let quadrics = qem_accumulate_quadrics_reference(&mesh.positions, &mesh.indices);
    assert_eq!(
        quadrics.len(),
        mesh.vertex_count() * wasm_spatial_core::QUADRIC_FLOAT_COUNT
    );
    assert!(quadrics.iter().any(|&v| v.abs() > 0.0));
}

#[cfg(all(feature = "mesh-edit", feature = "webgpu"))]
#[test]
fn test_qem_edge_costs_reference_finite() {
    use wasm_spatial_core::{
        grid_mesh,
        test_exports::{
            qem_accumulate_quadrics_reference, qem_evaluate_edge_costs_reference,
            qem_unique_edges_from_indices,
        },
    };

    let mesh = grid_mesh(12);
    let quadrics = qem_accumulate_quadrics_reference(&mesh.positions, &mesh.indices);
    let edges = qem_unique_edges_from_indices(&mesh.indices);
    let costs = qem_evaluate_edge_costs_reference(&mesh.positions, &quadrics, &edges);
    assert_eq!(costs.len(), edges.len() / 2);
    assert!(costs.iter().all(|c| c.is_finite()));
}
