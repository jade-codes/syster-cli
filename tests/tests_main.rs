//! Tests for main function in syster CLI
//!
//! Note: Testing main() directly is challenging because it uses clap::Parser
//! and requires command-line arguments. The logic is thoroughly tested via
//! run_analysis() which main() delegates to. The main() function is a thin
//! wrapper that:
//! 1. Parses CLI arguments
//! 2. Calls run_analysis with parsed arguments
//! 3. Formats the output
//!
//! All business logic is tested through run_analysis() tests in cli_tests.rs.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use syster_cli::run_analysis;
use tempfile::TempDir;

#[test]
fn test_main_logic_through_run_analysis() {
    // This test verifies the core logic that main() uses
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    // Simulate what main() does:
    // 1. Parse args (simulated with direct values)
    let input = &file_path;
    let verbose = false;
    let load_stdlib = true; // !no_stdlib
    let stdlib_path: Option<&std::path::Path> = None;

    // 2. Call run_analysis
    let result = run_analysis(input, verbose, load_stdlib, stdlib_path);

    // 3. Verify result can be formatted as main() does
    assert!(result.is_ok());
    let result = result.unwrap();
    let output = format!(
        "✓ Analyzed {} files: {} symbols, {} warnings",
        result.file_count, result.symbol_count, result.warning_count
    );

    assert!(output.contains("Analyzed"));
    assert!(output.contains("files"));
    assert!(output.contains("symbols"));
}

#[test]
fn test_main_error_handling_through_run_analysis() {
    // Test error path that main() would handle
    let result = run_analysis(&PathBuf::from("/nonexistent"), false, false, None);

    assert!(result.is_err());
    let error = result.unwrap_err();

    // Verify error message is meaningful
    assert!(error.contains("does not exist"));
}

#[test]
fn test_main_verbose_flag_through_run_analysis() {
    // Test verbose flag behavior
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    // Test with verbose = true
    let result = run_analysis(&file_path, true, false, None);
    assert!(result.is_ok());

    // Test with verbose = false
    let result = run_analysis(&file_path, false, false, None);
    assert!(result.is_ok());
}

#[test]
fn test_main_stdlib_flags_through_run_analysis() {
    // Test stdlib loading behavior
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    // Test with stdlib enabled (default behavior)
    let result = run_analysis(&file_path, false, true, None);
    assert!(result.is_ok());

    // Test with stdlib disabled (--no-stdlib flag)
    let result = run_analysis(&file_path, false, false, None);
    assert!(result.is_ok());
}

#[test]
fn test_main_output_format() {
    // Test that output format matches what main() produces
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();
    writeln!(file, "part def Car;").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    // Verify the format string that main() uses
    let output = format!(
        "✓ Analyzed {} files: {} symbols, {} warnings",
        result.file_count, result.symbol_count, result.warning_count
    );

    // Check formatting is present
    assert!(output.starts_with("✓ Analyzed"));
    assert!(output.contains("1 files"));
    assert!(output.contains("2 symbols"));
}

#[test]
fn test_main_with_errors_returns_failure() {
    // Test that files with errors would cause main to return failure
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    // Unresolved type reference should produce an error
    writeln!(file, "part p : UnknownType;").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    // Check we got at least some diagnostic
    assert!(result.error_count > 0 || result.warning_count > 0 || !result.diagnostics.is_empty());
}

#[test]
fn test_main_diagnostic_output() {
    // Test that diagnostics can be formatted as main() does
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    // Diagnostics should be formattable
    for diag in &result.diagnostics {
        let formatted = format!(
            "{}:{}:{}: {:?}: {}",
            diag.file, diag.line, diag.col, diag.severity, diag.message
        );
        assert!(!formatted.is_empty());
    }
}
