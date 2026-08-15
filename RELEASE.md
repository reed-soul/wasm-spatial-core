# Release Process

How wasm-spatial-core versions are cut. The whole flow is one commit plus one
tag push — CI does the publishing.

## Versioning

- Semantic Versioning. During 0.x, **BREAKING changes bump the minor**
  (0.9 → 0.10); fixes and additions bump the patch. After 1.0, semver applies
  strictly (breaking → major).
- `Cargo.toml` is the single source of truth for the version — it drives the
  wasm-pack generated bindings and the `version()` export. Every other copy of
  the version string is synced to it and guarded by
  `scripts/check-version-sync.sh`, which CI runs on every push.

## What a tag push publishes

Pushing a tag matching `v*` (`.github/workflows/ci.yml`) triggers:

1. **`release-build`** — three WASM variants: `default` (zero-config),
   `point-cloud`, `multi-thread` (nightly + atomics, needs COOP/COEP).
2. **`github-release`** — a GitHub Release with the three tar.gz assets and a
   commit-log summary.
3. **`publish-npm`** — publishes from `npm/` using the curated
   `npm/package.json`: the `exports` subpaths (`/node`, `/abort`, `/webgpu`,
   `/draco`), compiled JS entries, and both the web- and nodejs-target WASM
   builds, with provenance.

npm publishes are public and permanent. **Never move or re-point a tag** —
if a release is bad, fix forward and cut the next version.

## Pre-flight

Run before making the release commit:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
bash scripts/check-version-sync.sh        # green BEFORE the bump, green AFTER
cd npm && npm install --ignore-scripts && npm run typecheck && cd ..
```

If you touched wasm-bindgen surfaces, also smoke-test the artifacts (CI covers
these, but local runs catch issues earlier):

```bash
wasm-pack build --target web --release --out-dir pkg --features point-cloud,geotiff,laz-support,mesh-ingest
wasm-pack build --target nodejs --release --out-dir pkg-node --features point-cloud,geotiff,laz-support,mesh-ingest
WASM_PKG_DIR=pkg-node node tests/wasm_smoke_test.mjs
node tests/node_batch_test.mjs
```

Finish by waiting for a fully green CI run on master.

## The release commit

One commit, `chore(release): vX.Y.Z`, bumping every version string in lockstep
(the list below is exactly what `check-version-sync.sh` enforces):

| File | Change |
|------|--------|
| `Cargo.toml` | `version = "X.Y.Z"` — source of truth; `Cargo.lock` follows automatically |
| `npm/package.json` | `"version": "X.Y.Z"` |
| `site/llms.txt` | `Version X.Y.Z` in the header paragraph |
| `site/index.html` | JSON-LD `"version"` **and** hero badge `vX.Y.Z` (2 spots) |
| `site/shared/site-nav.mjs` | nav badge `vX.Y.Z` |
| `site/shared/brand/og-image.svg` | OG image label `vX.Y.Z` |
| `examples/shared/site-shell.mjs` | demo nav badge `vX.Y.Z` |
| `CHANGELOG.md` | `## [Unreleased]` → `## [X.Y.Z] - YYYY-MM-DD`; update the compare link and add the tag link at the bottom |

If the release contains BREAKING changes, write `docs/MIGRATION_VX_Y.md` and
link it from the CHANGELOG version header and the README.

## Tag and publish

```bash
git tag -a vX.Y.Z -m "wasm-spatial-core vX.Y.Z"
git push origin master vX.Y.Z
```

Then verify:

- the three tag jobs (`release-build`, `github-release`, `publish-npm`) pass;
- the GitHub Release exists with all three tar.gz assets;
- `npm view wasm-spatial-core version` reports `X.Y.Z` and
  `npm view wasm-spatial-core dist-tags` shows `latest` on it;
- site badges pick up the new version on the next master deploy.
