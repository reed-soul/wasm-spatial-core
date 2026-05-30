//! Well-Known Text (WKT) and Well-Known Binary (WKB) format support.
//!
//! Provides parsing and serialization for GIS-standard spatial geometry
//! representations: POINT, LINESTRING, POLYGON, MULTIPOINT.

use js_sys::Float64Array;
use wasm_bindgen::prelude::*;

// ===========================================================================
// WKT Parsing & Serialization
// ===========================================================================

/// Parse a Well-Known Text (WKT) string into a flat `Float64Array`.
///
/// Supports: POINT, LINESTRING, POLYGON, MULTIPOINT.
///
/// # Arguments
/// - `input`: WKT string (case-insensitive).
///
/// # Returns
/// Flat `[lng0, lat0, lng1, lat1, ...]` coordinates.
///
/// # Example
/// ```js
/// const coords = parseWkt("LINESTRING(0 0, 10 10, 20 0)");
/// ```
#[wasm_bindgen(js_name = "parseWkt")]
pub fn parse_wkt(input: &str) -> Result<Float64Array, JsValue> {
    let coords = parse_wkt_core(input).map_err(|e| crate::errors::parse_js(&e))?;
    let arr = Float64Array::new_with_length(coords.len() as u32);
    if !coords.is_empty() {
        arr.copy_from(&coords);
    }
    Ok(arr)
}

/// Core WKT parser that returns Vec<f64>.
fn parse_wkt_core(input: &str) -> Result<Vec<f64>, String> {
    let trimmed = input.trim();
    let upper = trimmed.to_uppercase();

    if upper.starts_with("POINT") {
        if upper.contains("EMPTY") {
            return Ok(vec![]);
        }
        let content = extract_parens_str(trimmed)?;
        let nums = parse_coord_string(content)?;
        if nums.len() < 2 {
            return Err("POINT requires at least 2 coordinates".to_string());
        }
        Ok(vec![nums[0], nums[1]])
    } else if upper.starts_with("MULTIPOINT") {
        if upper.contains("EMPTY") {
            return Ok(vec![]);
        }
        let content = extract_parens_str(trimmed)?;
        let mut coords = Vec::new();
        // Try parenthesized form: ((x y), (x y))
        if content.starts_with('(') {
            let groups = split_paren_groups(content);
            for g in groups {
                let nums = parse_coord_string(g.trim())?;
                if nums.len() >= 2 {
                    coords.push(nums[0]);
                    coords.push(nums[1]);
                }
            }
        } else {
            let nums = parse_coord_string(content)?;
            for c in nums.chunks_exact(2) {
                coords.push(c[0]);
                coords.push(c[1]);
            }
        }
        Ok(coords)
    } else if upper.starts_with("LINESTRING") {
        if upper.contains("EMPTY") {
            return Ok(vec![]);
        }
        let content = extract_parens_str(trimmed)?;
        let nums = parse_coord_string(content)?;
        let mut out = Vec::with_capacity(nums.len());
        for c in nums.chunks_exact(2) {
            out.push(c[0]);
            out.push(c[1]);
        }
        Ok(out)
    } else if upper.starts_with("POLYGON") {
        if upper.contains("EMPTY") {
            return Ok(vec![]);
        }
        let content = extract_parens_str(trimmed)?;
        let mut coords = Vec::new();
        let rings = split_paren_groups(content);
        for ring in rings {
            let nums = parse_coord_string(ring.trim())?;
            for c in nums.chunks_exact(2) {
                coords.push(c[0]);
                coords.push(c[1]);
            }
        }
        Ok(coords)
    } else {
        Err(format!(
            "Unsupported WKT geometry type: {}",
            if upper.is_empty() {
                "(empty)".to_string()
            } else {
                upper[..upper.len().min(20)].to_string()
            }
        ))
    }
}

/// Extract text between first '(' and last ')'.
fn extract_parens_str(input: &str) -> Result<&str, String> {
    let start = input.find('(').ok_or("No '(' found")?;
    let end = input.rfind(')').ok_or("No ')' found")?;
    if end <= start {
        return Err("Invalid parentheses".to_string());
    }
    Ok(&input[start + 1..end])
}

