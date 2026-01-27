# Plan: CLI Parse/Resolve Implementation

## Overview

Update the CLI to use the new syster-base v0.2.x API for parsing and resolving SysML/KerML files.

## Current State

The CLI uses the **old API** which has been removed in base v0.2.x:
- `Workspace` - removed
- `WorkspaceLoader` - removed  
- `StdLibLoader` - removed
- `populate_all()` - removed

## New API (base v0.2.x)

| Old API | New API |
|---------|---------|
| `Workspace` | `AnalysisHost` |
| `WorkspaceLoader.load_file()` | `AnalysisHost.set_file_content()` |
| `StdLibLoader` | Manual file loading or embedded stdlib |
| `workspace.populate_all()` | `host.analysis()` (auto-rebuilds index) |
| `workspace.symbol_table()` | `host.symbol_index()` |
| - | `check_file()` for diagnostics |

## Tasks

### Phase 1: Update Dependencies

- [ ] **1.1** Update `Cargo.toml` to use syster-base v0.2.x
- [ ] **1.2** Add `walkdir` dependency for directory traversal
- [ ] **1.3** Remove unused dependencies

### Phase 2: Rewrite Core Library

- [ ] **2.1** Replace `Workspace` with `AnalysisHost`
- [ ] **2.2** Implement `load_file()` helper using `set_file_content()`
- [ ] **2.3** Implement `load_directory()` helper with walkdir
- [ ] **2.4** Implement `load_stdlib()` helper
- [ ] **2.5** Update `AnalysisResult` struct with diagnostics
- [ ] **2.6** Implement diagnostic collection using `check_file()`

### Phase 3: Update CLI Output

- [ ] **3.1** Add error/warning output formatting
- [ ] **3.2** Exit with code 1 if errors found
- [ ] **3.3** Add verbose diagnostic output

### Phase 4: Testing

- [ ] **4.1** Update/fix existing tests
- [ ] **4.2** Add test for single file parsing
- [ ] **4.3** Add test for directory parsing
- [ ] **4.4** Add test for error reporting

---

## Implementation Details

### New `lib.rs` Structure

```rust
use std::path::{Path, PathBuf};
use syster::ide::AnalysisHost;
use syster::hir::{check_file, Diagnostic, Severity};

#[derive(Debug)]
pub struct AnalysisResult {
    pub file_count: usize,
    pub symbol_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub diagnostics: Vec<DiagnosticInfo>,
}

#[derive(Debug)]
pub struct DiagnosticInfo {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub message: String,
    pub severity: Severity,
    pub code: Option<String>,
}

pub fn run_analysis(
    input: &PathBuf,
    verbose: bool,
    load_stdlib: bool,
    stdlib_path: Option<&PathBuf>,
) -> Result<AnalysisResult, String> {
    let mut host = AnalysisHost::new();
    
    // 1. Load stdlib if requested
    if load_stdlib {
        load_stdlib_files(&mut host, stdlib_path, verbose)?;
    }
    
    // 2. Load input file(s)
    load_input(&mut host, input, verbose)?;
    
    // 3. Trigger index rebuild and get analysis
    let _analysis = host.analysis();
    
    // 4. Collect diagnostics from all files
    let diagnostics = collect_diagnostics(&host);
    
    // 5. Build result
    let error_count = diagnostics.iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .count();
    let warning_count = diagnostics.iter()
        .filter(|d| matches!(d.severity, Severity::Warning))
        .count();
    
    Ok(AnalysisResult {
        file_count: host.file_count(),
        symbol_count: host.symbol_index().all_symbols().count(),
        error_count,
        warning_count,
        diagnostics,
    })
}
```

### File Loading Helpers

```rust
fn load_input(host: &mut AnalysisHost, input: &Path, verbose: bool) -> Result<(), String> {
    if input.is_file() {
        load_file(host, input, verbose)
    } else if input.is_dir() {
        load_directory(host, input, verbose)
    } else {
        Err(format!("Path does not exist: {}", input.display()))
    }
}

fn load_file(host: &mut AnalysisHost, path: &Path, verbose: bool) -> Result<(), String> {
    if verbose {
        println!("  Loading: {}", path.display());
    }
    
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    
    let path_str = path.to_string_lossy();
    let parse_errors = host.set_file_content(&path_str, &content);
    
    // Parse errors are reported but don't fail the load
    for err in parse_errors {
        eprintln!("parse error: {}:{}: {}", path.display(), err.line, err.message);
    }
    
    Ok(())
}

fn load_directory(host: &mut AnalysisHost, dir: &Path, verbose: bool) -> Result<(), String> {
    use walkdir::WalkDir;
    
    if verbose {
        println!("Scanning directory: {}", dir.display());
    }
    
    for entry in WalkDir::new(dir).follow_links(true) {
        let entry = entry.map_err(|e| format!("Walk error: {}", e))?;
        let path = entry.path();
        
        if is_sysml_file(path) {
            load_file(host, path, verbose)?;
        }
    }
    
    Ok(())
}

fn is_sysml_file(path: &Path) -> bool {
    path.is_file() && matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("sysml") | Some("kerml")
    )
}
```

### Stdlib Loading

