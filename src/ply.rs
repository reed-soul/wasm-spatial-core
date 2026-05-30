//! PLY (Polygon File Format) parser — ASCII and binary_little_endian.
//!
//! Extracts vertex positions, colors (RGB), normals, and face count.

use wasm_bindgen::prelude::*;

use crate::errors::{SpatialError, SpatialErrorDetail};
use crate::DEFAULT_MAX_INPUT_SIZE;

// ===========================================================================
// PLY Result — WASM class
// ===========================================================================

/// Result of parsing a PLY file. Contains vertex positions, optional colors,
/// optional normals, and face count.
#[wasm_bindgen]
pub struct PlyResult {
    /// Vertex positions as flat Float32Array [x0, y0, z0, x1, y1, z1, ...]
    positions: Vec<f32>,
    /// Vertex colors as flat Uint8Array [r0, g0, b0, r1, g1, b1, ...] (0-255)
    colors: Option<Vec<u8>>,
    /// Vertex normals as flat Float32Array [nx0, ny0, nz0, ...]
    normals: Option<Vec<f32>>,
    /// Number of vertices
    vertex_count: u32,
    /// Number of faces (triangles/polygons)
    face_count: u32,
}

#[wasm_bindgen]
impl PlyResult {
    /// Vertex positions as Float32Array [x, y, z, x, y, z, ...].
    #[wasm_bindgen(getter)]
    pub fn positions(&self) -> js_sys::Float32Array {
        js_sys::Float32Array::from(&self.positions[..])
    }

    /// Vertex colors as Uint8Array [r, g, b, ...], or null if no color data.
    #[wasm_bindgen(getter)]
    pub fn colors(&self) -> js_sys::Uint8Array {
        match &self.colors {
            Some(c) => js_sys::Uint8Array::from(&c[..]),
            None => js_sys::Uint8Array::new_with_length(0),
        }
    }

    /// Whether color data is present.
    #[wasm_bindgen(js_name = "hasColors")]
    pub fn has_colors(&self) -> bool {
        self.colors.is_some()
    }

    /// Vertex normals as Float32Array [nx, ny, nz, ...], or null if no normal data.
    #[wasm_bindgen(getter)]
    pub fn normals(&self) -> js_sys::Float32Array {
        match &self.normals {
            Some(n) => js_sys::Float32Array::from(&n[..]),
            None => js_sys::Float32Array::new_with_length(0),
        }
    }

    /// Whether normal data is present.
    #[wasm_bindgen(js_name = "hasNormals")]
    pub fn has_normals(&self) -> bool {
        self.normals.is_some()
    }

    /// Number of vertices.
    #[wasm_bindgen(js_name = "vertexCount", getter)]
    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    /// Number of faces (polygons).
    #[wasm_bindgen(js_name = "faceCount", getter)]
    pub fn face_count(&self) -> u32 {
        self.face_count
    }
}

// ===========================================================================
// PLY Header Parsing
// ===========================================================================

