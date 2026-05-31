//! # B3dm / I3dm Tile Encoder
//!
//! Encodes glTF/GLB models into 3D Tiles binary formats:
//!
//! - **b3dm** (Batched 3D Model) — the most general 3D Tiles format, wraps
//!   glTF/GLB meshes with feature/batch table metadata.
//! - **i3dm** (Instanced 3D Model) — efficient instanced rendering: one
//!   template model + N transform matrices.

use crate::errors::SpatialError;
use crate::gltf_writer::build_glb;
use crate::pnts::TilesetResult;
use wasm_bindgen::prelude::*;

// ===========================================================================
// B3dm encoding
// ===========================================================================

/// B3dm header (28 bytes).
struct B3dmHeader {
    magic: [u8; 4],
    version: u32,
    byte_length: u32,
    feature_table_json_byte_length: u32,
    feature_table_binary_byte_length: u32,
    batch_table_json_byte_length: u32,
    batch_table_binary_byte_length: u32,
}

/// Encode a glTF/GLB binary into a b3dm tile.
///
/// # Arguments
/// * `glb_bytes` — Complete GLB binary blob.
/// * `batch_length` — Number of batched models (default 1).
/// * `batch_table_json` — Optional batch table JSON metadata string.
///
/// # Returns
/// The complete `.b3dm` binary blob.
pub fn encode_b3dm_tile(
    glb_bytes: &[u8],
    batch_length: u32,
    batch_table_json: Option<&str>,
) -> Result<Vec<u8>, crate::errors::SpatialErrorDetail> {
    if glb_bytes.len() < 12 || &glb_bytes[0..4] != b"glTF" {
        return Err(SpatialError::InvalidInput
            .with_detail("b3dm requires a valid GLB input (must start with 'glTF' magic)"));
    }

    // Feature Table JSON: {"BATCH_LENGTH": N}
    let ft_json = format!(r#"{{"BATCH_LENGTH":{}}}"#, batch_length);
    let ft_json_padded = pad_to_4(&ft_json);

    // Feature Table Binary: empty
    let ft_binary_len = 0u32;

    // Batch Table
    let bt_json = batch_table_json.unwrap_or("{}");
    let bt_json_padded = pad_to_4(bt_json);
    let bt_binary_len = 0u32;

    let total = 28
        + ft_json_padded.len() as u32
        + ft_binary_len
        + bt_json_padded.len() as u32
        + bt_binary_len
        + glb_bytes.len() as u32;

    let header = B3dmHeader {
        magic: *b"b3dm",
        version: 1,
        byte_length: total,
        feature_table_json_byte_length: ft_json_padded.len() as u32,
        feature_table_binary_byte_length: ft_binary_len,
        batch_table_json_byte_length: bt_json_padded.len() as u32,
        batch_table_binary_byte_length: bt_binary_len,
    };

    let mut buf = Vec::with_capacity(total as usize);
    // Header
    buf.extend_from_slice(&header.magic);
    buf.extend_from_slice(&header.version.to_le_bytes());
    buf.extend_from_slice(&header.byte_length.to_le_bytes());
    buf.extend_from_slice(&header.feature_table_json_byte_length.to_le_bytes());
    buf.extend_from_slice(&header.feature_table_binary_byte_length.to_le_bytes());
    buf.extend_from_slice(&header.batch_table_json_byte_length.to_le_bytes());
    buf.extend_from_slice(&header.batch_table_binary_byte_length.to_le_bytes());
    // Feature Table JSON
    buf.extend_from_slice(ft_json_padded.as_bytes());
    // Batch Table JSON
    buf.extend_from_slice(bt_json_padded.as_bytes());
    // GLB body
    buf.extend_from_slice(glb_bytes);

    debug_assert_eq!(buf.len(), total as usize);
    Ok(buf)
}

/// WASM-exported b3dm encoder.
///
/// # Arguments
/// * `glb_bytes` — Uint8Array containing a complete GLB.
/// * `batch_length` — Number of batches (default 1).
/// * `batch_table_json` — Optional JSON string for batch table metadata.
///
/// # Returns
/// Uint8Array containing the `.b3dm` binary.
#[wasm_bindgen(js_name = "encodeB3dmTile")]
pub fn encode_b3dm_tile_js(
    glb_bytes: js_sys::Uint8Array,
    batch_length: u32,
    batch_table_json: Option<String>,
) -> Result<js_sys::Uint8Array, JsValue> {
    let mut buf = vec![0u8; glb_bytes.length() as usize];
    glb_bytes.copy_to(&mut buf);

    let result =
        encode_b3dm_tile(&buf, batch_length, batch_table_json.as_deref()).map_err(JsValue::from)?;

    let arr = js_sys::Uint8Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    Ok(arr)
}

/// Create a complete b3dm tileset from a triangle mesh.
///
/// Builds a glTF → GLB → b3dm pipeline, then generates tileset.json.
///
/// # Arguments
/// * `positions` — Flat `[x, y, z, ...]` vertex positions.
/// * `indices` — Flat `[i0, i1, i2, ...]` triangle indices.
/// * `normals` — Optional flat `[nx, ny, nz, ...]`.
/// * `colors` — Optional flat `[r, g, b, a, ...]` vertex colors (Uint8, RGBA).
/// * `center` — Tile center `[cx, cy, cz]` for content transform.
/// * `geometric_error` — Geometric error for the tile (default 500.0).
///
/// # Returns
/// A `TilesetResult` with tileset.json and one b3dm tile.
pub fn create_mesh_tileset(
    positions: &[f32],
    indices: &[u32],
    normals: Option<&[f32]>,
    colors: Option<&[u8]>,
    center: [f64; 3],
    geometric_error: f64,
) -> Result<TilesetResult, crate::errors::SpatialErrorDetail> {
    if positions.is_empty() || indices.is_empty() {
        return Err(
            SpatialError::InvalidInput.with_detail("positions and indices must not be empty")
        );
    }

    let num_vertices = (positions.len() / 3) as u32;
    let num_triangles = (indices.len() / 3) as u32;

    // Compute bounds for bounding volume
    let mut min_pos = [f32::MAX; 3];
    let mut max_pos = [f32::MIN; 3];
    for chunk in positions.chunks_exact(3) {
        for j in 0..3 {
            min_pos[j] = min_pos[j].min(chunk[j]);
            max_pos[j] = max_pos[j].max(chunk[j]);
        }
    }

    let mesh_positions: Vec<f32> = positions.to_vec();
    let mesh_indices: Vec<u32> = indices.to_vec();
    let mesh_normals: Option<Vec<f32>> = normals.map(|n| n.to_vec());
    let mesh_colors: Option<Vec<u8>> = colors.filter(|c| !c.is_empty()).map(|c| c.to_vec());

    // Build GLB using gltf_writer internals
    use crate::gltf_writer::MeshData;
    let meshes = vec![MeshData {
        positions: mesh_positions,
        indices: mesh_indices,
        normals: mesh_normals,
        colors: mesh_colors,
        material_index: None,
        mode: 4, // TRIANGLES
    }];
    let glb = build_glb(&meshes, &[]);

    // Encode b3dm
    let b3dm = encode_b3dm_tile(&glb, 1, None)?;

    // Compute tile bounding box (relative to center)
    let bounds: crate::octree::Bounds = [
        (min_pos[0] as f64) - center[0],
        (min_pos[1] as f64) - center[1],
        (min_pos[2] as f64) - center[2],
        (max_pos[0] as f64) - center[0],
        (max_pos[1] as f64) - center[1],
        (max_pos[2] as f64) - center[2],
    ];

    // Generate tileset.json
    let tileset_json =
        build_b3dm_tileset_json(center, geometric_error, num_vertices, num_triangles);

    Ok(TilesetResult {
        tileset_json,
        tiles: vec![b3dm],
        tile_bounds: vec![bounds],
        tile_uris: vec!["0.b3dm".to_string()],
    })
}

/// Create an instanced b3dm tileset: one template GLB per instance.
///
/// Each 4x4 transform is applied to produce a separate b3dm tile.
///
/// # Arguments
/// * `template_glb` — The GLB model to instance.
/// * `transforms` — Flat array of 4x4 column-major matrices (16 floats each).
/// * `center` — Tile center `[cx, cy, cz]`.
/// * `geometric_error` — Geometric error (default 500.0).
pub fn create_instanced_tileset(
    template_glb: &[u8],
    transforms: &[f32],
    center: [f64; 3],
    geometric_error: f64,
) -> Result<TilesetResult, crate::errors::SpatialErrorDetail> {
    if !transforms.len().is_multiple_of(16) {
        return Err(SpatialError::InvalidInput
            .with_detail("transforms must be a flat array of 4x4 matrices (16 floats each)"));
    }
    let num_instances = transforms.len() / 16;

    let mut tiles = Vec::with_capacity(num_instances);
    let mut tile_uris = Vec::with_capacity(num_instances);
    let mut tile_bounds = Vec::with_capacity(num_instances);

    for i in 0..num_instances {
        let t = &transforms[i * 16..(i + 1) * 16];

        // Apply transform to GLB positions and rebuild
        let transformed_glb = transform_glb(template_glb, t)?;

        let b3dm = encode_b3dm_tile(&transformed_glb, 1, None)?;
        tiles.push(b3dm);
        tile_uris.push(format!("{}.b3dm", i));

        // Compute transformed bounding box (approximate from origin transform)
        tile_bounds.push([
            t[12] as f64 - center[0],
            t[13] as f64 - center[1],
            t[14] as f64 - center[2],
            (t[12] + 1.0) as f64 - center[0],
            (t[13] + 1.0) as f64 - center[1],
            (t[14] + 1.0) as f64 - center[2],
        ]);
    }

    let tileset_json = build_instanced_tileset_json(center, geometric_error, num_instances);

    Ok(TilesetResult {
        tileset_json,
        tiles,
        tile_bounds,
        tile_uris,
    })
}

/// Build tileset.json for a single b3dm tile.
fn build_b3dm_tileset_json(
    center: [f64; 3],
    geometric_error: f64,
    num_vertices: u32,
    num_triangles: u32,
) -> String {
    format!(
        r#"{{
  "asset": {{
    "version": "1.0",
    "tilesetVersion": "1.0.0",
    "generator": "wasm-spatial-core b3dm encoder"
  }},
  "geometricError": {geometric_error},
  "root": {{
    "boundingVolume": {{
      "box": [{cx}, {cy}, {cz}, 500, 0, 0, 0, 500, 0, 0, 0, 500]
    }},
    "geometricError": {geometric_error},
    "refine": "ADD",
    "content": {{
      "uri": "0.b3dm",
      "boundingVolume": {{
        "box": [{cx}, {cy}, {cz}, 500, 0, 0, 0, 500, 0, 0, 0, 500]
      }}
    }},
    "extras": {{
      "vertices": {num_vertices},
      "triangles": {num_triangles}
    }}
  }}
}}"#,
        cx = center[0],
        cy = center[1],
        cz = center[2],
    )
}

