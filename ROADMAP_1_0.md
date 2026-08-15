# Roadmap: 1.0

Status: **draft** — the path from v0.10 to a stable 1.0.

V1 delivered the point-cloud → 3D Tiles pipeline; the V2 waves (W1 runtime,
W2 Spatial IR, W3 terrain, W4 WebGPU, W5 mesh edit) are complete. Nothing on
this roadmap adds new engine capability — 1.0 is a **stability and
credibility** milestone: the promise that the public API can be relied on.

## Release path

| Release | Theme | Breaking window |
|---------|-------|-----------------|
| **v0.10** | Correctness — OCR-audit fixes (COPC DoS, geohash, b3dm truncation, UTM hemisphere), npm packaging fixed (real `exports` map) | yes (UTM layout, `Result` returns) |
| **v0.11** | API hygiene — `SpatialError` everywhere, error taxonomy cleanup, last deprecation sweep | **yes — the final 0.x breaking window** |
| **1.0.0** | Stability — docs complete, benchmarks current, no API changes | no |

## 1.0 acceptance criteria

1. **Error handling is uniform.** Every public wasm-bindgen API reports
   failures as `SpatialError` (typed `code`). The remaining
   `JsValue::from_str` escape hatches listed in CONTRIBUTING.md are gone.
2. **Deprecation policy is written and followed.** After 1.0: deprecations
   land in minor releases with `#[deprecated]` + CHANGELOG entry + migration
   note; removals only in majors. 0.11 is the last chance for gratuitous
   breakage.
3. **Docs match the shipped API.** typedoc (published to Pages) covers every
   exported symbol; every performance claim in README/site cites a published,
   reproducible benchmark (`/benchmarks/`, head-to-head harness).
4. **Release infrastructure is guarded.** Version-string sync
   (`scripts/check-version-sync.sh`) and RELEASE.md are in place and enforced
   in CI. *(done in 0.10)*
5. **The npm package installs and imports from the public registry.** Someone
   other than the author can `npm install wasm-spatial-core` in a fresh Vite
   project and load `/node`, `/abort`, `/webgpu` subpaths. *(packaging fixed
   in 0.10; the from-registry smoke test is the 1.0 gate)*
6. *(stretch)* **Browser-side Rust tests.** `tests/web.rs` grows from 1
   `wasm-bindgen-test` to at least one smoke test per exported module group,
   so the browser target is exercised by Rust CI, not only by Playwright.

## Explicitly not gating 1.0

- New formats, Spatial IR completion (Heightfield path), streaming, GPU work —
  these continue on their own track (see ROADMAP_V2 / ENGINE_BOUNDARY) and
  ship whenever ready; 1.0 only freezes what already exists.
- crates.io publishing — optional decision before 1.0; default is npm-only.
  The crate builds natively (`rlib`), so the door stays open.
- MSRV pinning — the project tracks stable Rust (see ci.yml).