/// Parsed PLY header metadata.
struct PlyHeader {
    format: PlyFormat,
    vertex_count: u32,
    face_count: u32,
    vertex_properties: Vec<PlyProperty>,
    /// Byte offset where data begins (after "end_header\n")
    data_offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PlyFormat {
    Ascii,
    BinaryLittleEndian,
    #[allow(dead_code)]
    BinaryBigEndian,
}

#[derive(Debug, Clone)]
enum PlyPropertyType {
    Float,
    Double,
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    // list types handled separately
}

#[derive(Debug, Clone)]
struct PlyProperty {
    name: String,
    type_: PlyPropertyType,
}

fn parse_property_type(s: &str) -> Option<PlyPropertyType> {
    match s.trim().to_lowercase().as_str() {
        "float" | "float32" => Some(PlyPropertyType::Float),
        "double" | "float64" => Some(PlyPropertyType::Double),
        "char" | "int8" => Some(PlyPropertyType::Int8),
        "uchar" | "uint8" => Some(PlyPropertyType::Uint8),
        "short" | "int16" => Some(PlyPropertyType::Int16),
        "ushort" | "uint16" => Some(PlyPropertyType::Uint16),
        "int" | "int32" => Some(PlyPropertyType::Int32),
        "uint" | "uint32" => Some(PlyPropertyType::Uint32),
        _ => None,
    }
}

fn type_size(t: &PlyPropertyType) -> usize {
    match t {
        PlyPropertyType::Float => 4,
        PlyPropertyType::Double => 8,
        PlyPropertyType::Int8 | PlyPropertyType::Uint8 => 1,
        PlyPropertyType::Int16 | PlyPropertyType::Uint16 => 2,
        PlyPropertyType::Int32 | PlyPropertyType::Uint32 => 4,
    }
}

/// Parse PLY header from bytes. Returns the header and the offset where data begins.
fn parse_ply_header(bytes: &[u8]) -> Result<PlyHeader, String> {
    // Find end_header
    let header_end = match find_end_header(bytes) {
        Some(pos) => pos,
        None => return Err("PLY header: end_header not found".to_string()),
    };

    let header_str =
        String::from_utf8(bytes[..header_end].to_vec()).map_err(|e| format!("Invalid UTF-8 in PLY header: {}", e))?;

    // First line must be "ply"
    let mut lines = header_str.lines();
    let first_line = lines.next().ok_or("PLY header: empty")?.trim();
    if first_line != "ply" {
        return Err(format!("PLY header: expected 'ply', got '{}'", first_line));
    }

    // Parse format
    let format_line = lines.next().ok_or("PLY header: no format line")?.trim();
    let format = if format_line.starts_with("format ascii") {
        PlyFormat::Ascii
    } else if format_line.starts_with("format binary_little_endian") {
        PlyFormat::BinaryLittleEndian
    } else if format_line.starts_with("format binary_big_endian") {
        PlyFormat::BinaryBigEndian
    } else {
        return Err(format!("Unsupported PLY format: {}", format_line));
    };

    let mut vertex_count: u32 = 0;
    let mut face_count: u32 = 0;
    let mut vertex_properties: Vec<PlyProperty> = Vec::new();
    let mut in_vertex_element = false;

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with("comment") {
            continue;
        }
        if line == "end_header" {
            break;
        }
        if line.starts_with("element vertex") {
            vertex_count = parse_element_count(line, "vertex")?;
            in_vertex_element = true;
            continue;
        }
        if line.starts_with("element face") {
            face_count = parse_element_count(line, "face")?;
            in_vertex_element = false;
            continue;
        }
        if line.starts_with("element ") {
            // Other element (e.g., edge) — skip
            in_vertex_element = false;
            continue;
        }
        if line.starts_with("property ") && in_vertex_element {
            // Skip "property list" lines (face vertex indices etc.)
            if line.starts_with("property list") {
                continue;
            }
            // property <type> <name>
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let Some(pt) = parse_property_type(parts[1]) {
                    vertex_properties.push(PlyProperty {
                        name: parts[2].to_string(),
                        type_: pt,
                    });
                }
            }
        }
    }

    // Data offset is everything up to and including "end_header\n"
    let data_offset = header_end + "end_header".len();
    // Skip the newline after end_header
    let data_offset = if data_offset < bytes.len() && bytes[data_offset] == b'\n' {
        data_offset + 1
    } else if data_offset + 1 < bytes.len() && bytes[data_offset] == b'\r' && bytes[data_offset + 1] == b'\n' {
        data_offset + 2
    } else {
        data_offset
    };

    Ok(PlyHeader {
        format,
        vertex_count,
        face_count,
        vertex_properties,
        data_offset,
    })
}

fn find_end_header(bytes: &[u8]) -> Option<usize> {
    let needle = b"end_header";
    for i in 0..=bytes.len().saturating_sub(needle.len()) {
        if &bytes[i..i + needle.len()] == needle {
            return Some(i);
        }
    }
    None
}

fn parse_element_count(line: &str, element_name: &str) -> Result<u32, String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(format!("Invalid element line: {}", line));
    }
    parts[2].parse::<u32>().map_err(|e| format!("Invalid {} count: {}", element_name, e))
}