/// Build tileset.json for instanced tiles.
fn build_instanced_tileset_json(
    center: [f64; 3],
    geometric_error: f64,
    num_instances: usize,
) -> String {
    let children: Vec<String> = (0..num_instances)
        .map(|i| {
            format!(
                r#"{{
      "boundingVolume": {{
        "box": [{cx}, {cy}, {cz}, 50, 0, 0, 0, 50, 0, 0, 0, 50]
      }},
      "geometricError": {ge},
      "content": {{
        "uri": "{i}.b3dm"
      }},
      "refine": "ADD"
    }}"#,
                cx = center[0],
                cy = center[1],
                cz = center[2],
                ge = geometric_error,
                i = i,
            )
        })
        .collect();

    format!(
        r#"{{
  "asset": {{
    "version": "1.0",
    "tilesetVersion": "1.0.0",
    "generator": "wasm-spatial-core b3dm instanced encoder"
  }},
  "geometricError": {ge},
  "root": {{
    "boundingVolume": {{
      "box": [{cx}, {cy}, {cz}, 5000, 0, 0, 0, 5000, 0, 0, 0, 5000]
    }},
    "geometricError": {ge},
    "refine": "ADD",
    "children": [
      {children}
    ],
    "extras": {{
      "instanceCount": {num_instances}
    }}
  }}
}}"#,
        cx = center[0],
        cy = center[1],
        cz = center[2],
        ge = geometric_error,
        children = children.join(",\n"),
        num_instances = num_instances,
    )
}

