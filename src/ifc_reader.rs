//! IFC/BIM Geometry Extraction (Experimental)
//!
//! Parses IFC-SPF (STEP Physical File) text to extract mesh geometry from
//! `IFCEXTRUDEDAREASOLID` entities. This is a practical subset — full IFC
//! parsing is extremely complex and outside the scope of a WASM module.
//!
//! Strategy: lightweight text parsing (regex / string matching), no heavy IFC crates.

use wasm_bindgen::prelude::*;

// ===========================================================================
// Data Structures
// ===========================================================================

/// A single mesh extracted from an IFC entity.
#[wasm_bindgen]
#[derive(Clone)]
pub struct IfcMesh {
    positions: Vec<f64>,
    indices: Vec<u32>,
}

impl IfcMesh {
    /// Vertex positions `[x0, y0, z0, x1, y1, z1, ...]`.
    pub fn get_positions(&self) -> &[f64] {
        &self.positions
    }

    /// Triangle indices `[i0, i1, i2, ...]`.
    pub fn get_indices(&self) -> &[u32] {
        &self.indices
    }
}

#[wasm_bindgen]
impl IfcMesh {
    /// Vertex positions as `Float64Array` `[x0, y0, z0, x1, y1, z1, ...]`.
    #[wasm_bindgen(getter)]
    pub fn positions(&self) -> js_sys::Float64Array {
        let arr = js_sys::Float64Array::new_with_length(self.positions.len() as u32);
        arr.copy_from(&self.positions);
        arr
    }

    /// Triangle indices as `Uint32Array` `[i0, i1, i2, ...]`.
    #[wasm_bindgen(getter)]
    pub fn indices(&self) -> js_sys::Uint32Array {
        let arr = js_sys::Uint32Array::new_with_length(self.indices.len() as u32);
        arr.copy_from(&self.indices);
        arr
    }

    /// Number of vertices.
    #[wasm_bindgen(getter, js_name = "vertexCount")]
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    /// Number of triangles.
    #[wasm_bindgen(getter, js_name = "triangleCount")]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Result of parsing IFC geometry.
#[wasm_bindgen]
pub struct IfcGeometryResult {
    meshes: Vec<IfcMesh>,
}

impl IfcGeometryResult {
    /// Get the extracted meshes.
    pub fn get_meshes(&self) -> &[IfcMesh] {
        &self.meshes
    }

    /// Total number of meshes extracted.
    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }
}

#[wasm_bindgen]
impl IfcGeometryResult {
    /// Array of extracted meshes.
    #[wasm_bindgen(getter)]
    pub fn meshes(&self) -> js_sys::Array {
        let arr = js_sys::Array::new_with_length(self.meshes.len() as u32);
        for (i, mesh) in self.meshes.iter().enumerate() {
            let js_val: JsValue = mesh.clone().into();
            arr.set(i as u32, js_val);
        }
        arr
    }

    /// Total number of meshes extracted (JS getter delegates to impl method).
    #[wasm_bindgen(getter, js_name = "meshCount")]
    pub fn js_mesh_count(&self) -> usize {
        self.mesh_count()
    }
}

// ===========================================================================
// Internal IFC Entity Storage
// ===========================================================================

/// Parsed data for an IFCEXTRUDEDAREASOLID entity.
struct ExtrudedAreaSolid {
    /// Reference ID for the shape representation (for tracing).
    #[allow(dead_code)]
    shape_ref: Option<u32>,
    /// Reference ID for the placement (position).
    placement_ref: Option<u32>,
    /// Reference ID for the extrusion direction.
    direction_ref: Option<u32>,
    /// Extrusion depth along the direction.
    depth: f64,
    /// Reference ID for the profile (polyline/cartesian point list).
    #[allow(dead_code)]
    profile_ref: Option<u32>,
}

/// A 3D direction vector.
#[derive(Clone)]
struct Direction {
    dx: f64,
    dy: f64,
    dz: f64,
}