/// Find property index by name (case-insensitive).
fn find_property(props: &[PlyProperty], name: &str) -> Option<usize> {
    props.iter().position(|p| p.name.eq_ignore_ascii_case(name))
}

// ===========================================================================
// ASCII PLY Parsing
// ===========================================================================

fn parse_ply_ascii(data: &str, header: &PlyHeader) -> Result<PlyResult, String> {
    let x_idx = find_property(&header.vertex_properties, "x");
    let y_idx = find_property(&header.vertex_properties, "y");
    let z_idx = find_property(&header.vertex_properties, "z");
    let r_idx = find_property(&header.vertex_properties, "red");
    let g_idx = find_property(&header.vertex_properties, "green");
    let b_idx = find_property(&header.vertex_properties, "blue");
    let nx_idx = find_property(&header.vertex_properties, "nx");
    let ny_idx = find_property(&header.vertex_properties, "ny");
    let nz_idx = find_property(&header.vertex_properties, "nz");

    if x_idx.is_none() || y_idx.is_none() || z_idx.is_none() {
        return Err("PLY: vertex element missing x/y/z properties".to_string());
    }

    let has_colors = r_idx.is_some() && g_idx.is_some() && b_idx.is_some();
    let has_normals = nx_idx.is_some() && ny_idx.is_some() && nz_idx.is_some();

    let mut positions: Vec<f32> = Vec::with_capacity(header.vertex_count as usize * 3);
    let mut colors: Option<Vec<u8>> = if has_colors {
        Some(Vec::with_capacity(header.vertex_count as usize * 3))
    } else {
        None
    };
    let mut normals: Option<Vec<f32>> = if has_normals {
        Some(Vec::with_capacity(header.vertex_count as usize * 3))
    } else {
        None
    };

    let mut vertex_lines_parsed = 0u32;
    let prop_count = header.vertex_properties.len();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Split by whitespace
        let values: Vec<&str> = line.split_whitespace().collect();

        // For faces: first value is the number of vertices in the face
        // We count faces but don't store them
        if vertex_lines_parsed >= header.vertex_count {
            // We're past vertex data — faces or other elements
            if values.len() >= 1 {
                if let Ok(n) = values[0].parse::<u32>() {
                    // This is a face with n vertices — count it
                }
            }
            continue;
        }

        // Parse vertex line
        if values.len() < prop_count {
            // Skip incomplete lines
            continue;
        }

        // Extract x, y, z
        let x: f32 = values[x_idx.unwrap()].parse().unwrap_or(0.0);
        let y: f32 = values[y_idx.unwrap()].parse().unwrap_or(0.0);
        let z: f32 = values[z_idx.unwrap()].parse().unwrap_or(0.0);
        positions.push(x);
        positions.push(y);
        positions.push(z);

        // Extract colors
        if has_colors {
            let r: u8 = values[r_idx.unwrap()].parse().unwrap_or(0);
            let g: u8 = values[g_idx.unwrap()].parse().unwrap_or(0);
            let b: u8 = values[b_idx.unwrap()].parse().unwrap_or(0);
            if let Some(ref mut c) = colors {
                c.push(r);
                c.push(g);
                c.push(b);
            }
        }

        // Extract normals
        if has_normals {
            let nx: f32 = values[nx_idx.unwrap()].parse().unwrap_or(0.0);
            let ny: f32 = values[ny_idx.unwrap()].parse().unwrap_or(0.0);
            let nz: f32 = values[nz_idx.unwrap()].parse().unwrap_or(0.0);
            if let Some(ref mut n) = normals {
                n.push(nx);
                n.push(ny);
                n.push(nz);
            }
        }

        vertex_lines_parsed += 1;
    }

    Ok(PlyResult {
        positions,
        colors,
        normals,
        vertex_count: vertex_lines_parsed,
        face_count: header.face_count,
    })
}

// ===========================================================================
// Binary PLY Parsing (little-endian)
// ===========================================================================