/// Transform GLB positions by a 4x4 matrix.
///
/// Parses the GLB JSON chunk to find position accessor offsets, then
/// modifies the BIN chunk in-place.
fn transform_glb(
    glb: &[u8],
    transform: &[f32],
) -> Result<Vec<u8>, crate::errors::SpatialErrorDetail> {
    if glb.len() < 12 || &glb[0..4] != b"glTF" {
        return Err(
            SpatialError::InvalidInput.with_detail("transform_glb requires valid GLB input")
        );
    }

    // Parse GLB chunks
    let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let json_start = 20;
    let json_data = &glb[json_start..json_start + json_len];

    let bin_len_offset = json_start + json_len;
    let bin_data_len =
        u32::from_le_bytes(glb[bin_len_offset..bin_len_offset + 4].try_into().unwrap()) as usize;
    let bin_start = bin_len_offset + 8;
    let _bin_end = bin_start + bin_data_len;

    // Parse glTF JSON to find position bufferView
    let gltf: serde_json::Value =
        serde_json::from_slice(json_data).map_err(|e| SpatialError::ParseError.with_detail(e))?;

    // Find first POSITION bufferView
    let mut bin_offset = None;
    let mut count = 0u32;

    if let Some(accessors) = gltf.get("accessors").and_then(|a| a.as_array()) {
        for acc in accessors {
            if let Some(attrs) = acc.get("type").and_then(|t| t.as_str()) {
                if attrs == "VEC3" {
                    bin_offset = acc
                        .get("byteOffset")
                        .and_then(|o| o.as_u64())
                        .map(|o| o as usize);
                    count = acc.get("count").and_then(|c| c.as_u64()).unwrap_or(0) as u32;
                    break; // Use first VEC3 accessor as positions
                }
            }
        }
    }

    let mut result = glb.to_vec();

    if let Some(offset) = bin_offset {
        // Apply 4x4 transform to positions
        for i in 0..count as usize {
            let px = f32::from_le_bytes(
                result[bin_start + offset + i * 12..bin_start + offset + i * 12 + 4]
                    .try_into()
                    .unwrap(),
            );
            let py = f32::from_le_bytes(
                result[bin_start + offset + i * 12 + 4..bin_start + offset + i * 12 + 8]
                    .try_into()
                    .unwrap(),
            );
            let pz = f32::from_le_bytes(
                result[bin_start + offset + i * 12 + 8..bin_start + offset + i * 12 + 12]
                    .try_into()
                    .unwrap(),
            );

            // Matrix multiply (column-major): result = M * [px, py, pz, 1]
            let nx = transform[0] * px + transform[4] * py + transform[8] * pz + transform[12];
            let ny = transform[1] * px + transform[5] * py + transform[9] * pz + transform[13];
            let nz = transform[2] * px + transform[6] * py + transform[10] * pz + transform[14];

            if bin_start + offset + i * 12 + 12 <= result.len() {
                result[bin_start + offset + i * 12..bin_start + offset + i * 12 + 4]
                    .copy_from_slice(&nx.to_le_bytes());
                result[bin_start + offset + i * 12 + 4..bin_start + offset + i * 12 + 8]
                    .copy_from_slice(&ny.to_le_bytes());
                result[bin_start + offset + i * 12 + 8..bin_start + offset + i * 12 + 12]
                    .copy_from_slice(&nz.to_le_bytes());
            }
        }
    }

    Ok(result)
}

// ===========================================================================
// I3dm encoding
// ===========================================================================

/// I3dm header (32 bytes).
struct I3dmHeader {
    magic: [u8; 4],
    version: u32,
    byte_length: u32,
    feature_table_json_byte_length: u32,
    feature_table_binary_byte_length: u32,
    batch_table_json_byte_length: u32,
    batch_table_binary_byte_length: u32,
    gltf_format: u32,
}

/// Instance transform data.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct InstanceTransform {
    position: [f32; 3],
    rotation: [f32; 4], // quaternion (x, y, z, w)
    scale: f32,
}