/// A 3D cartesian point.
#[derive(Clone)]
struct CartesianPoint {
    x: f64,
    y: f64,
    z: f64,
}

/// A 2D cartesian point (used in profiles).
#[derive(Clone)]
struct CartesianPoint2D {
    x: f64,
    y: f64,
}

/// Axis2Placement3D: position + direction + ref direction.
struct Axis2Placement3D {
    location: CartesianPoint,
    #[allow(dead_code)]
    axis: Direction,
    #[allow(dead_code)]
    ref_dir: Option<Direction>,
}

// ===========================================================================
// IFC-SPF Parsing
// ===========================================================================

/// Extract the entity ID number from a line like `#123=...`.
fn extract_id(line: &str) -> Option<u32> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix('#') {
        if let Some(eq_pos) = rest.find('=') {
            return rest[..eq_pos].parse::<u32>().ok();
        }
    }
    None
}

/// Extract a tuple of floats from an IFC list like `(1.0,2.0,3.0)`.
/// Handles text like `IFCDIRECTION((0.0,0.0,1.0));` by finding the outermost
/// `(...)` and then stripping a nested inner `(...)` if present.
fn extract_float_tuple(text: &str) -> Vec<f64> {
    let mut result = Vec::new();
    let content = extract_first_paren_group(text);
    // If the content is itself wrapped in (), unwrap it
    let inner = content.trim();
    let inner = if inner.starts_with('(') && inner.ends_with(')') {
        &inner[1..inner.len() - 1]
    } else {
        inner
    };
    for part in inner.split(',') {
        if let Ok(v) = part.trim().parse::<f64>() {
            result.push(v);
        }
    }
    result
}

/// Extract a list of reference IDs from an IFC list like `(#10,#11,#12)`.
/// Handles text like `IFCPOLYLINE((#11,#12,#13,#14));` by finding the outermost
/// `(...)` and then stripping a nested inner `(...)` if present.
fn extract_ref_list(text: &str) -> Vec<u32> {
    let mut result = Vec::new();
    let content = extract_first_paren_group(text);
    // If the content is itself wrapped in (), unwrap it
    let inner = content.trim();
    let inner = if inner.starts_with('(') && inner.ends_with(')') {
        &inner[1..inner.len() - 1]
    } else {
        inner
    };
    for part in inner.split(',') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix('#') {
            if let Ok(v) = rest.parse::<u32>() {
                result.push(v);
            }
        }
    }
    result
}

/// Find the content between the first `(` and its matching `)`.
fn extract_first_paren_group(text: &str) -> String {
    let start = match text.find('(') {
        Some(i) => i + 1,
        None => return String::new(),
    };
    let chars: Vec<char> = text[start..].chars().collect();
    let mut end = 0;
    let mut depth = 1i32;
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    text[start..start + end].to_string()
}

/// Extract a single reference ID from a string like `#10` or `$`.
fn extract_ref(text: &str) -> Option<u32> {
    let trimmed = text.trim();
    if trimmed == "$" {
        return None;
    }
    trimmed
        .strip_prefix('#')
        .and_then(|r| r.parse::<u32>().ok())
}