fn parse_ply_binary_le(bytes: &[u8], header: &PlyHeader) -> Result<PlyResult, String> {
    let data = &bytes[header.data_offset..];

    let x_idx = find_property(&header.vertex_properties, "x");
    let y_idx = find_property(&header.vertex_properties, "y");
    let z_idx = find_property(&header.vertex_properties, "z");
    let r_idx = find_property(&header.vertex_properties, "red");
    let g_idx = find_property(&header.vertex_properties, "green");
    let b_idx = find_property(&header.vertex_properties, "blue");
    let nx_idx = find_property(&header.vertex_properties, "nx");
    let ny_idx = find_property(&header.vertex_properties, "ny");
    let nz_idx = find_property(&header.vertex_properties, "nz");

    if x_idx.is_none() || y_idx.is_none() || z_idx.is_none() {
        return Err("PLY: vertex element missing x/y/z properties".to_string());
    }

    let has_colors = r_idx.is_some() && g_idx.is_some() && b_idx.is_some();
    let has_normals = nx_idx.is_some() && ny_idx.is_some() && nz_idx.is_some();

    let mut positions: Vec<f32> = Vec::with_capacity(header.vertex_count as usize * 3);
    let mut colors: Option<Vec<u8>> = if has_colors {
        Some(Vec::with_capacity(header.vertex_count as usize * 3))
    } else {
        None
    };
    let mut normals: Option<Vec<f32>> = if has_normals {
        Some(Vec::with_capacity(header.vertex_count as usize * 3))
    } else {
        None
    };

    // Calculate per-vertex byte size
    let vertex_size: usize = header.vertex_properties.iter().map(|p| type_size(&p.type_)).sum();
    let vertex_data_end = header.vertex_count as usize * vertex_size;

    if data.len() < vertex_data_end {
        return Err(format!(
            "PLY binary: expected {} bytes for {} vertices, got {}",
            vertex_data_end,
            header.vertex_count,
            data.len()
        ));
    }

    let mut offset = 0usize;
    for _ in 0..header.vertex_count {
        let vertex_base = offset;

        // Read x
        let x = read_float_at(data, vertex_base, &header.vertex_properties[x_idx.unwrap()].type_);
        let y = read_float_at(data, vertex_base + property_byte_offset(&header.vertex_properties, y_idx.unwrap()), &header.vertex_properties[y_idx.unwrap()].type_);
        let z = read_float_at(data, vertex_base + property_byte_offset(&header.vertex_properties, z_idx.unwrap()), &header.vertex_properties[z_idx.unwrap()].type_);

        positions.push(x);
        positions.push(y);
        positions.push(z);

        if has_colors {
            let r = read_u8_at(data, vertex_base + property_byte_offset(&header.vertex_properties, r_idx.unwrap()));
            let g = read_u8_at(data, vertex_base + property_byte_offset(&header.vertex_properties, g_idx.unwrap()));
            let b = read_u8_at(data, vertex_base + property_byte_offset(&header.vertex_properties, b_idx.unwrap()));
            if let Some(ref mut c) = colors {
                c.push(r);
                c.push(g);
                c.push(b);
            }
        }

        if has_normals {
            let nx = read_float_at(data, vertex_base + property_byte_offset(&header.vertex_properties, nx_idx.unwrap()), &header.vertex_properties[nx_idx.unwrap()].type_);
            let ny = read_float_at(data, vertex_base + property_byte_offset(&header.vertex_properties, ny_idx.unwrap()), &header.vertex_properties[ny_idx.unwrap()].type_);
            let nz = read_float_at(data, vertex_base + property_byte_offset(&header.vertex_properties, nz_idx.unwrap()), &header.vertex_properties[nz_idx.unwrap()].type_);
            if let Some(ref mut n) = normals {
                n.push(nx);
                n.push(ny);
                n.push(nz);
            }
        }

        offset += vertex_size;
    }

    Ok(PlyResult {
        positions,
        colors,
        normals,
        vertex_count: header.vertex_count,
        face_count: header.face_count,
    })
}