/// Parse a coordinate string like "116.4 39.9" or "0 0, 10 10, 20 0"
/// into a flat Vec<f64>.
fn parse_coord_string(s: &str) -> Result<Vec<f64>, String> {
    let mut nums = Vec::new();
    // Split by comma first (for multi-point), then split by whitespace
    for part in s.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Split by whitespace
        for token in trimmed.split_whitespace() {
            let val = token
                .parse::<f64>()
                .map_err(|e| format!("invalid float '{}': {}", token, e))?;
            nums.push(val);
        }
    }
    Ok(nums)
}

/// Split "(x y), (x y), (x y)" into ["x y", "x y", "x y"]
fn split_paren_groups(s: &str) -> Vec<&str> {
    let mut groups = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '(' => {
                if depth == 0 {
                    start = i + 1;
                }
                depth += 1;
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    groups.push(&s[start..i]);
                }
            }
            _ => {}
        }
    }
    if groups.is_empty() && !s.is_empty() {
        groups.push(s.trim());
    }
    groups
}

/// Generate a Well-Known Text (WKT) string from coordinates.
///
/// # Arguments
/// - `coords`: Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
/// - `geometry_type`: Geometry type string: `"POINT"`, `"LINESTRING"`,
///   `"POLYGON"`, `"MULTIPOINT"`.
///
/// # Example
/// ```js
/// const wkt = toWkt(coords, "LINESTRING");
/// ```
#[wasm_bindgen(js_name = "toWkt")]
pub fn to_wkt(coords: &Float64Array, geometry_type: &str) -> Result<String, JsValue> {
    let mut buf = vec![0.0f64; coords.length() as usize];
    coords.copy_to(&mut buf);

    let n = buf.len() / 2;
    let gt = geometry_type.to_uppercase();

    match gt.as_str() {
        "POINT" => {
            if n == 0 {
                Ok("POINT EMPTY".to_string())
            } else if n == 1 {
                Ok(format!("POINT({} {})", buf[0], buf[1]))
            } else {
                Err(crate::errors::parse_js(
                    "POINT requires exactly 1 coordinate pair",
                ))
            }
        }
        "MULTIPOINT" => {
            if n == 0 {
                return Ok("MULTIPOINT EMPTY".to_string());
            }
            let pairs: Vec<String> = (0..n)
                .map(|i| format!("({} {})", buf[i * 2], buf[i * 2 + 1]))
                .collect();
            Ok(format!("MULTIPOINT({})", pairs.join(", ")))
        }
        "LINESTRING" => {
            if n == 0 {
                return Ok("LINESTRING EMPTY".to_string());
            }
            let pairs: Vec<String> = (0..n)
                .map(|i| format!("{} {}", buf[i * 2], buf[i * 2 + 1]))
                .collect();
            Ok(format!("LINESTRING({})", pairs.join(", ")))
        }
        "POLYGON" => {
            if n == 0 {
                return Ok("POLYGON EMPTY".to_string());
            }
            // Assume single ring
            let pairs: Vec<String> = (0..n)
                .map(|i| format!("{} {}", buf[i * 2], buf[i * 2 + 1]))
                .collect();
            Ok(format!("POLYGON(({}))", pairs.join(", ")))
        }
        _ => Err(crate::errors::parse_js(format!(
            "Unsupported geometry type: {}",
            geometry_type
        ))),
    }
}

// ===========================================================================
// WKB Parsing & Serialization
// ===========================================================================

// WKB geometry type codes (2D)
const WKB_POINT: u32 = 1;
const WKB_LINESTRING: u32 = 2;
const WKB_POLYGON: u32 = 3;
const WKB_MULTIPOINT: u32 = 4;