/// Parse an IFC-SPF text into structured maps.
#[allow(clippy::type_complexity)]
fn parse_ifc_entities(
    text: &str,
) -> (
    std::collections::HashMap<u32, String>, // id -> entity type keyword
    std::collections::HashMap<u32, ExtrudedAreaSolid>,
    std::collections::HashMap<u32, Direction>,
    std::collections::HashMap<u32, CartesianPoint>,
    std::collections::HashMap<u32, CartesianPoint2D>,
    std::collections::HashMap<u32, Axis2Placement3D>,
    std::collections::HashMap<u32, Vec<u32>>, // polyline: id -> point refs
) {
    let mut types: std::collections::HashMap<u32, String> =
        std::collections::HashMap::new();
    let mut extrusions: std::collections::HashMap<u32, ExtrudedAreaSolid> =
        std::collections::HashMap::new();
    let mut directions: std::collections::HashMap<u32, Direction> =
        std::collections::HashMap::new();
    let mut points3d: std::collections::HashMap<u32, CartesianPoint> =
        std::collections::HashMap::new();
    let mut points2d: std::collections::HashMap<u32, CartesianPoint2D> =
        std::collections::HashMap::new();
    let mut placements: std::collections::HashMap<u32, Axis2Placement3D> =
        std::collections::HashMap::new();
    let mut polylines: std::collections::HashMap<u32, Vec<u32>> =
        std::collections::HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('#') || !line.contains('=') {
            continue;
        }

        let id = match extract_id(line) {
            Some(v) => v,
            None => continue,
        };

        // Extract the entity keyword (after `=`)
        let after_eq = line.split_once('=').map(|x| x.1).unwrap_or("");
        let keyword = after_eq
            .trim_start()
            .split('(')
            .next()
            .unwrap_or("")
            .trim();

        types.insert(id, keyword.to_uppercase());

        match keyword.to_uppercase().as_str() {
            "IFCEXTRUDEDAREASOLID" => {
                // #1=IFCEXTRUDEDAREASOLID(#2,#3,#4,100.0);
                let inner = after_eq
                    .trim_start()
                    .trim_start_matches(keyword.trim())
                    .trim_start_matches('(')
                    .trim_end_matches(");");


                // Split by comma, respecting parentheses
                let parts: Vec<&str> = split_ifc_args(inner);


                if parts.len() >= 4 {
                    let depth = parts[3].trim().parse::<f64>().unwrap_or(0.0);
                    extrusions.insert(
                        id,
                        ExtrudedAreaSolid {
                            shape_ref: extract_ref(parts[0]),
                            placement_ref: extract_ref(parts[1]),
                            direction_ref: extract_ref(parts[2]),
                            depth,
                            profile_ref: None, // will resolve from shape_ref
                        },
                    );
                }
            }
            "IFCDIRECTION" => {
                let floats = extract_float_tuple(after_eq);
                if floats.len() >= 3 {
                    directions.insert(
                        id,
                        Direction {
                            dx: floats[0],
                            dy: floats[1],
                            dz: floats[2],
                        },
                    );
                }
            }
            "IFCCARTESIANPOINT" => {
                let floats = extract_float_tuple(after_eq);
                if floats.len() >= 3 {
                    points3d.insert(
                        id,
                        CartesianPoint {
                            x: floats[0],
                            y: floats[1],
                            z: floats[2],
                        },
                    );
                } else if floats.len() == 2 {
                    points2d.insert(
                        id,
                        CartesianPoint2D {
                            x: floats[0],
                            y: floats[1],
                        },
                    );
                }
            }
            "IFCAXIS2PLACEMENT3D" => {
                let parts: Vec<&str> = split_ifc_args(
                    after_eq
                        .trim_start()
                        .trim_start_matches("IFCAXIS2PLACEMENT3D")
                        .trim_start_matches('(')
                        .trim_end_matches(')')
                        .trim_end_matches(';'),
                );
                let loc_ref = parts.first().and_then(|p| extract_ref(p));
                let loc = loc_ref
                    .and_then(|r| points3d.get(&r).cloned())
                    .unwrap_or(CartesianPoint {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    });
                let axis_ref = parts.get(1).and_then(|p| extract_ref(p));
                let axis = axis_ref
                    .and_then(|r| directions.get(&r).cloned())
                    .unwrap_or(Direction {
                        dx: 0.0,
                        dy: 0.0,
                        dz: 1.0,
                    });
                placements.insert(id, Axis2Placement3D { location: loc, axis, ref_dir: None });
            }
            "IFCPOLYLINE" => {
                let refs = extract_ref_list(after_eq);
                if !refs.is_empty() {
                    polylines.insert(id, refs);
                }
            }
            _ => {}
        }
    }

    (
        types,
        extrusions,
        directions,
        points3d,
        points2d,
        placements,
        polylines,
    )
}

