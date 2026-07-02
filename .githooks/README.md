# Git hooks

This directory holds project-local git hooks. They are **opt-in per clone**
(git never runs hooks from a tracked directory by default for safety).

## Install (one-time, per clone)

```bash
git config core.hooksPath .githooks
```

After this, every `git commit` runs `.githooks/pre-commit` automatically.

## `pre-commit` — auto-format Rust

If the commit stages any `.rs` file, the hook runs `cargo fmt --all` and
re-stages any files fmt modified, so the commit always includes formatted
code. This kills the "local green, CI red on `cargo fmt --check`" loop that
historically regressed multiple times (commits `2c7a2b5`, the 3DGS fmt fix).

The hook is a no-op when:

- no `.rs` files are staged (docs/config-only commits), or
- `cargo` is not on `PATH` (non-Rust environments).

## Bypass

For a work-in-progress commit where you intentionally want unformatted code:

```bash
git commit --no-verify
```

CI will still enforce `cargo fmt --all -- --check`, so bypassing locally only
defers the check — don't push `--no-verify` commits without a follow-up fmt.