fn property_byte_offset(properties: &[PlyProperty], index: usize) -> usize {
    properties[..index].iter().map(|p| type_size(&p.type_)).sum()
}

fn read_float_at(data: &[u8], base: usize, type_: &PlyPropertyType) -> f32 {
    match type_ {
        PlyPropertyType::Float => {
            if base + 4 <= data.len() {
                f32::from_le_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]])
            } else {
                0.0
            }
        }
        PlyPropertyType::Double => {
            if base + 8 <= data.len() {
                f64::from_le_bytes([
                    data[base], data[base + 1], data[base + 2], data[base + 3],
                    data[base + 4], data[base + 5], data[base + 6], data[base + 7],
                ]) as f32
            } else {
                0.0
            }
        }
        PlyPropertyType::Int8 => {
            if base < data.len() {
                data[base] as i8 as f32
            } else {
                0.0
            }
        }
        PlyPropertyType::Uint8 => {
            if base < data.len() {
                data[base] as f32
            } else {
                0.0
            }
        }
        PlyPropertyType::Int16 => {
            if base + 2 <= data.len() {
                i16::from_le_bytes([data[base], data[base + 1]]) as f32
            } else {
                0.0
            }
        }
        PlyPropertyType::Uint16 => {
            if base + 2 <= data.len() {
                u16::from_le_bytes([data[base], data[base + 1]]) as f32
            } else {
                0.0
            }
        }
        PlyPropertyType::Int32 => {
            if base + 4 <= data.len() {
                i32::from_le_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]) as f32
            } else {
                0.0
            }
        }
        PlyPropertyType::Uint32 => {
            if base + 4 <= data.len() {
                u32::from_le_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]) as f32
            } else {
                0.0
            }
        }
    }
}

fn read_u8_at(data: &[u8], offset: usize) -> u8 {
    if offset < data.len() {
        data[offset]
    } else {
        0
    }
}

// ===========================================================================
// Core Parser (no WASM dependency, testable everywhere)
// ===========================================================================

/// Core PLY parser — works on native and WASM.
pub fn parse_ply_core(bytes: &[u8]) -> Result<PlyResult, String> {
    if bytes.len() < 4 {
        return Err("Data too short to be PLY".to_string());
    }
    if &bytes[0..3] != b"ply" {
        return Err(format!("Not a PLY file: magic={:?}", &bytes[0..4]));
    }

    let header = parse_ply_header(bytes)?;

    match header.format {
        PlyFormat::Ascii => {
            let data_str = String::from_utf8(bytes[header.data_offset..].to_vec())
                .map_err(|e| format!("Invalid UTF-8 in PLY ASCII data: {}", e))?;
            parse_ply_ascii(&data_str, &header)
        }
        PlyFormat::BinaryLittleEndian => parse_ply_binary_le(bytes, &header),
        PlyFormat::BinaryBigEndian => Err("Binary Big Endian PLY is not supported".to_string()),
    }
}

// ===========================================================================
// WASM API
// ===========================================================================