impl Default for InstanceTransform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0], // identity quaternion
            scale: 1.0,
        }
    }
}

/// Encode a glTF/GLB template with instance transforms into an i3dm tile.
///
/// # Arguments
/// * `glb_bytes` — Template GLB binary.
/// * `positions` — Flat `[x, y, z, ...]` positions for each instance.
/// * `orientations` — Optional flat `[qx, qy, qz, qw, ...]` quaternion rotations.
/// * `scales` — Optional per-instance scale factors.
///
/// # Returns
/// The complete `.i3dm` binary blob.
pub fn encode_i3dm_tile(
    glb_bytes: &[u8],
    positions: &[f32],
    orientations: Option<&[f32]>,
    scales: Option<&[f32]>,
) -> Result<Vec<u8>, crate::errors::SpatialErrorDetail> {
    if glb_bytes.len() < 12 || &glb_bytes[0..4] != b"glTF" {
        return Err(SpatialError::InvalidInput.with_detail("i3dm requires a valid GLB input"));
    }

    let n = positions.len() / 3;
    if n == 0 {
        return Err(SpatialError::InvalidInput.with_detail("i3dm requires at least one instance"));
    }

    if let Some(orient) = orientations {
        if orient.len() != n * 4 {
            return Err(SpatialError::InvalidInput.with_detail(format!(
                "orientations length mismatch: expected {} ({}×4), got {}",
                n * 4,
                n,
                orient.len()
            )));
        }
    }
    if let Some(sc) = scales {
        if sc.len() != n {
            return Err(SpatialError::InvalidInput.with_detail(format!(
                "scales length mismatch: expected {}, got {}",
                n,
                sc.len()
            )));
        }
    }

    let has_orientations = orientations.is_some();
    let has_scales = scales.is_some();

    // Feature Table Binary layout:
    // POSITION: n × 3 × float32 = 12n bytes
    // NORMAL_UP (rotation): n × 3 × float32 = 12n bytes (optional)
    // NORMAL_RIGHT (rotation): n × 3 × float32 = 12n bytes (optional)
    // SCALE: n × float32 = 4n bytes (optional)
    //
    // For i3dm spec: POSITION, NORMAL_UP, NORMAL_RIGHT are the 3 columns of the
    // rotation matrix, not quaternion. We convert quaternion → 3x3 rotation.
    // Actually per spec: POSITION(12n), NORMAL_UP(12n), NORMAL_RIGHT(12n), SCALE_NON_UNIFORM(12n)
    // or SCALE(4n).
    // We use: POSITION, NORMAL_UP, NORMAL_RIGHT (rotation as 3 columns), SCALE.

    let has_rotation = has_orientations;
    let pos_bytes = n * 3 * 4;
    let normal_up_bytes = if has_rotation { n * 3 * 4 } else { 0 };
    let normal_right_bytes = if has_rotation { n * 3 * 4 } else { 0 };
    let scale_bytes = if has_scales { n * 4 } else { 0 };

    let ft_binary_len = pos_bytes + normal_up_bytes + normal_right_bytes + scale_bytes;

    // Feature Table JSON
    let mut ft_json = format!(
        r#"{{"INSTANCES_LENGTH":{},"POSITION":{{"byteOffset":0,"componentType":5126,"type":"VEC3"}}}}"#,
        n
    );

    let mut offset = pos_bytes;
    if has_rotation {
        let up_str = format!(
            r#","NORMAL_UP":{{"byteOffset":{},"componentType":5126,"type":"VEC3"}}"#,
            offset
        );
        offset += normal_up_bytes;
        let right_str = format!(
            r#","NORMAL_RIGHT":{{"byteOffset":{},"componentType":5126,"type":"VEC3"}}"#,
            offset
        );
        offset += normal_right_bytes;
        // Insert before closing brace
        ft_json = ft_json.trim_end_matches('}').to_string() + &up_str + &right_str + "}";
    }
    if has_scales {
        let scale_str = format!(
            r#","SCALE":{{"byteOffset":{},"componentType":5126,"type":"SCALAR"}}"#,
            offset
        );
        ft_json = ft_json.trim_end_matches('}').to_string() + &scale_str + "}";
    }

    let ft_json_padded = pad_to_4(&ft_json);

    // Batch Table (empty)
    let bt_json_padded = pad_to_4("{}");

    let total = 32
        + ft_json_padded.len() as u32
        + ft_binary_len as u32
        + bt_json_padded.len() as u32
        + glb_bytes.len() as u32;

    let header = I3dmHeader {
        magic: *b"i3dm",
        version: 1,
        byte_length: total,
        feature_table_json_byte_length: ft_json_padded.len() as u32,
        feature_table_binary_byte_length: ft_binary_len as u32,
        batch_table_json_byte_length: bt_json_padded.len() as u32,
        batch_table_binary_byte_length: 0,
        gltf_format: 1, // 1 = embedded GLB
    };

    let mut buf = Vec::with_capacity(total as usize);

    // Header (32 bytes)
    buf.extend_from_slice(&header.magic);
    buf.extend_from_slice(&header.version.to_le_bytes());
    buf.extend_from_slice(&header.byte_length.to_le_bytes());
    buf.extend_from_slice(&header.feature_table_json_byte_length.to_le_bytes());
    buf.extend_from_slice(&header.feature_table_binary_byte_length.to_le_bytes());
    buf.extend_from_slice(&header.batch_table_json_byte_length.to_le_bytes());
    buf.extend_from_slice(&header.batch_table_binary_byte_length.to_le_bytes());
    buf.extend_from_slice(&header.gltf_format.to_le_bytes());

    // Feature Table JSON
    buf.extend_from_slice(ft_json_padded.as_bytes());

    // Feature Table Binary
    for i in 0..n {
        buf.extend_from_slice(&positions[i * 3].to_le_bytes());
        buf.extend_from_slice(&positions[i * 3 + 1].to_le_bytes());
        buf.extend_from_slice(&positions[i * 3 + 2].to_le_bytes());
    }

    // Rotation columns (quaternion → 3x3)
    if let Some(orient) = orientations {
        for i in 0..n {
            let qx = orient[i * 4];
            let qy = orient[i * 4 + 1];
            let qz = orient[i * 4 + 2];
            let qw = orient[i * 4 + 3];

            // Quaternion to rotation matrix (column-major for i3dm: UP, RIGHT, FORWARD)
            let (up_x, up_y, up_z, right_x, right_y, right_z) =
                quat_to_rotation_columns(qx, qy, qz, qw);

            // NORMAL_UP
            buf.extend_from_slice(&up_x.to_le_bytes());
            buf.extend_from_slice(&up_y.to_le_bytes());
            buf.extend_from_slice(&up_z.to_le_bytes());
            // NORMAL_RIGHT
            buf.extend_from_slice(&right_x.to_le_bytes());
            buf.extend_from_slice(&right_y.to_le_bytes());
            buf.extend_from_slice(&right_z.to_le_bytes());
        }
    }

    // Scale
    if let Some(sc) = scales {
        for &s in sc {
            buf.extend_from_slice(&s.to_le_bytes());
        }
    }

    // Batch Table JSON
    buf.extend_from_slice(bt_json_padded.as_bytes());

    // GLB template
    buf.extend_from_slice(glb_bytes);

    debug_assert_eq!(buf.len(), total as usize);
    Ok(buf)
}

