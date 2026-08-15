# Migrating from 0.9 to 0.10

0.10 is a correctness release: it fixes critical bugs (COPC default-build
breakage, b3dm index truncation above 65,535 vertices, geohash decoding for
repeated characters, a COPC hierarchy DoS, several panic-on-input paths) and
ships the first correctly-packaged npm tarball (subpath exports like
`wasm-spatial-core/abort` and `wasm-spatial-core/node` now actually resolve).

Two of those fixes could not be made without changing API shapes. This guide
covers every BREAKING change; each section shows the old and new code.

## 1. Batch UTM APIs carry the hemisphere explicitly

**Why:** `batchUtmToWgs84` used a `northing >= 0` heuristic to guess the
hemisphere. Valid UTM northings are always positive (southern-hemisphere
northings have 10,000,000 added), so **every southern-hemisphere point decoded
with a ~10,000,000 m error**. The hemisphere is now explicit data instead of a
guess.

### `batchWgs84ToUtm` — output layout changed from 3 to 4 values per point

The 4th value is `isNorth`: `1.0` for northern hemisphere, `0.0` for southern.

```js
// 0.9 — [zone, easting, northing] per point
const utm = core.batchWgs84ToUtm(coords);
const zone = utm[0], easting = utm[1], northing = utm[2];

// 0.10 — [zone, easting, northing, isNorth] per point
const utm = core.batchWgs84ToUtm(coords);
const [zone, easting, northing, isNorth] = utm;
```

### `batchUtmToWgs84` — input layout changed from 3 to 4 values per point

You must now tell the engine which hemisphere your northings are in:

```js
// 0.9 — hemisphere guessed (wrong south of the equator)
const wgs84 = core.batchUtmToWgs84(utm3);            // [zone, easting, northing]

// 0.10 — [zone, easting, northing, isNorth] per point
const wgs84 = core.batchUtmToWgs84(new Float64Array([
  zone, easting, northing, isNorth ? 1.0 : 0.0,
]));
```

If your data comes from `batchWgs84ToUtm`, just feed its output straight back —
the layouts now match.

### `batchWgs84ToUtmInPlace` / `batchUtmToWgs84InPlace` — same 4-value layout

The typed array is still mutated in place; only the per-point stride changes
from 3 to 4. Buffers must be sized `4 × pointCount`.

## 2. `error_to_js` throws a real `Error`

Errors thrown from the WASM side are now real `js_sys::Error` objects instead
of plain objects. **Fix, not breakage, for most code** — but if you matched on
the exact thrown shape, note:

```js
try {
  core.parseLasPoints(bytes);
} catch (e) {
  // 0.10: both now work (they did not before)
  e instanceof Error;   // true
  e.stack;              // available
  e.code;               // still present, e.g. "INVALID_INPUT"
  e.name;               // "SpatialError"
}
```

## 3. ENU frame construction and the `*_core` helpers return `Result`

`createEnuFrame` / `EnuFrame.from_anchor` and the exported
`batch_wgs84_to_enu_core` / `batch_enu_to_wgs84_core` helpers previously
panicked (aborting the whole WASM instance) on non-finite anchors or latitudes
outside [-90, 90]. They now validate and throw a `SpatialError` instead —
JS call sites only change if they relied on the panic path, which no correct
code did.

## 4. `batchWgs84ToCartesian3` returns errors instead of panicking

Odd-length input used to abort the WASM instance; it now throws
`SpatialError` (`e.code === "INVALID_INPUT"`). No layout change.

---

### Quick checklist

- [ ] Regenerate any cached UTM values — **all southern-hemisphere conversions
      from ≤ 0.9 are wrong by ~10,000 km and must be recomputed.**
- [ ] Update stride arithmetic for the four batch UTM APIs (3 → 4 per point).
- [ ] If you parse thrown errors structurally, read `e.code` (unchanged) —
      don't depend on the error not being an `Error` instance.