/// Split IFC argument list by commas, respecting nested parentheses.
fn split_ifc_args(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;
    let chars: Vec<char> = s.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                result.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        result.push(&s[start..]);
    }
    result
}

/// Triangulate a 2D polygon using the ear-clipping method (simple fan from centroid).
/// For convex polygons this works perfectly. For concave polygons it may produce
/// incorrect results but is acceptable for an experimental IFC parser.
fn triangulate_profile_2d(vertices: &[(f64, f64)]) -> Vec<u32> {
    let n = vertices.len();
    if n < 3 {
        return Vec::new();
    }

    if n == 3 {
        return vec![0, 1, 2];
    }

    // Compute centroid
    let (_cx, _cy): (f64, f64) = (
        vertices.iter().map(|v| v.0).sum::<f64>() / n as f64,
        vertices.iter().map(|v| v.1).sum::<f64>() / n as f64,
    );

    let mut indices = Vec::with_capacity((n - 2) * 3);
    for i in 0..n {
        let next = (i + 1) % n;
        indices.push(n as u32); // centroid vertex
        indices.push(i as u32);
        indices.push(next as u32);
    }

    // Actually we need the centroid in the vertex list too.
    // Let's use a different approach: simple ear-triangulation from vertex 0.
    indices.clear();
    for i in 1..n - 1 {
        indices.push(0);
        indices.push(i as u32);
        indices.push((i + 1) as u32);
    }

    indices
}

/// Core IFC geometry parser (pure Rust, testable without WASM runtime).
pub fn parse_ifc_geometry_core(text: &str) -> IfcGeometryResult {
    let (_types, extrusions, directions, _points3d, points2d, placements, polylines) =
        parse_ifc_entities(text);


    // Pre-resolve all polyline 2D profiles
    let mut resolved_profiles: std::collections::HashMap<u32, Vec<(f64, f64)>> =
        std::collections::HashMap::new();
    for (poly_id, poly_refs) in &polylines {
        let pts: Vec<(f64, f64)> = poly_refs
            .iter()
            .filter_map(|&r| points2d.get(&r).map(|p| (p.x, p.y)))
            .collect();
        if pts.len() >= 3 {
            resolved_profiles.insert(*poly_id, pts);
        }
    }

    // Build a mapping from extrusion ID → polyline ID.
    // Strategy: scan the extrusion's shape_ref's IFCSHAPEREPRESENTATION line for
    // any polyline references. If that fails, fall back to nearest polyline by ID.
    let mut used_polylines: std::collections::HashSet<u32> = std::collections::HashSet::new();

    let mut meshes = Vec::new();

    // Process extrusions in order of their ID for deterministic assignment
    let mut sorted_extrusions: Vec<(u32, &ExtrudedAreaSolid)> =
        extrusions.iter().map(|(id, e)| (*id, e)).collect();
    sorted_extrusions.sort_by_key(|(id, _)| *id);

    for (ext_id, ext) in sorted_extrusions {
        // Get extrusion direction
        let dir = ext
            .direction_ref
            .and_then(|r| directions.get(&r).cloned())
            .unwrap_or(Direction {
                dx: 0.0,
                dy: 0.0,
                dz: 1.0,
            });

        // Get placement position
        let placement = ext.placement_ref.and_then(|r| placements.get(&r));

        let (ox, oy, oz) = match placement {
            Some(p) => (p.location.x, p.location.y, p.location.z),
            None => (0.0, 0.0, 0.0),
        };

        // Resolve profile: try to find the best matching polyline
        let profile_points = find_profile_for_extrusion(
            ext_id,
            ext,
            text,
            &resolved_profiles,
            &mut used_polylines,
        );

        if profile_points.len() < 3 || ext.depth <= 0.0 {
            continue;
        }

        // Triangulate the bottom face (2D)
        let indices_2d = triangulate_profile_2d(&profile_points);

        if indices_2d.is_empty() {
            continue;
        }

        // Build bottom and top face vertices
        let n = profile_points.len();
        let mut positions = Vec::with_capacity(n * 2 * 3);
        let mut indices = Vec::with_capacity(indices_2d.len() * 4); // bottom + top + sides

        // Bottom face vertices (z = 0)
        for (x, y) in &profile_points {
            positions.push(ox + x);
            positions.push(oy + y);
            positions.push(oz);
        }

        // Top face vertices (z = depth)
        for (x, y) in &profile_points {
            positions.push(ox + x + dir.dx * ext.depth);
            positions.push(oy + y + dir.dy * ext.depth);
            positions.push(oz + dir.dz * ext.depth);
        }

        // Bottom face indices (reversed winding for correct normals)
        for i in (0..indices_2d.len()).step_by(3) {
            indices.push(indices_2d[i]);
            indices.push(indices_2d[i + 2]);
            indices.push(indices_2d[i + 1]);
        }

        // Top face indices
        let n_u32 = n as u32;
        for i in (0..indices_2d.len()).step_by(3) {
            indices.push(n_u32 + indices_2d[i]);
            indices.push(n_u32 + indices_2d[i + 1]);
            indices.push(n_u32 + indices_2d[i + 2]);
        }

        // Side faces (quads → two triangles each)
        for i in 0..n_u32 {
            let next = (i + 1) % n_u32;
            // Triangle 1: bottom[i], bottom[next], top[i]
            indices.push(i);
            indices.push(next);
            indices.push(n_u32 + i);
            // Triangle 2: bottom[next], top[next], top[i]
            indices.push(next);
            indices.push(n_u32 + next);
            indices.push(n_u32 + i);
        }

        meshes.push(IfcMesh { positions, indices });
    }

    IfcGeometryResult { meshes }
}