/// WASM-exported i3dm encoder.
#[wasm_bindgen(js_name = "encodeI3dmTile")]
pub fn encode_i3dm_tile_js(
    glb_bytes: js_sys::Uint8Array,
    positions: js_sys::Float32Array,
    orientations: Option<js_sys::Float32Array>,
    scales: Option<js_sys::Float32Array>,
) -> Result<js_sys::Uint8Array, JsValue> {
    let mut glb = vec![0u8; glb_bytes.length() as usize];
    glb_bytes.copy_to(&mut glb);

    let mut pos = vec![0.0f32; positions.length() as usize];
    positions.copy_to(&mut pos);

    let mut orient_opt: Option<Vec<f32>> = None;
    if let Some(ref o) = orientations {
        if o.length() > 0 {
            let mut v = vec![0.0f32; o.length() as usize];
            o.copy_to(&mut v);
            orient_opt = Some(v);
        }
    }

    let mut scale_opt: Option<Vec<f32>> = None;
    if let Some(ref s) = scales {
        if s.length() > 0 {
            let v = vec![0.0f32; s.length() as usize];
            s.copy_from(&v);
            scale_opt = Some(v);
        }
    }

    let result = encode_i3dm_tile(&glb, &pos, orient_opt.as_deref(), scale_opt.as_deref())
        .map_err(JsValue::from)?;

    let arr = js_sys::Uint8Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    Ok(arr)
}

/// Create a complete i3dm tileset from a template GLB and instance transforms.
///
/// # Arguments
/// * `template_glb` — Template GLB model.
/// * `positions` — Flat `[x, y, z, ...]` instance positions.
/// * `orientations` — Optional quaternion rotations `[qx, qy, qz, qw, ...]`.
/// * `scales` — Optional per-instance scale factors.
/// * `center` — Tile center `[cx, cy, cz]`.
/// * `geometric_error` — Geometric error (default 500.0).
pub fn create_instanced_tileset_i3dm(
    template_glb: &[u8],
    positions: &[f32],
    orientations: Option<&[f32]>,
    scales: Option<&[f32]>,
    center: [f64; 3],
    geometric_error: f64,
) -> Result<TilesetResult, crate::errors::SpatialErrorDetail> {
    let n = positions.len() / 3;
    if n == 0 {
        return Err(SpatialError::InvalidInput.with_detail("need at least one instance position"));
    }

    // Compute bounding sphere radius from positions
    let mut min_pos = [f32::MAX; 3];
    let mut max_pos = [f32::MIN; 3];
    for chunk in positions.chunks_exact(3) {
        for j in 0..3 {
            min_pos[j] = min_pos[j].min(chunk[j]);
            max_pos[j] = max_pos[j].max(chunk[j]);
        }
    }

    // Encode i3dm
    let i3dm = encode_i3dm_tile(template_glb, positions, orientations, scales)?;

    let bounds: crate::octree::Bounds = [
        (min_pos[0] as f64) - center[0],
        (min_pos[1] as f64) - center[1],
        (min_pos[2] as f64) - center[2],
        (max_pos[0] as f64) - center[0],
        (max_pos[1] as f64) - center[1],
        (max_pos[2] as f64) - center[2],
    ];

    let tileset_json = build_i3dm_tileset_json(center, geometric_error, n);

    Ok(TilesetResult {
        tileset_json,
        tiles: vec![i3dm],
        tile_bounds: vec![bounds],
        tile_uris: vec!["0.i3dm".to_string()],
    })
}