/// Parse a PLY (Polygon File Format) file.
///
/// Supports ASCII and binary_little_endian formats.
/// Returns a `PlyResult` with vertex positions, optional colors, optional normals.
///
/// # Example (JS)
/// ```js
/// const result = core.parsePly(arrayBuffer);
/// const positions = result.positions();
/// const colors = result.colors();
/// const vertexCount = result.vertexCount;
/// ```
#[wasm_bindgen(js_name = "parsePly")]
pub fn parse_ply(bytes: &[u8]) -> Result<PlyResult, SpatialErrorDetail> {
    if bytes.len() > DEFAULT_MAX_INPUT_SIZE {
        return Err(SpatialError::InputTooLarge.with_detail(format!(
            "PLY input is {} bytes, max is {}",
            bytes.len(),
            DEFAULT_MAX_INPUT_SIZE
        )));
    }
    parse_ply_core(bytes).map_err(|e| {
        SpatialError::point_cloud_error(&e)
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ascii_ply(positions: &[(f32, f32, f32)], colors: Option<&[(u8, u8, u8)]>, normals: Option<&[(f32, f32, f32)]>) -> Vec<u8> {
        let mut header = String::from("ply\nformat ascii 1.0\n");
        header.push_str(&format!("element vertex {}\n", positions.len()));
        header.push_str("property float x\nproperty float y\nproperty float z\n");
        if colors.is_some() {
            header.push_str("property uchar red\nproperty uchar green\nproperty uchar blue\n");
        }
        if normals.is_some() {
            header.push_str("property float nx\nproperty float ny\nproperty float nz\n");
        }
        header.push_str("end_header\n");

        let mut data = header.into_bytes();
        for (i, &(x, y, z)) in positions.iter().enumerate() {
            data.extend_from_slice(format!("{} {} {}", x, y, z).as_bytes());
            if let Some(cols) = colors {
                let (r, g, b) = cols[i];
                data.extend_from_slice(format!(" {} {} {}", r, g, b).as_bytes());
            }
            if let Some(nors) = normals {
                let (nx, ny, nz) = nors[i];
                data.extend_from_slice(format!(" {} {} {}", nx, ny, nz).as_bytes());
            }
            data.push(b'\n');
        }
        data
    }

    fn make_binary_ply(positions: &[(f32, f32, f32)], colors: Option<&[(u8, u8, u8)]>, normals: Option<&[(f32, f32, f32)]>) -> Vec<u8> {
        let has_colors = colors.is_some();
        let has_normals = normals.is_some();

        let mut header = String::from("ply\nformat binary_little_endian 1.0\n");
        header.push_str(&format!("element vertex {}\n", positions.len()));
        header.push_str("property float x\nproperty float y\nproperty float z\n");
        if has_colors {
            header.push_str("property uchar red\nproperty uchar green\nproperty uchar blue\n");
        }
        if has_normals {
            header.push_str("property float nx\nproperty float ny\nproperty float nz\n");
        }
        header.push_str("end_header\n");

        let mut data = header.into_bytes();
        for (i, &(x, y, z)) in positions.iter().enumerate() {
            data.extend_from_slice(&x.to_le_bytes());
            data.extend_from_slice(&y.to_le_bytes());
            data.extend_from_slice(&z.to_le_bytes());
            if let Some(cols) = colors {
                let (r, g, b) = cols[i];
                data.push(r);
                data.push(g);
                data.push(b);
            }
            if let Some(nors) = normals {
                let (nx, ny, nz) = nors[i];
                data.extend_from_slice(&nx.to_le_bytes());
                data.extend_from_slice(&ny.to_le_bytes());
                data.extend_from_slice(&nz.to_le_bytes());
            }
        }
        data
    }

    // ── ASCII tests ───────────────────────────────────────────────

    #[test]
    fn test_ascii_simple_positions() {
        let pts = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0), (7.0, 8.0, 9.0)];
        let data = make_ascii_ply(&pts, None, None);
        let result = parse_ply_core(&data).unwrap();

        assert_eq!(result.vertex_count(), 3);
        assert!(!result.has_colors());
        assert!(!result.has_normals());
        assert_eq!(result.face_count(), 0);
        assert_eq!(result.positions.len(), 9);
        assert_eq!(result.positions[0], 1.0);
        assert_eq!(result.positions[4], 5.0);
        assert_eq!(result.positions[8], 9.0);
    }

    #[test]
    fn test_ascii_with_colors() {
        let pts = vec![(1.0, 2.0, 3.0), (10.0, 20.0, 30.0)];
        let cols = vec![(255, 0, 0), (0, 128, 255)];
        let data = make_ascii_ply(&pts, Some(&cols), None);
        let result = parse_ply_core(&data).unwrap();

        assert_eq!(result.vertex_count(), 2);
        assert!(result.has_colors());
        let c = result.colors.as_ref().unwrap();
        assert_eq!(c[0], 255);
        assert_eq!(c[1], 0);
        assert_eq!(c[2], 0);
        assert_eq!(c[3], 0);
        assert_eq!(c[4], 128);
        assert_eq!(c[5], 255);
    }

    #[test]
    fn test_ascii_with_normals() {
        let pts = vec![(0.0, 0.0, 1.0), (1.0, 0.0, 0.0)];
        let nors = vec![(0.0, 0.0, 1.0), (1.0, 0.0, 0.0)];
        let data = make_ascii_ply(&pts, None, Some(&nors));
        let result = parse_ply_core(&data).unwrap();

        assert!(result.has_normals());
        let n = result.normals.as_ref().unwrap();
        assert_eq!(n[0], 0.0);
        assert_eq!(n[2], 1.0);
        assert_eq!(n[3], 1.0);
        assert_eq!(n[4], 0.0);
    }

    // ── Binary tests ───────────────────────────────────────────────

    #[test]
    fn test_binary_simple_positions() {
        let pts = vec![(1.5, 2.5, 3.5), (-1.0, -2.0, -3.0)];
        let data = make_binary_ply(&pts, None, None);
        let result = parse_ply_core(&data).unwrap();

        assert_eq!(result.vertex_count(), 2);
        assert!(!result.has_colors());
        assert_eq!(result.positions[0], 1.5);
        assert_eq!(result.positions[1], 2.5);
        assert_eq!(result.positions[2], 3.5);
        assert_eq!(result.positions[3], -1.0);
    }

    #[test]
    fn test_binary_with_colors() {
        let pts = vec![(10.0, 20.0, 30.0)];
        let cols = vec![(128, 64, 32)];
        let data = make_binary_ply(&pts, Some(&cols), None);
        let result = parse_ply_core(&data).unwrap();

        assert!(result.has_colors());
        let c = result.colors.as_ref().unwrap();
        assert_eq!(c[0], 128);
        assert_eq!(c[1], 64);
        assert_eq!(c[2], 32);
    }

    #[test]
    fn test_binary_with_normals() {
        let pts = vec![(0.0, 1.0, 0.0)];
        let nors = vec![(0.0, 1.0, 0.0)];
        let data = make_binary_ply(&pts, None, Some(&nors));
        let result = parse_ply_core(&data).unwrap();

        assert!(result.has_normals());
        let n = result.normals.as_ref().unwrap();
        assert_eq!(n[0], 0.0);
        assert_eq!(n[1], 1.0);
        assert_eq!(n[2], 0.0);
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn test_rejects_non_ply() {
        let data = b"not a ply file";
        let result = parse_ply_core(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_too_short() {
        let result = parse_ply_core(&[0u8; 2]);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_ply() {
        let data = b"ply\nformat ascii 1.0\nelement vertex 0\nproperty float x\nproperty float y\nproperty float z\nend_header\n";
        let result = parse_ply_core(data).unwrap();
        assert_eq!(result.vertex_count(), 0);
        assert_eq!(result.positions.len(), 0);
    }

    #[test]
    fn test_ply_with_face_count() {
        // Build a PLY with faces
        let mut data = String::from("ply\nformat ascii 1.0\n");
        data.push_str("element vertex 3\n");
        data.push_str("property float x\nproperty float y\nproperty float z\n");
        data.push_str("element face 1\n");
        data.push_str("property list uchar int vertex_indices\n");
        data.push_str("end_header\n");
        data.push_str("0 0 0\n1 0 0\n0 1 0\n");
        data.push_str("3 0 1 2\n");

        let result = parse_ply_core(data.as_bytes()).unwrap();
        assert_eq!(result.vertex_count(), 3);
        assert_eq!(result.face_count(), 1);
    }

    #[test]
    fn test_binary_many_vertices() {
        let pts: Vec<(f32, f32, f32)> = (0..1000)
            .map(|i| (i as f32 * 0.1, (i as f32 * 0.2).sin(), (i as f32 * 0.3).cos()))
            .collect();
        let data = make_binary_ply(&pts, None, None);
        let result = parse_ply_core(&data).unwrap();
        assert_eq!(result.vertex_count(), 1000);
        assert_eq!(result.positions.len(), 3000);
    }
}
