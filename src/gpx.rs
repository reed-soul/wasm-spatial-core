//! GPX (GPS Exchange Format) parsing.
//!
//! Parses GPX XML files to extract trackpoint coordinates, elevation data,
//! and track statistics. GPX is the standard format for GPS data exchange
//! used by GPS devices, fitness trackers, and mapping applications.

use wasm_bindgen::prelude::*;

use crate::validate_input_size;

// ===========================================================================
// Internal XML parsing helpers (hand-written, no quick-xml dependency)
// ===========================================================================

/// Extract all text content between `<trkpt>` and `</trkpt>` tags.
fn extract_trackpoints(gpx_xml: &str) -> Vec<String> {
    let mut trkpts = Vec::new();
    let mut in_trkpt = false;
    let mut start = 0usize;
    let tag_name = "<trkpt";
    let close_tag = "</trkpt>";

    let bytes = gpx_xml.as_bytes();
    let len = bytes.len();
    let tag_len = tag_name.len();
    let close_len = close_tag.len();

    let mut i = 0;
    while i + tag_len <= len {
        if in_trkpt {
            // Look for close tag
            if bytes[i..].starts_with(close_tag.as_bytes()) {
                trkpts.push(gpx_xml[start..i + close_len].to_string());
                in_trkpt = false;
                i += close_len;
                continue;
            }
        } else {
            // Look for open tag (case-insensitive for robustness)
            let slice = &gpx_xml[i..i + tag_len].to_lowercase();
            if slice == tag_name {
                // Check if this is followed by >
                let rest = &gpx_xml[i + tag_len..];
                let pos = rest.find('>').unwrap_or(rest.len());
                in_trkpt = true;
                start = i;
                i += tag_len + pos + 1;
                continue;
            }
        }
        i += 1;
    }

    trkpts
}

/// Extract latitude and longitude from a `<trkpt lat="..." lon="...">` tag.
fn parse_trkpt_coords(trkpt: &str) -> Option<(f64, f64)> {
    let lower = trkpt.to_lowercase();

    // Find lat="..."
    let lat_val = extract_attr(&lower, "lat")?;
    let lon_val = extract_attr(&lower, "lon")?;

    let lat: f64 = lat_val.parse().ok()?;
    let lon: f64 = lon_val.parse().ok()?;

    Some((lon, lat)) // our convention: lng, lat
}

/// Extract elevation from `<ele>` tag within a trackpoint.
fn parse_trkpt_elevation(trkpt: &str) -> Option<f64> {
    let lower = trkpt.to_lowercase();
    let open = "<ele>";
    let close = "</ele>";

    if let Some(start) = lower.find(open) {
        let content_start = start + open.len();
        if let Some(end) = lower[content_start..].find(close) {
            let val = lower[content_start..content_start + end].trim();
            return val.parse().ok();
        }
    }
    None
}

/// Extract time from `<time>` tag within a trackpoint (ISO 8601).
fn parse_trkpt_time(trkpt: &str) -> Option<String> {
    let lower = trkpt.to_lowercase();
    let open = "<time>";
    let close = "</time>";

    if let Some(start) = lower.find(open) {
        let content_start = start + open.len();
        if let Some(end) = lower[content_start..].find(close) {
            let val = lower[content_start..content_start + end].trim().to_string();
            return Some(val);
        }
    }
    None
}

/// Extract an attribute value from an XML-like tag: `attr="value"`.
fn extract_attr(xml: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr_name);
    if let Some(start) = xml.find(&pattern) {
        let val_start = start + pattern.len();
        if let Some(end) = xml[val_start..].find('"') {
            return Some(xml[val_start..val_start + end].to_string());
        }
    }
    // Try single quotes
    let pattern2 = format!("{}='", attr_name);
    if let Some(start) = xml.find(&pattern2) {
        let val_start = start + pattern2.len();
        if let Some(end) = xml[val_start..].find('\'') {
            return Some(xml[val_start..val_start + end].to_string());
        }
    }
    None
}

// ===========================================================================
// Core functions (testable without WASM)
// ===========================================================================

/// Parse GPX and extract all trackpoint coordinates as `[lng, lat, ...]`.
pub(crate) fn parse_gpx_core(input: &str) -> Result<Vec<f64>, String> {
    validate_input_size(input.len(), "parseGpx").map_err(|e| e.as_string().unwrap_or_default())?;

    let trkpts = extract_trackpoints(input);
    let mut out = Vec::with_capacity(trkpts.len() * 2);

    for trkpt in &trkpts {
        if let Some((lng, lat)) = parse_trkpt_coords(trkpt) {
            out.push(lng);
            out.push(lat);
        }
    }

    Ok(out)
}

