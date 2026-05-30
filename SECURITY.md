# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.2.x   | ✅        |
| 0.1.x   | ✅ (critical fixes only) |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Preferred**: Open a [GitHub Security Advisory](https://github.com/reed-soul/wasm-spatial-core/security/advisories/new) (confidential, visible only to maintainers).
2. **Alternative**: Send email to `qingxi@zhiqiweilai.com` with the subject `[wasm-spatial-core] Security`.

We will acknowledge your report within 48 hours and aim to provide a fix or mitigation within 7 days.

Please do **not** open a public issue for security vulnerabilities.

## Security Overview

`wasm-spatial-core` is designed with defense-in-depth:

- **WASM Sandbox** — All code runs inside the browser's WebAssembly sandbox with no filesystem or network access.
- **Input Size Limits** — All public APIs enforce a configurable input size cap (default 100 MB) via `validate_input_size()` / `setInputSizeLimit()`. Large inputs are rejected before processing.
- **No Network Requests** — The library makes zero outbound HTTP requests. All data processing is local.
- **Minimal `unsafe`** — Almost all code is safe Rust. A small `unsafe` block exists in `gltf_writer.rs` for byte-slice casting in GLB assembly; it is not used on untrusted paths without prior validation.
- **No Filesystem Access** — WASM linear memory is the only data store; no files are read or written.

## Known Limitations

- The library processes untrusted input (GeoJSON, LAS, IFC, etc.). Malformed input may cause panics (caught by the panic hook) but should not cause memory corruption thanks to Rust's ownership model and the WASM sandbox.
- The `multi-thread` feature uses `SharedArrayBuffer`, which requires specific HTTP headers (`Cross-Origin-Opener-Policy`, `Cross-Origin-Embedder-Policy`). Misconfigured servers may expose the page to Spectre-class attacks — this is a server configuration issue, not a library vulnerability.