/// Find the best polyline profile for an extrusion entity.
fn find_profile_for_extrusion(
    ext_id: u32,
    ext: &ExtrudedAreaSolid,
    full_text: &str,
    resolved_profiles: &std::collections::HashMap<u32, Vec<(f64, f64)>>,
    used_polylines: &mut std::collections::HashSet<u32>,
) -> Vec<(f64, f64)> {
    // Strategy 1: Look at the IFC line for the shape_ref (first arg) and find
    // polyline references in the IFCSHAPEREPRESENTATION line.
    if let Some(shape_ref) = ext.shape_ref {
        // Find the SHAPEREPRESENTATION line and look for polyline refs in it
        let target_line = format!("#{}=IFCSHAPEREPRESENTATION", shape_ref);
        for line in full_text.lines() {
            if line.contains(&target_line) {
                // Extract all #N references from this line
                let refs = extract_all_refs_from_line(line);
                for r in refs {
                    if let Some(pts) = resolved_profiles.get(&r) {
                        used_polylines.insert(r);
                        return pts.clone();
                    }
                }
            }
        }
    }

    // Strategy 2: Find nearest unused polyline (by ID proximity)
    let mut best: Option<(u32, Vec<(f64, f64)>)> = None;
    for (&poly_id, pts) in resolved_profiles {
        if used_polylines.contains(&poly_id) {
            continue;
        }
        let distance = poly_id.abs_diff(ext_id);
        if best.is_none() || distance < best.as_ref().unwrap().0 {
            best = Some((distance, pts.clone()));
        }
    }

    if let Some((_, pts)) = best {
        // Mark it as used if we found it by proximity
        if let Some((&poly_id, _)) = resolved_profiles.iter().find(|(_, p)| **p == pts) {
            used_polylines.insert(poly_id);
        }
        return pts;
    }

    // Strategy 3: Use any available polyline
    for (&poly_id, pts) in resolved_profiles {
        if !used_polylines.contains(&poly_id) {
            used_polylines.insert(poly_id);
            return pts.clone();
        }
    }

    Vec::new()
}