/// Parse GPX and extract trackpoint coordinates with elevation as `[lng, lat, elev, ...]`.
pub(crate) fn parse_gpx_with_elevation_core(input: &str) -> Result<Vec<f64>, String> {
    validate_input_size(input.len(), "parseGpxWithElevation").map_err(|e| e.as_string().unwrap_or_default())?;

    let trkpts = extract_trackpoints(input);
    let mut out = Vec::with_capacity(trkpts.len() * 3);

    for trkpt in &trkpts {
        if let Some((lng, lat)) = parse_trkpt_coords(trkpt) {
            out.push(lng);
            out.push(lat);
            let elev = parse_trkpt_elevation(trkpt).unwrap_or(0.0);
            out.push(elev);
        }
    }

    Ok(out)
}

/// Compute track statistics from GPX data.
/// Returns JSON with: total_distance, total_ascent, total_descent, point_count, time_range.
pub(crate) fn gpx_track_stats_core(input: &str) -> Result<String, String> {
    validate_input_size(input.len(), "gpxTrackStats").map_err(|e| e.as_string().unwrap_or_default())?;

    let trkpts = extract_trackpoints(input);
    let mut total_distance = 0.0_f64;
    let mut total_ascent = 0.0_f64;
    let mut total_descent = 0.0_f64;
    let mut prev_elev: Option<f64> = None;
    let mut prev_lng: Option<f64> = None;
    let mut prev_lat: Option<f64> = None;
    let mut times: Vec<String> = Vec::new();
    let mut point_count = 0usize;

    for trkpt in &trkpts {
        if let Some((lng, lat)) = parse_trkpt_coords(trkpt) {
            point_count += 1;

            // Distance from previous point (Haversine)
            if let (Some(plng), Some(plat)) = (prev_lng, prev_lat) {
                let dist = haversine_gpx(plat, plng, lat, lng);
                total_distance += dist;
            }

            // Elevation gain/loss
            if let Some(elev) = parse_trkpt_elevation(trkpt) {
                if let Some(pe) = prev_elev {
                    let diff = elev - pe;
                    if diff > 0.0 {
                        total_ascent += diff;
                    } else {
                        total_descent += diff.abs();
                    }
                }
                prev_elev = Some(elev);
            }

            prev_lng = Some(lng);
            prev_lat = Some(lat);

            if let Some(time) = parse_trkpt_time(trkpt) {
                times.push(time);
            }
        }
    }

    let time_range = if times.len() >= 2 {
        let first = &times[0];
        let last = &times[times.len() - 1];
        serde_json::json!([first, last])
    } else {
        serde_json::json!(null)
    };

    let stats = serde_json::json!({
        "totalDistance": total_distance,
        "totalAscent": total_ascent,
        "totalDescent": total_descent,
        "pointCount": point_count,
        "timeRange": time_range,
    });

    serde_json::to_string(&stats).map_err(|e| format!("Failed to serialize stats: {}", e))
}

/// Haversine distance between two WGS-84 points.
fn haversine_gpx(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const EARTH_RADIUS: f64 = 6_371_000.0;
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let dlat = lat2_r - lat1_r;
    let dlng = (lng2 - lng1).to_radians();

    let a = (dlat / 2.0).sin() * (dlat / 2.0).sin()
        + lat1_r.cos() * lat2_r.cos() * (dlng / 2.0).sin() * (dlng / 2.0).sin();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS * c
}

// ===========================================================================
// WASM API
// ===========================================================================

/// Parse GPX and return all trackpoint coordinates as a flat `Float64Array`.
///
/// # Arguments
/// - `input`: GPX XML string.
///
/// # Returns
/// Flat `Float64Array` `[lng0, lat0, lng1, lat1, ...]`.
#[wasm_bindgen(js_name = "parseGpx")]
pub fn parse_gpx(input: &str) -> Result<js_sys::Float64Array, JsValue> {
    let coords = parse_gpx_core(input).map_err(|e| JsValue::from_str(&e))?;
    let arr = js_sys::Float64Array::new_with_length(coords.len() as u32);
    if !coords.is_empty() {
        arr.copy_from(&coords);
    }
    Ok(arr)
}

/// Parse GPX and return trackpoint coordinates with elevation as a flat `Float64Array`.
///
/// # Arguments
/// - `input`: GPX XML string.
///
/// # Returns
/// Flat `Float64Array` `[lng0, lat0, elev0, lng1, lat1, elev1, ...]`.
#[wasm_bindgen(js_name = "parseGpxWithElevation")]
pub fn parse_gpx_with_elevation(input: &str) -> Result<js_sys::Float64Array, JsValue> {
    let coords = parse_gpx_with_elevation_core(input).map_err(|e| JsValue::from_str(&e))?;
    let arr = js_sys::Float64Array::new_with_length(coords.len() as u32);
    if !coords.is_empty() {
        arr.copy_from(&coords);
    }
    Ok(arr)
}

