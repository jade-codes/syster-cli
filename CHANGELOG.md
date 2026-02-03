# Changelog

All notable changes to syster-cli will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