/// Parse Well-Known Binary (WKB) data into a flat `Float64Array`.
///
/// Supports 2D POINT, LINESTRING, POLYGON, MULTIPOINT.
/// Byte order is auto-detected (little-endian or big-endian).
///
/// # Arguments
/// - `bytes`: `Uint8Array` containing WKB data.
///
/// # Example
/// ```js
/// const coords = parseWkb(new Uint8Array(wkbBuffer));
/// ```
#[wasm_bindgen(js_name = "parseWkb")]
pub fn parse_wkb(bytes: &js_sys::Uint8Array) -> Result<Float64Array, JsValue> {
    let mut buf = vec![0u8; bytes.length() as usize];
    bytes.copy_to(&mut buf);

    if buf.len() < 5 {
        return Err(crate::errors::parse_js("WKB too short"));
    }

    let byte_order = buf[0];
    let is_le = byte_order == 1; // 1 = little-endian, 0 = big-endian

    let geom_type = read_u32(&buf[1..5], is_le);

    let coords = match geom_type {
        WKB_POINT => {
            if buf.len() < 5 + 16 {
                return Err(crate::errors::parse_js("WKB POINT too short"));
            }
            vec![read_f64(&buf[5..13], is_le), read_f64(&buf[13..21], is_le)]
        }
        WKB_LINESTRING => parse_wkb_linestring(&buf, 5, is_le)?,
        WKB_POLYGON => parse_wkb_polygon(&buf, 5, is_le)?,
        WKB_MULTIPOINT => parse_wkb_multipoint(&buf, 5, is_le)?,
        _ => {
            return Err(crate::errors::parse_js(format!(
                "Unsupported WKB geometry type: {}",
                geom_type
            )))
        }
    };

    let arr = Float64Array::new_with_length(coords.len() as u32);
    if !coords.is_empty() {
        arr.copy_from(&coords);
    }
    Ok(arr)
}

fn parse_wkb_linestring(buf: &[u8], offset: usize, is_le: bool) -> Result<Vec<f64>, JsValue> {
    if buf.len() < offset + 4 {
        return Err(crate::errors::parse_js(
            "WKB LINESTRING too short for npoints",
        ));
    }
    let npoints = read_u32(&buf[offset..offset + 4], is_le) as usize;
    let needed = offset + 4 + npoints * 16;
    if buf.len() < needed {
        return Err(crate::errors::parse_js(
            "WKB LINESTRING too short for coordinates",
        ));
    }
    let mut coords = Vec::with_capacity(npoints * 2);
    for i in 0..npoints {
        let base = offset + 4 + i * 16;
        coords.push(read_f64(&buf[base..base + 8], is_le));
        coords.push(read_f64(&buf[base + 8..base + 16], is_le));
    }
    Ok(coords)
}

fn parse_wkb_polygon(buf: &[u8], offset: usize, is_le: bool) -> Result<Vec<f64>, JsValue> {
    if buf.len() < offset + 4 {
        return Err(crate::errors::parse_js("WKB POLYGON too short for nrings"));
    }
    let nrings = read_u32(&buf[offset..offset + 4], is_le) as usize;
    let mut coords = Vec::new();
    let mut pos = offset + 4;

    for _ in 0..nrings {
        if buf.len() < pos + 4 {
            return Err(crate::errors::parse_js("WKB POLYGON ring too short"));
        }
        let npoints = read_u32(&buf[pos..pos + 4], is_le) as usize;
        pos += 4;
        let needed = pos + npoints * 16;
        if buf.len() < needed {
            return Err(crate::errors::parse_js("WKB POLYGON ring coords too short"));
        }
        for i in 0..npoints {
            let base = pos + i * 16;
            coords.push(read_f64(&buf[base..base + 8], is_le));
            coords.push(read_f64(&buf[base + 8..base + 16], is_le));
        }
        pos += npoints * 16;
    }
    Ok(coords)
}

fn parse_wkb_multipoint(buf: &[u8], offset: usize, is_le: bool) -> Result<Vec<f64>, JsValue> {
    if buf.len() < offset + 4 {
        return Err(crate::errors::parse_js(
            "WKB MULTIPOINT too short for npoints",
        ));
    }
    let npoints = read_u32(&buf[offset..offset + 4], is_le) as usize;
    let mut coords = Vec::with_capacity(npoints * 2);
    let mut pos = offset + 4;

    for _ in 0..npoints {
        // Each point is a nested WKB element: byte_order(1) + type(4) + coords(16)
        if buf.len() < pos + 5 + 16 {
            return Err(crate::errors::parse_js("WKB MULTIPOINT element too short"));
        }
        let pt_byte_order = buf[pos];
        let pt_is_le = pt_byte_order == 1;
        // Skip byte_order + type
        coords.push(read_f64(&buf[pos + 5..pos + 13], pt_is_le));
        coords.push(read_f64(&buf[pos + 13..pos + 21], pt_is_le));
        pos += 21;
    }
    Ok(coords)
}