/// Compute track statistics from GPX data.
///
/// # Arguments
/// - `input`: GPX XML string.
///
/// # Returns
/// JSON string with `totalDistance`, `totalAscent`, `totalDescent`, `pointCount`, `timeRange`.
#[wasm_bindgen(js_name = "gpxTrackStats")]
pub fn gpx_track_stats(input: &str) -> Result<String, JsValue> {
    gpx_track_stats_core(input).map_err(|e| JsValue::from_str(&e))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_gpx() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1">
  <trk>
    <name>Test Track</name>
    <trkseg>
      <trkpt lat="39.915" lon="116.404">
        <ele>50.0</ele>
        <time>2024-01-01T08:00:00Z</time>
      </trkpt>
      <trkpt lat="39.920" lon="116.410">
        <ele>60.0</ele>
        <time>2024-01-01T08:10:00Z</time>
      </trkpt>
      <trkpt lat="39.925" lon="116.415">
        <ele>55.0</ele>
        <time>2024-01-01T08:20:00Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#
    }

    #[test]
    fn test_parse_gpx_basic() {
        let coords = parse_gpx_core(simple_gpx()).unwrap();
        assert_eq!(coords.len(), 6); // 3 points × 2 coords
        assert!((coords[0] - 116.404).abs() < 1e-10);
        assert!((coords[1] - 39.915).abs() < 1e-10);
        assert!((coords[2] - 116.410).abs() < 1e-10);
        assert!((coords[3] - 39.920).abs() < 1e-10);
    }

    #[test]
    fn test_parse_gpx_with_elevation() {
        let coords = parse_gpx_with_elevation_core(simple_gpx()).unwrap();
        assert_eq!(coords.len(), 9); // 3 points × 3 values
        assert!((coords[0] - 116.404).abs() < 1e-10);
        assert!((coords[1] - 39.915).abs() < 1e-10);
        assert!((coords[2] - 50.0).abs() < 1e-10);
        assert!((coords[5] - 60.0).abs() < 1e-10);
        assert!((coords[8] - 55.0).abs() < 1e-10);
    }

    #[test]
    fn test_gpx_track_stats() {
        let stats_json = gpx_track_stats_core(simple_gpx()).unwrap();
        let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();

        assert_eq!(stats["pointCount"], 3);
        assert!(stats["totalDistance"].as_f64().unwrap() > 0.0);
        assert!(stats["totalAscent"].as_f64().unwrap() > 0.0); // 50→60 = 10m
        assert!((stats["totalDescent"].as_f64().unwrap() - 5.0).abs() < 1e-10); // 60→55 = 5m
    }

    #[test]
    fn test_gpx_empty() {
        let coords = parse_gpx_core("<?xml version=\"1.0\"?><gpx></gpx>").unwrap();
        assert_eq!(coords.len(), 0);
    }

    #[test]
    fn test_gpx_no_elevation() {
        let gpx = r#"<?xml version="1.0"?>
<gpx version="1.1">
  <trk>
    <trkseg>
      <trkpt lat="39.915" lon="116.404"></trkpt>
      <trkpt lat="39.920" lon="116.410"></trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let coords = parse_gpx_with_elevation_core(gpx).unwrap();
        assert_eq!(coords.len(), 6);
        // Missing elevation defaults to 0
        assert!((coords[2] - 0.0).abs() < 1e-10);
        assert!((coords[5] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_gpx_multiple_segments() {
        let gpx = r#"<?xml version="1.0"?>
<gpx version="1.1">
  <trk>
    <trkseg>
      <trkpt lat="39.9" lon="116.4"></trkpt>
    </trkseg>
    <trkseg>
      <trkpt lat="39.95" lon="116.45"></trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        let coords = parse_gpx_core(gpx).unwrap();
        assert_eq!(coords.len(), 4); // 2 points from 2 segments
    }

    #[test]
    fn test_gpx_case_insensitive() {
        let gpx = r#"<?xml version="1.0"?>
<gpx version="1.1">
  <trk>
    <trkseg>
      <TRKPT lat="39.915" LON="116.404">
        <ELE>100.0</ELE>
        <TIME>2024-01-01T00:00:00Z</TIME>
      </TRKPT>
    </trkseg>
  </trk>
</gpx>"#;

        let coords = parse_gpx_with_elevation_core(gpx).unwrap();
        assert_eq!(coords.len(), 3);
        assert!((coords[0] - 116.404).abs() < 1e-10);
        assert!((coords[2] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_extract_attr() {
        let xml = r#"<trkpt lat="39.915" lon="116.404">"#;
        assert_eq!(extract_attr(xml, "lat"), Some("39.915".to_string()));
        assert_eq!(extract_attr(xml, "lon"), Some("116.404".to_string()));
    }

    #[test]
    fn test_gpx_waypoints_not_extracted() {
        let gpx = r#"<?xml version="1.0"?>
<gpx version="1.1">
  <wpt lat="39.9" lon="116.4">
    <name>Waypoint 1</name>
  </wpt>
  <trk>
    <trkseg>
      <trkpt lat="39.915" lon="116.404"></trkpt>
    </trkseg>
  </trk>
</gpx>"#;

        // Waypoints should not be extracted — only trackpoints
        let coords = parse_gpx_core(gpx).unwrap();
        assert_eq!(coords.len(), 2); // Only 1 trackpoint
    }
}