/// Extract all #N reference IDs from a line of IFC text.
fn extract_all_refs_from_line(line: &str) -> Vec<u32> {
    let mut result = Vec::new();
    let mut in_ref = false;
    let mut current = String::new();
    for ch in line.chars() {
        if ch == '#' {
            in_ref = true;
            current.clear();
        } else if in_ref && ch.is_ascii_digit() {
            current.push(ch);
        } else if in_ref && !ch.is_ascii_digit() {
            if let Ok(v) = current.parse::<u32>() {
                result.push(v);
            }
            in_ref = false;
        }
    }
    // Handle ref at end of line
    if in_ref && !current.is_empty() {
        if let Ok(v) = current.parse::<u32>() {
            result.push(v);
        }
    }
    result
}

/// Parse IFC-SPF text and extract mesh geometry from IFCEXTRUDEDAREASOLID entities.
///
/// This is an **experimental** feature that extracts a practical subset of IFC geometry:
/// - `IFCEXTRUDEDAREASOLID` entities are triangulated into indexed meshes
/// - Associated `IFCPOLYLINE` profiles provide the cross-section
/// - `IFCDIRECTION` and `IFCAXIS2PLACEMENT3D` define extrusion direction and position
///
/// Returns an `IfcGeometryResult` containing all extracted meshes.
///
/// # Arguments
///
/// * `text` - The full IFC-SPF file content as a UTF-8 string.
///
/// # Example
///
/// ```ignore
/// let result = parse_ifc_geometry(ifc_text);
/// console.log(`Extracted ${result.meshCount()} meshes`);
/// ```
#[wasm_bindgen(js_name = "parseIfcGeometry")]
pub fn parse_ifc_geometry(text: &str) -> IfcGeometryResult {
    // Silently return empty result for oversized input
    if text.len() > 100 * 1024 * 1024 {
        return IfcGeometryResult { meshes: Vec::new() };
    }
    parse_ifc_geometry_core(text)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal IFC file with one extruded area solid (a simple box).
    fn sample_ifc_box() -> &'static str {
        r#"
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('A minimal IFC file'),'2;1');
FILE_NAME('box.ifc','2024-01-01',('Author'),(''),'IfcOpenShell','IfcOpenShell','');
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1=IFCEXTRUDEDAREASOLID(#2,#3,#4,10.0);
#2=IFCSHAPEREPRESENTATION('Box','Body',(#10),#100);
#3=IFCAXIS2PLACEMENT3D(#5,#6,$);
#4=IFCDIRECTION((0.0,0.0,1.0));
#5=IFCCARTESIANPOINT((0.0,0.0,0.0));
#6=IFCDIRECTION((0.0,1.0,0.0));
#10=IFCPOLYLINE((#11,#12,#13,#14));
#11=IFCCARTESIANPOINT((0.0,0.0));
#12=IFCCARTESIANPOINT((5.0,0.0));
#13=IFCCARTESIANPOINT((5.0,5.0));
#14=IFCCARTESIANPOINT((0.0,5.0));
#100=IFCAXIS2PLACEMENT3D(#5,#6,$);
ENDSEC;
END-ISO-10303-21;
"#
    }

    #[test]
    fn test_parse_ifc_box() {
        let result = parse_ifc_geometry_core(sample_ifc_box());
        assert_eq!(result.mesh_count(), 1, "Should extract exactly 1 mesh");

        let mesh = &result.meshes[0];
        // Box profile has 4 vertices → bottom (4) + top (4) = 8 vertices
        assert_eq!(mesh.vertex_count(), 8);
        // Each face contributes triangles:
        // - Bottom face: 2 triangles (4 vertices fan)
        // - Top face: 2 triangles
        // - Side faces: 4 quads × 2 triangles = 8
        // Total: 12 triangles = 36 indices
        assert_eq!(mesh.triangle_count(), 12);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn test_parse_ifc_empty_and_invalid() {
        // Empty string
        let result = parse_ifc_geometry_core("");
        assert_eq!(result.mesh_count(), 0);

        // Invalid content
        let result = parse_ifc_geometry_core("this is not IFC data at all");
        assert_eq!(result.mesh_count(), 0);

        // Only header, no extrusion entities
        let result = parse_ifc_geometry_core(
            "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\n",
        );
        assert_eq!(result.mesh_count(), 0);
    }

    #[test]
    fn test_ifc_positions_indices_consistency() {
        let result = parse_ifc_geometry_core(sample_ifc_box());
        assert_eq!(result.mesh_count(), 1);

        let mesh = &result.meshes[0];
        let vc = mesh.vertex_count() as u32;
        let max_idx = *mesh.indices.iter().max().unwrap();

        // All indices must reference valid vertices
        assert!(
            max_idx < vc,
            "Max index {} exceeds vertex count {}",
            max_idx,
            vc
        );

        // Positions count must be vertex_count * 3
        assert_eq!(mesh.positions.len(), mesh.vertex_count() * 3);

        // Indices count must be triangle_count * 3
        assert_eq!(mesh.indices.len(), mesh.triangle_count() * 3);
    }

    #[test]
    fn test_extract_float_tuple() {
        let result = extract_float_tuple("(1.0, 2.0, 3.0)");
        assert_eq!(result, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_extract_ref_list() {
        let result = extract_ref_list("(#10, #11, #12)");
        assert_eq!(result, vec![10, 11, 12]);
    }

    #[test]
    fn test_extract_ref() {
        assert_eq!(extract_ref("#42"), Some(42));
        assert_eq!(extract_ref("$"), None);
    }

    #[test]
    fn test_split_ifc_args() {
        let result = split_ifc_args("#2,#3,#4,100.0");
        assert_eq!(result.len(), 4);
        assert_eq!(result[3].trim(), "100.0");
    }

    #[test]
    fn test_triangulate_profile_triangle() {
        let verts = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)];
        let indices = triangulate_profile_2d(&verts);
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_triangulate_profile_quad() {
        let verts = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let indices = triangulate_profile_2d(&verts);
        assert_eq!(indices.len(), 6); // 2 triangles
    }

    #[test]
    fn test_triangulate_profile_degenerate() {
        let verts = vec![(0.0, 0.0), (1.0, 0.0)];
        let indices = triangulate_profile_2d(&verts);
        assert!(indices.is_empty());
    }

    #[test]
    fn test_multiple_extrusions() {
        let ifc = r#"
#1=IFCEXTRUDEDAREASOLID(#2,#3,#4,5.0);
#2=IFCSHAPEREPRESENTATION('Body','Body',(#10),#100);
#3=IFCAXIS2PLACEMENT3D(#5,#6,$);
#4=IFCDIRECTION((0.0,0.0,1.0));
#5=IFCCARTESIANPOINT((0.0,0.0,0.0));
#6=IFCDIRECTION((0.0,1.0,0.0));
#10=IFCPOLYLINE((#11,#12,#13));
#11=IFCCARTESIANPOINT((0.0,0.0));
#12=IFCCARTESIANPOINT((3.0,0.0));
#13=IFCCARTESIANPOINT((1.5,2.0));
#100=IFCAXIS2PLACEMENT3D(#5,#6,$);
#20=IFCEXTRUDEDAREASOLID(#21,#22,#4,8.0);
#21=IFCSHAPEREPRESENTATION('Body','Body',(#30),#200);
#22=IFCAXIS2PLACEMENT3D(#25,#6,$);
#25=IFCCARTESIANPOINT((10.0,0.0,0.0));
#30=IFCPOLYLINE((#31,#32,#33,#34));
#31=IFCCARTESIANPOINT((0.0,0.0));
#32=IFCCARTESIANPOINT((4.0,0.0));
#33=IFCCARTESIANPOINT((4.0,4.0));
#34=IFCCARTESIANPOINT((0.0,4.0));
#200=IFCAXIS2PLACEMENT3D(#5,#6,$);
"#;
        let result = parse_ifc_geometry_core(ifc);
        assert_eq!(result.mesh_count(), 2);
    }
}