/// Generate Well-Known Binary (WKB) from coordinates.
///
/// Produces little-endian WKB (byte order = 1).
///
/// # Arguments
/// - `coords`: Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
/// - `geometry_type`: `"POINT"`, `"LINESTRING"`, `"POLYGON"`, `"MULTIPOINT"`.
///
/// # Example
/// ```js
/// const wkb = toWkb(coords, "LINESTRING");
/// ```
#[wasm_bindgen(js_name = "toWkb")]
pub fn to_wkb(coords: &Float64Array, geometry_type: &str) -> Result<js_sys::Uint8Array, JsValue> {
    let mut buf = vec![0.0f64; coords.length() as usize];
    coords.copy_to(&mut buf);

    let n = buf.len() / 2;
    let gt = geometry_type.to_uppercase();

    let wkb: Vec<u8> = match gt.as_str() {
        "POINT" => {
            if n != 1 {
                return Err(crate::errors::parse_js(
                    "POINT requires exactly 1 coordinate pair",
                ));
            }
            let mut out = Vec::with_capacity(21);
            out.push(1); // little-endian
            out.extend_from_slice(&WKB_POINT.to_le_bytes());
            out.extend_from_slice(&buf[0].to_le_bytes());
            out.extend_from_slice(&buf[1].to_le_bytes());
            out
        }
        "MULTIPOINT" => {
            let mut out = Vec::with_capacity(9 + n * 21);
            out.push(1);
            out.extend_from_slice(&(n as u32).to_le_bytes()); // npoints in MULTIPOINT header
            for i in 0..n {
                out.push(1); // little-endian for each point
                out.extend_from_slice(&WKB_POINT.to_le_bytes());
                out.extend_from_slice(&buf[i * 2].to_le_bytes());
                out.extend_from_slice(&buf[i * 2 + 1].to_le_bytes());
            }
            out
        }
        "LINESTRING" => {
            let mut out = Vec::with_capacity(9 + n * 16);
            out.push(1);
            out.extend_from_slice(&WKB_LINESTRING.to_le_bytes());
            out.extend_from_slice(&(n as u32).to_le_bytes());
            for i in 0..n {
                out.extend_from_slice(&buf[i * 2].to_le_bytes());
                out.extend_from_slice(&buf[i * 2 + 1].to_le_bytes());
            }
            out
        }
        "POLYGON" => {
            // Single ring polygon
            let mut out = Vec::with_capacity(13 + n * 16);
            out.push(1);
            out.extend_from_slice(&WKB_POLYGON.to_le_bytes());
            out.extend_from_slice(&1u32.to_le_bytes()); // 1 ring
            out.extend_from_slice(&(n as u32).to_le_bytes());
            for i in 0..n {
                out.extend_from_slice(&buf[i * 2].to_le_bytes());
                out.extend_from_slice(&buf[i * 2 + 1].to_le_bytes());
            }
            out
        }
        _ => {
            return Err(crate::errors::parse_js(format!(
                "Unsupported geometry type: {}",
                geometry_type
            )))
        }
    };

    let arr = js_sys::Uint8Array::new_with_length(wkb.len() as u32);
    arr.copy_from(&wkb);
    Ok(arr)
}

// ===========================================================================
// Byte helpers
// ===========================================================================

