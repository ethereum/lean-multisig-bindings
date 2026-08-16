# Python Binding Layout Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make Python the first language-specific binding in `lean-multisig-bindings`, without changing its published distribution or import API.

**Architecture:** The repository root becomes a small Rust workspace and cross-binding landing page. All Python-specific Rust source, Python packaging, tests, stubs, documentation, and release metadata move to `bindings/python/`. Future bindings add sibling directories and use their own native bridge; no C ABI is introduced.

**Tech Stack:** Rust workspace, PyO3, maturin, uv, pytest, mypy stubtest, GitHub Actions, Release Please.

### Task 1: Establish the root workspace and binding directory

**Files:**
- Create: `Cargo.toml`
- Create: `README.md`
- Move: `src/` → `bindings/python/src/`
- Move: `Cargo.toml` → `bindings/python/Cargo.toml`
- Move: `pyproject.toml`, `uv.lock`, `rust-toolchain.toml` → `bindings/python/`
- Move: `py.typed`, `py_lean_multisig.pyi`, `stubtest_allowlist.txt` → `bindings/python/`
- Move: `tests/` → `bindings/python/tests/`
- Move: `README.md` → `bindings/python/README.md`

**Step 1: Add the root workspace manifest.**

```toml
[workspace]
members = ["bindings/python"]
resolver = "2"
```

**Step 2: Move all Python-only build, source, test, and documentation files to `bindings/python/`; keep `Cargo.lock` at the workspace root.**

**Step 3: Add a root README that describes the repository as language bindings for leanMultisig, lists `bindings/python/`, and states that bindings use language-native wrappers over the Rust implementation.**

**Step 4: Update `bindings/python/README.md` to retain its install and API guidance, while identifying it as the Python binding.**

**Step 5: Verify the API remains unchanged.**

Run: `cd bindings/python && uv run --extra dev maturin develop --release && uv run --extra dev pytest tests/ -v`

Expected: 39 tests pass and imports remain `import py_lean_multisig as lm`.

### Task 2: Make metadata and automation binding-aware

**Files:**
- Modify: `bindings/python/pyproject.toml`
- Modify: `release-please-config.json`
- Modify: `.release-please-manifest.json`
- Move: `CHANGELOG.md` → `bindings/python/CHANGELOG.md`
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`

**Step 1: Update Python project URLs to `https://github.com/ethereum/lean-multisig-bindings` and its issues page, preserving the PyPI distribution name `py-lean-multisig`.**

**Step 2: Configure Release Please’s sole package at `bindings/python`, including its moved changelog and manifest entry.**

**Step 3: Point CI at `bindings/python` for virtualenv setup, maturin develop, pytest, and stubtest.**

**Step 4: Point release wheel, sdist, and artifact build invocations at `bindings/python/Cargo.toml`; retain the existing tag and PyPI publishing behavior.**

**Step 5: Confirm all YAML and JSON parse.**

Run: `ruby -e 'require "yaml"; ARGV.each { |p| YAML.load_file(p) }' .github/workflows/*.yml && jq empty release-please-config.json .release-please-manifest.json`

Expected: zero exit status.

### Task 3: Verify packages from their new locations

**Files:**
- Verify: `Cargo.toml`
- Verify: `bindings/python/Cargo.toml`
- Verify: `bindings/python/pyproject.toml`
- Verify: `bindings/python/tests/`

**Step 1: Run formatting and workspace compilation.**

Run: `cargo fmt --all -- --check && cargo check --workspace`

Expected: zero exit status.

**Step 2: Rebuild and test from the Python binding directory.**

Run: `cd bindings/python && uv run --extra dev maturin develop --release && uv run --extra dev pytest tests/ -v && uv run --extra dev python -m mypy.stubtest py_lean_multisig --allowlist stubtest_allowlist.txt`

Expected: 39 tests pass and stubtest succeeds.

**Step 3: Build an sdist using the moved manifest.**

Run: `maturin sdist --manifest-path bindings/python/Cargo.toml --out /tmp/lean-multisig-bindings-sdist`

Expected: one source archive is produced.

**Step 4: Review the diff for accidental API or generated-artifact changes.**

Run: `git diff --check && git status --short`

Expected: only the planned reorganization and metadata changes.

**Step 5: Commit.**

```bash
git add Cargo.toml Cargo.lock README.md bindings .github release-please-config.json .release-please-manifest.json
git commit -m "refactor: organize Python as a language binding"
```