/// Build tileset.json for an i3dm tile.
fn build_i3dm_tileset_json(center: [f64; 3], geometric_error: f64, num_instances: usize) -> String {
    format!(
        r#"{{
  "asset": {{
    "version": "1.0",
    "tilesetVersion": "1.0.0",
    "generator": "wasm-spatial-core i3dm encoder"
  }},
  "geometricError": {ge},
  "root": {{
    "boundingVolume": {{
      "sphere": [{cx}, {cy}, {cz}, 1000]
    }},
    "geometricError": {ge},
    "refine": "ADD",
    "content": {{
      "uri": "0.i3dm",
      "boundingVolume": {{
        "sphere": [{cx}, {cy}, {cz}, 1000]
      }}
    }},
    "extras": {{
      "instanceCount": {num_instances},
      "format": "i3dm"
    }}
  }}
}}"#,
        cx = center[0],
        cy = center[1],
        cz = center[2],
        ge = geometric_error,
        num_instances = num_instances,
    )
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Pad a string to 4-byte alignment with spaces.
fn pad_to_4(s: &str) -> String {
    let len = s.len();
    let pad = (4 - len % 4) % 4;
    if pad == 0 {
        s.to_string()
    } else {
        let mut padded = s.to_string();
        padded.extend(std::iter::repeat_n(' ', pad));
        padded
    }
}

/// Convert quaternion to rotation matrix columns (NORMAL_UP, NORMAL_RIGHT).
fn quat_to_rotation_columns(qx: f32, qy: f32, qz: f32, qw: f32) -> (f32, f32, f32, f32, f32, f32) {
    let (ux, uy, uz) = quat_rotate(qx, qy, qz, qw, 0.0, 1.0, 0.0);
    let (rx, ry, rz) = quat_rotate(qx, qy, qz, qw, 1.0, 0.0, 0.0);
    (ux, uy, uz, rx, ry, rz)
}