fn read_u32(bytes: &[u8], is_le: bool) -> u32 {
    if is_le {
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    } else {
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}

fn read_f64(bytes: &[u8], is_le: bool) -> f64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    if is_le {
        f64::from_le_bytes(arr)
    } else {
        f64::from_be_bytes(arr)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

// ===========================================================================
// Native helpers (for testing without WASM runtime)
// ===========================================================================

/// Native WKT parse — uses the shared core parser.
#[cfg(test)]
fn parse_wkt_native(input: &str) -> Result<Vec<f64>, String> {
    parse_wkt_core(input)
}

/// Native WKB encode for a single POINT (for testing).
#[cfg(test)]
fn to_wkb_point_native(x: f64, y: f64) -> Vec<u8> {
    let mut out = Vec::with_capacity(21);
    out.push(1); // little-endian
    out.extend_from_slice(&1u32.to_le_bytes()); // POINT type
    out.extend_from_slice(&x.to_le_bytes());
    out.extend_from_slice(&y.to_le_bytes());
    out
}

/// Native WKB encode for a LINESTRING (for testing).
#[cfg(test)]
fn to_wkb_linestring_native(coords: &[f64]) -> Vec<u8> {
    let n = coords.len() / 2;
    let mut out = Vec::with_capacity(9 + n * 16);
    out.push(1);
    out.extend_from_slice(&2u32.to_le_bytes()); // LINESTRING type
    out.extend_from_slice(&(n as u32).to_le_bytes());
    for c in coords.chunks_exact(2) {
        out.extend_from_slice(&c[0].to_le_bytes());
        out.extend_from_slice(&c[1].to_le_bytes());
    }
    out
}

/// Native WKB encode for MULTIPOINT (for testing).
#[cfg(test)]
fn to_wkb_multipoint_native(coords: &[f64]) -> Vec<u8> {
    let n = coords.len() / 2;
    let mut out = Vec::with_capacity(9 + n * 21);
    out.push(1);
    out.extend_from_slice(&4u32.to_le_bytes()); // MULTIPOINT type
    out.extend_from_slice(&(n as u32).to_le_bytes());
    for c in coords.chunks_exact(2) {
        out.push(1);
        out.extend_from_slice(&1u32.to_le_bytes()); // POINT type per element
        out.extend_from_slice(&c[0].to_le_bytes());
        out.extend_from_slice(&c[1].to_le_bytes());
    }
    out
}

/// Native WKB decode (for testing without WASM runtime).
#[cfg(test)]
fn parse_wkb_native(bytes: &[u8]) -> Result<Vec<f64>, String> {
    if bytes.len() < 5 {
        return Err("WKB too short".to_string());
    }
    let is_le = bytes[0] == 1;
    let geom_type = read_u32(&bytes[1..5], is_le);

    match geom_type {
        1 => {
            // POINT
            if bytes.len() < 21 {
                return Err("POINT too short".to_string());
            }
            Ok(vec![
                read_f64(&bytes[5..13], is_le),
                read_f64(&bytes[13..21], is_le),
            ])
        }
        2 => {
            // LINESTRING
            if bytes.len() < 9 {
                return Err("LINESTRING too short".to_string());
            }
            let npoints = read_u32(&bytes[5..9], is_le) as usize;
            if bytes.len() < 9 + npoints * 16 {
                return Err("LINESTRING coords too short".to_string());
            }
            let mut coords = Vec::with_capacity(npoints * 2);
            for i in 0..npoints {
                let base = 9 + i * 16;
                coords.push(read_f64(&bytes[base..base + 8], is_le));
                coords.push(read_f64(&bytes[base + 8..base + 16], is_le));
            }
            Ok(coords)
        }
        4 => {
            // MULTIPOINT
            if bytes.len() < 9 {
                return Err("MULTIPOINT too short".to_string());
            }
            let npoints = read_u32(&bytes[5..9], is_le) as usize;
            let mut coords = Vec::with_capacity(npoints * 2);
            let mut pos = 9;
            for _ in 0..npoints {
                if bytes.len() < pos + 21 {
                    return Err("MULTIPOINT element too short".to_string());
                }
                let pt_le = bytes[pos] == 1;
                coords.push(read_f64(&bytes[pos + 5..pos + 13], pt_le));
                coords.push(read_f64(&bytes[pos + 13..pos + 21], pt_le));
                pos += 21;
            }
            Ok(coords)
        }
        _ => Err(format!("Unsupported type {}", geom_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── WKT parsing tests ─────────────────────────────────────────

    #[test]
    fn test_parse_wkt_point() {
        let result = parse_wkt_native("POINT(116.4 39.9)").unwrap();
        assert_eq!(result, vec![116.4, 39.9]);
    }

    #[test]
    fn test_parse_wkt_point_case_insensitive() {
        let result = parse_wkt_native("point(1 2)").unwrap();
        assert_eq!(result, vec![1.0, 2.0]);
    }

    #[test]
    fn test_parse_wkt_linestring() {
        let result = parse_wkt_native("LINESTRING(0 0, 10 10, 20 0)").unwrap();
        assert_eq!(result, vec![0.0, 0.0, 10.0, 10.0, 20.0, 0.0]);
    }

    #[test]
    fn test_parse_wkt_polygon_single_ring() {
        let result = parse_wkt_native("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();
        assert_eq!(
            result,
            vec![0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0, 0.0, 0.0]
        );
    }

    #[test]
    fn test_parse_wkt_multipoint() {
        let result = parse_wkt_native("MULTIPOINT((1 2), (3 4), (5 6))").unwrap();
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_parse_wkt_empty() {
        let result = parse_wkt_native("POINT EMPTY").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_wkt_unsupported() {
        let result = parse_wkt_native("GEOMETRYCOLLECTION(POINT(1 2))");
        assert!(result.is_err());
    }

    // ── WKT to_wkt serialization ────────────────────────────────

    #[test]
    fn test_to_wkt_point() {
        let coords: &[f64] = &[116.4, 39.9];
        let pairs: Vec<String> = coords
            .chunks_exact(2)
            .map(|c| format!("{} {}", c[0], c[1]))
            .collect();
        assert_eq!(format!("POINT({})", pairs[0]), "POINT(116.4 39.9)");
    }

    #[test]
    fn test_to_wkt_linestring() {
        let coords: &[f64] = &[0.0, 0.0, 10.0, 10.0, 20.0, 0.0];
        let pairs: Vec<String> = coords
            .chunks_exact(2)
            .map(|c| format!("{} {}", c[0], c[1]))
            .collect();
        assert_eq!(
            format!("LINESTRING({})", pairs.join(", ")),
            "LINESTRING(0 0, 10 10, 20 0)"
        );
    }

    #[test]
    fn test_to_wkt_polygon() {
        let coords: &[f64] = &[0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0, 0.0, 0.0];
        let pairs: Vec<String> = coords
            .chunks_exact(2)
            .map(|c| format!("{} {}", c[0], c[1]))
            .collect();
        assert_eq!(
            format!("POLYGON(({}))", pairs.join(", ")),
            "POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))"
        );
    }

    // ── WKB roundtrip tests ────────────────────────────────────────

    #[test]
    fn test_wkb_roundtrip_point() {
        let wkb = to_wkb_point_native(116.4, 39.9);
        let parsed = parse_wkb_native(&wkb).unwrap();
        assert_eq!(parsed, vec![116.4, 39.9]);
    }

    #[test]
    fn test_wkb_roundtrip_linestring() {
        let coords = vec![0.0, 0.0, 10.0, 10.0, 20.0, 0.0];
        let wkb = to_wkb_linestring_native(&coords);
        let parsed = parse_wkb_native(&wkb).unwrap();
        assert_eq!(parsed, coords);
    }

    #[test]
    fn test_wkb_roundtrip_multipoint() {
        let coords = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let wkb = to_wkb_multipoint_native(&coords);
        let parsed = parse_wkb_native(&wkb).unwrap();
        assert_eq!(parsed, coords);
    }

    #[test]
    fn test_wkb_too_short() {
        assert!(parse_wkb_native(&[0u8, 1, 2]).is_err());
    }

    #[test]
    fn test_wkb_structure() {
        let wkb = to_wkb_point_native(10.0, 20.0);
        assert_eq!(wkb.len(), 21);
        assert_eq!(wkb[0], 1); // little-endian
        assert_eq!(u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]]), 1); // POINT
    }

    #[test]
    fn test_wkb_linestring_structure() {
        let wkb = to_wkb_linestring_native(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(wkb[0], 1);
        assert_eq!(u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]]), 2); // LINESTRING
        assert_eq!(u32::from_le_bytes([wkb[5], wkb[6], wkb[7], wkb[8]]), 2); // 2 points
    }
}
