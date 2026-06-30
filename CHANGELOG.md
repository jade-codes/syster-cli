# Changelog

All notable changes to syster-cli will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.6-alpha] - 2026-06-30

### Changed

- **syster-base**: Updated to v0.4.2-alpha
  - Fixes a false-positive `undefined reference` (E0001) on members inherited via a SemanticMetadata `baseType` implicit specialization (e.g. `:> ServiceMethod` inside a `#systemdd`-annotated element)
  - Includes three parser fixes from base #25 (`to` as a feature name, `actor def` definitions, prefix metadata with a body before a member)

## [0.4.5-alpha] - 2026-06-09

### Fixed

- **PyPI packaging**: Derive the Python wheel version from `Cargo.toml` via maturin's `dynamic = ["version"]` instead of a hardcoded `pyproject.toml` version. The two had drifted, so the 0.4.4-alpha release built stale `0.4.3a0` wheels and the PyPI publish failed; the wheel and crate versions can no longer diverge.

## [0.4.4-alpha] - 2026-06-09

### Changed

- **syster-base**: Updated to v0.4.1-alpha (conditional constraint invocation, unified short-form relationship edges, accept state, view def edges, byte-stable XMI round-trips)

## [0.4.3-alpha] - 2026-02-24

### Added

- **PyPI distribution**: Platform-specific wheels published to PyPI via maturin
- `pip install syster-cli` now installs the binary directly — no Rust toolchain needed

## [0.4.0-alpha] - 2026-02-15

### Added

- **Semantic model commands**: `--list`, `--query`, `--inspect`, `--rename`, `--add-member`, `--remove-member` for querying and editing models via the CLI
- **YAML export format**: New `--export yaml` option for human-readable model interchange
- **Decompile command**: `--decompile` option to convert XMI/JSON-LD back to SysML text with metadata
- **Import workspace**: `--import-workspace` flag to load interchange files for analysis with preserved element IDs
- **Self-contained export**: `--self-contained` flag to include stdlib in exports
- **Metadata round-trip**: Companion `.metadata.json` files preserve element IDs across rename, add, and remove operations
- **Import validation**: `--import` now distinguishes errors (exit 1) from warnings (exit 2) and clean imports (exit 0)
- **Import `_ext_` stub detection**: External reference stubs are reported as warnings, not errors

### Changed

- **syster-base**: Updated to v0.4.0-alpha (relationship unification, semantic views, change tracking)
- Made `load_input` and `load_stdlib_files` public for reuse in tests and downstream tooling
- Status/diagnostic messages consistently go to stderr; stdout reserved for data output
- Updated README with semantic editing commands, metadata docs, exit codes, and all CLI options
- Makefile targets now include `--features interchange` by default

### Fixed

- Removed unused `export_model` import in test suite
- Fixed clippy warnings for Rust 1.92 (`map_or` → `is_some_and`, `.map(|s| s.clone())` → `.cloned()`)

## [0.3.2-alpha] - 2026-02-09

### Changed

- **syster-base**: Updated to v0.3.2-alpha with semantic diagnostics API and false positive fixes

## [0.3.0-alpha] - 2026-02-03

### Changed

- **syster-base**: Updated to v0.3.0-alpha (Rowan parser refactor)

## [0.2.3-alpha] - 2026-02-02

### Added

- `export_from_host` function to export from pre-populated AnalysisHost
- Support for `--import-workspace --export` flag combination for direct roundtrip
- Element ID preservation in import-workspace → export pipeline
- Roundtrip test verifying XMI element IDs are preserved

### Changed

- Status messages go to stderr when `--export` is combined with `--import-workspace` (stdout reserved for data)

## [0.2.2-alpha] - 2026-01-29

### Changed

- **syster-base**: Updated to v0.2.2-alpha with SysML v2 Views support and filter import evaluation

## [0.2.0-alpha] - 2026-01-23

### Added

- Initial CLI implementation with analysis, export, and import commands
- Support for XMI and JSON-LD interchange formats via `--export` flag
- Workspace analysis with `--input` directory support