/// Rotate a vector by a quaternion.
#[inline]
fn quat_rotate(qx: f32, qy: f32, qz: f32, qw: f32, vx: f32, vy: f32, vz: f32) -> (f32, f32, f32) {
    // t = 2 * q × v
    let tx = 2.0 * (qy * vz - qz * vy);
    let ty = 2.0 * (qz * vx - qx * vz);
    let tz = 2.0 * (qx * vy - qy * vx);
    // result = v + qw * t + q × t
    let rx = vx + qw * tx + qy * tz - qz * ty;
    let ry = vy + qw * ty + qz * tx - qx * tz;
    let rz = vz + qw * tz + qx * ty - qy * tx;
    (rx, ry, rz)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a minimal GLB with one triangle.
    fn make_test_glb() -> Vec<u8> {
        use crate::gltf_writer::MeshData;
        let meshes = vec![MeshData {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
            indices: vec![0, 1, 2],
            normals: Some(vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0]),
            colors: None,
            material_index: None,
            mode: 4,
        }];
        build_glb(&meshes, &[])
    }

    // ── B3dm encoding tests ───────────────────────────────────────

    #[test]
    fn test_b3dm_magic() {
        let glb = make_test_glb();
        let b3dm = encode_b3dm_tile(&glb, 1, None).unwrap();
        assert_eq!(&b3dm[0..4], b"b3dm");
    }

    #[test]
    fn test_b3dm_version() {
        let glb = make_test_glb();
        let b3dm = encode_b3dm_tile(&glb, 1, None).unwrap();
        let version = u32::from_le_bytes(b3dm[4..8].try_into().unwrap());
        assert_eq!(version, 1);
    }

    #[test]
    fn test_b3dm_byte_length() {
        let glb = make_test_glb();
        let b3dm = encode_b3dm_tile(&glb, 1, None).unwrap();
        let byte_length = u32::from_le_bytes(b3dm[8..12].try_into().unwrap());
        assert_eq!(byte_length as usize, b3dm.len());
    }

    #[test]
    fn test_b3dm_header_size() {
        let glb = make_test_glb();
        let b3dm = encode_b3dm_tile(&glb, 1, None).unwrap();
        // 28-byte header + FT JSON + FT binary (0) + BT JSON + GLB
        assert!(b3dm.len() >= 28 + glb.len());
    }

    #[test]
    fn test_b3dm_batch_length() {
        let glb = make_test_glb();
        let b3dm = encode_b3dm_tile(&glb, 5, None).unwrap();
        // Header: magic(4) + version(4) + byteLength(4) + ftJSONLen(4) + ftBinLen(4) + btJSONLen(4) + btBinLen(4)
        let ft_json_len = u32::from_le_bytes(b3dm[12..16].try_into().unwrap()) as usize;
        let ft_json = std::str::from_utf8(&b3dm[28..28 + ft_json_len]).unwrap();
        assert!(
            ft_json.contains("5"),
            "FT JSON should contain BATCH_LENGTH=5, got: {}",
            ft_json
        );
    }

    #[test]
    fn test_b3dm_with_batch_table() {
        let glb = make_test_glb();
        let bt = r#"{"id":["mesh0"],"color":[255,0,0]}"#;
        let b3dm = encode_b3dm_tile(&glb, 1, Some(bt)).unwrap();
        assert_eq!(&b3dm[0..4], b"b3dm");
        let byte_length = u32::from_le_bytes(b3dm[8..12].try_into().unwrap());
        assert_eq!(byte_length as usize, b3dm.len());
    }

    #[test]
    fn test_b3dm_invalid_glb() {
        let fake_glb = b"not_a_glb_file_at_all";
        let result = encode_b3dm_tile(fake_glb, 1, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_b3dm_empty_glb() {
        let result = encode_b3dm_tile(&[], 1, None);
        assert!(result.is_err());
    }

    // ── B3dm mesh tileset tests ──────────────────────────────────

    #[test]
    fn test_create_mesh_tileset() {
        let positions = vec![0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0];
        let indices = vec![0, 1, 2];
        let result = create_mesh_tileset(&positions, &indices, None, None, [0.0, 0.0, 0.0], 500.0);
        assert!(result.is_ok());
        let ts = result.unwrap();
        assert_eq!(ts.tile_count(), 1);
        assert!(ts.tileset_json().contains("b3dm"));
        let tile = ts.tile(0).unwrap();
        assert_eq!(&tile[0..4], b"b3dm");
    }

    #[test]
    fn test_create_mesh_tileset_with_normals() {
        let positions = vec![0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0];
        let indices = vec![0, 1, 2];
        let normals = vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0];
        let result = create_mesh_tileset(
            &positions,
            &indices,
            Some(&normals),
            None,
            [0.0, 0.0, 0.0],
            100.0,
        );
        assert!(result.is_ok());
        let ts = result.unwrap();
        assert!(ts.tile(0).unwrap().len() > 28);
    }

    #[test]
    fn test_create_mesh_tileset_with_colors() {
        let positions = vec![0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0];
        let indices = vec![0, 1, 2];
        let colors = vec![255u8, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];
        let result = create_mesh_tileset(
            &positions,
            &indices,
            None,
            Some(&colors),
            [0.0, 0.0, 0.0],
            200.0,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_mesh_tileset_empty() {
        let result = create_mesh_tileset(&[], &[0, 1, 2], None, None, [0.0, 0.0, 0.0], 100.0);
        assert!(result.is_err());
    }

    // ── B3dm instanced tileset tests ─────────────────────────────

    #[test]
    fn test_create_instanced_tileset() {
        let glb = make_test_glb();
        // Two instances: identity + translate(10,0,0)
        let transforms = vec![
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0,
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 0.0, 0.0, 1.0,
        ];
        let result = create_instanced_tileset(&glb, &transforms, [0.0, 0.0, 0.0], 500.0);
        assert!(result.is_ok());
        let ts = result.unwrap();
        assert_eq!(ts.tile_count(), 2);
        assert_eq!(ts.tile_uri(0), Some("0.b3dm"));
        assert_eq!(ts.tile_uri(1), Some("1.b3dm"));
    }

    #[test]
    fn test_create_instanced_tileset_invalid_transforms() {
        let glb = make_test_glb();
        let bad_transforms = vec![1.0, 2.0, 3.0]; // Not a multiple of 16
        let result = create_instanced_tileset(&glb, &bad_transforms, [0.0, 0.0, 0.0], 500.0);
        assert!(result.is_err());
    }

    // ── I3dm encoding tests ────────────────────────────────────

    #[test]
    fn test_i3dm_magic() {
        let glb = make_test_glb();
        let positions = vec![0.0f32, 0.0, 0.0, 10.0, 0.0, 0.0];
        let i3dm = encode_i3dm_tile(&glb, &positions, None, None).unwrap();
        assert_eq!(&i3dm[0..4], b"i3dm");
    }

    #[test]
    fn test_i3dm_version() {
        let glb = make_test_glb();
        let positions = vec![0.0f32, 0.0, 0.0];
        let i3dm = encode_i3dm_tile(&glb, &positions, None, None).unwrap();
        let version = u32::from_le_bytes(i3dm[4..8].try_into().unwrap());
        assert_eq!(version, 1);
    }

    #[test]
    fn test_i3dm_byte_length() {
        let glb = make_test_glb();
        let positions = vec![0.0f32, 0.0, 0.0];
        let i3dm = encode_i3dm_tile(&glb, &positions, None, None).unwrap();
        let byte_length = u32::from_le_bytes(i3dm[8..12].try_into().unwrap());
        assert_eq!(byte_length as usize, i3dm.len());
    }

    #[test]
    fn test_i3dm_gltf_format() {
        let glb = make_test_glb();
        let positions = vec![0.0f32, 0.0, 0.0];
        let i3dm = encode_i3dm_tile(&glb, &positions, None, None).unwrap();
        // gltf_format is at offset 28 (last 4 bytes of 32-byte header)
        let gltf_format = u32::from_le_bytes(i3dm[28..32].try_into().unwrap());
        assert_eq!(gltf_format, 1); // embedded GLB
    }

    #[test]
    fn test_i3dm_header_size() {
        let glb = make_test_glb();
        let positions = vec![0.0f32, 0.0, 0.0];
        let i3dm = encode_i3dm_tile(&glb, &positions, None, None).unwrap();
        assert!(i3dm.len() >= 32 + glb.len());
    }

    #[test]
    fn test_i3dm_with_orientations() {
        let glb = make_test_glb();
        let positions = vec![0.0f32, 0.0, 0.0, 10.0, 0.0, 0.0];
        let orientations = vec![
            0.0, 0.0, 0.0, 1.0, // identity
            0.0, 0.707, 0.0, 0.707, // 90° rotation around Y
        ];
        let i3dm = encode_i3dm_tile(&glb, &positions, Some(&orientations), None).unwrap();
        assert_eq!(&i3dm[0..4], b"i3dm");
        let byte_length = u32::from_le_bytes(i3dm[8..12].try_into().unwrap());
        assert_eq!(byte_length as usize, i3dm.len());
    }

    #[test]
    fn test_i3dm_with_scales() {
        let glb = make_test_glb();
        let positions = vec![0.0f32, 0.0, 0.0, 5.0, 5.0, 5.0];
        let scales = vec![1.0f32, 2.0];
        let i3dm = encode_i3dm_tile(&glb, &positions, None, Some(&scales)).unwrap();
        assert_eq!(&i3dm[0..4], b"i3dm");
    }

    #[test]
    fn test_i3dm_invalid_glb() {
        let fake = b"not_a_glb";
        let positions = vec![0.0f32, 0.0, 0.0];
        let result = encode_i3dm_tile(fake, &positions, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_i3dm_empty_positions() {
        let glb = make_test_glb();
        let result = encode_i3dm_tile(&glb, &[], None, None);
        assert!(result.is_err());
    }

    // ── I3dm tileset tests ──────────────────────────────────────

    #[test]
    fn test_create_instanced_tileset_i3dm() {
        let glb = make_test_glb();
        let positions = vec![0.0f32, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 0.0];
        let result =
            create_instanced_tileset_i3dm(&glb, &positions, None, None, [0.0, 0.0, 0.0], 500.0);
        assert!(result.is_ok());
        let ts = result.unwrap();
        assert_eq!(ts.tile_count(), 1);
        assert!(ts.tileset_json().contains("i3dm"));
        assert!(ts.tileset_json().contains("instanceCount"));
        let tile = ts.tile(0).unwrap();
        assert_eq!(&tile[0..4], b"i3dm");
    }

    #[test]
    fn test_create_instanced_tileset_i3dm_empty() {
        let glb = make_test_glb();
        let result = create_instanced_tileset_i3dm(&glb, &[], None, None, [0.0, 0.0, 0.0], 500.0);
        assert!(result.is_err());
    }

    // ── Helper tests ────────────────────────────────────────────

    #[test]
    fn test_pad_to_4() {
        assert_eq!(pad_to_4("abcd"), "abcd");
        assert_eq!(pad_to_4("abc"), "abc ");
        assert_eq!(pad_to_4("ab"), "ab  ");
        assert_eq!(pad_to_4("a"), "a   ");
        assert_eq!(pad_to_4(""), "");
        assert_eq!(pad_to_4("abcde"), "abcde   "); // 5 + 3 = 8
    }

    #[test]
    fn test_quat_rotate_identity() {
        let (rx, ry, rz) = quat_rotate(0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 3.0);
        assert!((rx - 1.0).abs() < 1e-6);
        assert!((ry - 2.0).abs() < 1e-6);
        assert!((rz - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_quat_rotate_90_y() {
        // 90° rotation around Y: (1,0,0) → (0,0,-1)
        let q = std::f32::consts::FRAC_1_SQRT_2;
        let (rx, ry, rz) = quat_rotate(0.0, q, 0.0, q, 1.0, 0.0, 0.0);
        assert!((rx).abs() < 1e-5);
        assert!((ry).abs() < 1e-5);
        assert!((rz + 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_quat_to_rotation_columns_identity() {
        let (ux, uy, uz, rx, ry, rz) = quat_to_rotation_columns(0.0, 0.0, 0.0, 1.0);
        // UP column should be (0,1,0)
        assert!((ux).abs() < 1e-5);
        assert!((uy - 1.0).abs() < 1e-5);
        assert!((uz).abs() < 1e-5);
        // RIGHT column should be (1,0,0)
        assert!((rx - 1.0).abs() < 1e-5);
        assert!((ry).abs() < 1e-5);
        assert!((rz).abs() < 1e-5);
    }

    #[test]
    fn test_transform_glb_identity() {
        let glb = make_test_glb();
        let identity = vec![
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let result = transform_glb(&glb, &identity).unwrap();
        // Same GLB magic and length
        assert_eq!(&result[0..4], b"glTF");
        assert_eq!(result.len(), glb.len());
    }

    #[test]
    fn test_transform_glb_translate() {
        let glb = make_test_glb();
        let translate = vec![
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 100.0, 200.0, 300.0, 1.0,
        ];
        let result = transform_glb(&glb, &translate).unwrap();
        assert_eq!(&result[0..4], b"glTF");
        // Positions should be shifted by (100, 200, 300)
        // Parse the BIN and check first position
        let json_len = u32::from_le_bytes(result[12..16].try_into().unwrap()) as usize;
        let bin_start = 20 + json_len + 8;
        // First vertex was at (0,0,0) → should be ~(100, 200, 300)
        let px = f32::from_le_bytes(result[bin_start..bin_start + 4].try_into().unwrap());
        let py = f32::from_le_bytes(result[bin_start + 4..bin_start + 8].try_into().unwrap());
        let pz = f32::from_le_bytes(result[bin_start + 8..bin_start + 12].try_into().unwrap());
        assert!((px - 100.0).abs() < 0.1);
        assert!((py - 200.0).abs() < 0.1);
        assert!((pz - 300.0).abs() < 0.1);
    }

    // ── Orientation/scale mismatch tests ────────────────────────

    #[test]
    fn test_i3dm_orientation_mismatch() {
        let glb = make_test_glb();
        let positions = vec![0.0f32, 0.0, 0.0, 10.0, 0.0, 0.0]; // 2 instances
        let bad_orient = vec![0.0, 0.0, 0.0, 1.0]; // only 1 rotation
        let result = encode_i3dm_tile(&glb, &positions, Some(&bad_orient), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_i3dm_scale_mismatch() {
        let glb = make_test_glb();
        let positions = vec![0.0f32, 0.0, 0.0, 10.0, 0.0, 0.0]; // 2 instances
        let bad_scales = vec![1.0]; // only 1 scale
        let result = encode_i3dm_tile(&glb, &positions, None, Some(&bad_scales));
        assert!(result.is_err());
    }
}
