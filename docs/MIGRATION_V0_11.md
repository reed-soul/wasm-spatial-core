# Migrating from 0.10 to 0.11

0.11 is the **last 0.x breaking window**. It does not add engine capability —
it makes every public WASM failure a typed `SpatialError` and removes one
deprecated CRS alias. After 0.11, the next release is 1.0.0 (no API changes).

## 1. `bestCrsForRegion` removed

Use `suggestCrsHeuristic`. The two functions were identical; the old name
implied a PROJ-grade “best CRS” recommendation, which this engine does not
do.

```js
// 0.10
const json = core.bestCrsForRegion(minLng, minLat, maxLng, maxLat);

// 0.11
const json = core.suggestCrsHeuristic(minLng, minLat, maxLng, maxLat);
```

TypeScript consumers who imported `bestCrsForRegion` from
`wasm-spatial-core` must switch the named import.

## 2. WASM errors are always `SpatialError`

Invalid input that previously threw a **plain string** (no `.code`) now
throws a `js_sys::Error` with:

| Property | Value |
|----------|--------|
| `name` | `"SpatialError"` |
| `code` | `"INVALID_INPUT"`, `"PARSE_ERROR"`, `"TILE_ERROR"`, … |
| `message` | human-readable detail |

```js
try {
  core.parseGeoJsonCoords("not json");
} catch (e) {
  if (e.code === "PARSE_ERROR") {
    // handle
  }
}
```

One concrete fix: `encodeTerrainTmsPyramid` with `bounds.length !== 4` used
to throw a raw string. It now throws `INVALID_INPUT`.

## 3. Default npm WASM includes more features

The published package now compiles `terrain-edit`, `mesh-edit`, and `webgpu`
into the WASM binary (in addition to `point-cloud`, `geotiff`, `laz-support`,
and `mesh-ingest`). JS-only subpaths (`/abort`, `/webgpu`, `/draco`) are
unchanged. `e57-support` and `multi-thread` stay opt-in source builds.