```rust
fn load_stdlib_files(
    host: &mut AnalysisHost, 
    custom_path: Option<&PathBuf>,
    verbose: bool,
) -> Result<(), String> {
    if verbose {
        println!("Loading standard library...");
    }
    
    // Try custom path first
    if let Some(path) = custom_path {
        if path.exists() {
            return load_directory(host, path, verbose);
        } else {
            return Err(format!("Stdlib path does not exist: {}", path.display()));
        }
    }
    
    // Try default locations
    let default_paths = [
        PathBuf::from("sysml.library"),
        PathBuf::from("../sysml.library"),
        // Could also check env var or embedded resources
    ];
    
    for path in &default_paths {
        if path.exists() {
            return load_directory(host, path, verbose);
        }
    }
    
    if verbose {
        println!("  Warning: Standard library not found");
    }
    
    Ok(())
}
```

### Diagnostic Collection

```rust
fn collect_diagnostics(host: &AnalysisHost) -> Vec<DiagnosticInfo> {
    use syster::hir::check_file;
    
    let mut all_diagnostics = Vec::new();
    
    for (path, _) in host.files() {
        if let Some(file_id) = host.get_file_id_for_path(path) {
            let file_path = path.to_string_lossy().to_string();
            let diagnostics = check_file(host.symbol_index(), file_id);
            
            for diag in diagnostics {
                all_diagnostics.push(DiagnosticInfo {
                    file: file_path.clone(),
                    line: diag.start_line + 1,  // 1-indexed for display
                    col: diag.start_col + 1,
                    end_line: diag.end_line + 1,
                    end_col: diag.end_col + 1,
                    message: diag.message.to_string(),
                    severity: diag.severity,
                    code: diag.code.map(|c| c.to_string()),
                });
            }
        }
    }
    
    // Sort by file, then line, then column
    all_diagnostics.sort_by(|a, b| {
        (&a.file, a.line, a.col).cmp(&(&b.file, b.line, b.col))
    });
    
    all_diagnostics
}
```

### Updated `main.rs`

```rust
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use syster_cli::{run_analysis, DiagnosticInfo};
use syster::hir::Severity;

#[derive(Parser)]
#[command(name = "syster")]
#[command(about = "SysML v2 parser and semantic analyzer", long_about = None)]
struct Cli {
    /// Input file or directory to analyze
    #[arg(value_name = "FILE")]
    input: PathBuf,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Skip loading standard library
    #[arg(long)]
    no_stdlib: bool,

    /// Path to custom standard library
    #[arg(long, value_name = "PATH")]
    stdlib_path: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    
    match run_analysis(&cli.input, cli.verbose, !cli.no_stdlib, cli.stdlib_path.as_ref()) {
        Ok(result) => {
            // Print diagnostics
            for diag in &result.diagnostics {
                print_diagnostic(diag);
            }
            
            // Print summary
            if result.error_count == 0 {
                println!(
                    "✓ Analyzed {} files: {} symbols, {} warnings",
                    result.file_count,
                    result.symbol_count,
                    result.warning_count
                );
                ExitCode::SUCCESS
            } else {
                eprintln!(
                    "✗ Analyzed {} files: {} errors, {} warnings",
                    result.file_count,
                    result.error_count,
                    result.warning_count
                );
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn print_diagnostic(diag: &DiagnosticInfo) {
    let prefix = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
        Severity::Hint => "hint",
    };
    
    let code_suffix = diag.code.as_ref()
        .map(|c| format!("[{}]", c))
        .unwrap_or_default();
    
    eprintln!(
        "{}{}:{}:{}: {}",
        prefix,
        code_suffix,
        diag.file,
        diag.line,
        diag.message
    );
}
```

---

## Updated Cargo.toml

```toml
[package]
name = "syster-cli"
version = "0.2.0-alpha"
edition = "2024"
rust-version = "1.85"

[[bin]]
name = "syster"
path = "src/main.rs"

[lib]
name = "syster_cli"
path = "src/lib.rs"

[dependencies]
syster-base = { git = "https://github.com/jade-codes/syster-base.git", tag = "v0.2.1-alpha" }
clap = { version = "4", features = ["derive"] }
walkdir = "2"

[dev-dependencies]
tempfile = "3"
```

---

## Future Enhancements (Out of Scope)

1. **JSON output** (`--format json`) for CI integration
2. **SARIF output** for GitHub code scanning
3. **Colored output** using `termcolor`
4. **Watch mode** (`--watch`) for continuous analysis
5. **LSP mode** (`--lsp`) to run as language server
6. **Embedded stdlib** to avoid external dependency

---

## Test Plan

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_parse_single_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.sysml");
        fs::write(&file, "package Test {}").unwrap();
        
        let result = run_analysis(&file, false, false, None).unwrap();
        assert_eq!(result.file_count, 1);
        assert!(result.error_count == 0);
    }

    #[test]
    fn test_parse_directory() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.sysml"), "package A {}").unwrap();
        fs::write(dir.path().join("b.sysml"), "package B {}").unwrap();
        
        let result = run_analysis(&dir.path().to_path_buf(), false, false, None).unwrap();
        assert_eq!(result.file_count, 2);
    }

    #[test]
    fn test_error_reporting() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.sysml");
        fs::write(&file, "package Test { part p : Unknown; }").unwrap();
        
        let result = run_analysis(&file, false, false, None).unwrap();
        assert!(result.error_count > 0); // Unknown type
    }
}
```

### Integration Tests

```bash
# Parse single file
syster test.sysml

# Parse directory
syster ./models/

# With stdlib
syster --stdlib-path ./sysml.library test.sysml

# Verbose mode
syster -v test.sysml

# Check for errors (CI mode)
syster test.sysml && echo "No errors" || echo "Has errors"
```
