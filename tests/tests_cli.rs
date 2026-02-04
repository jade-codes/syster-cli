//! Tests for syster-cli run_analysis function
//!
//! These tests cover various scenarios for parsing and analyzing SysML/KerML files.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use syster_cli::{export_ast, export_json, run_analysis};
use tempfile::TempDir;

// ============================================================================
// BASIC FILE OPERATIONS
// ============================================================================

#[test]
fn test_analyze_single_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    assert!(result.symbol_count > 0);
    assert_eq!(result.error_count, 0);
}

#[test]
fn test_analyze_directory() {
    let temp_dir = TempDir::new().unwrap();

    let file1 = temp_dir.path().join("file1.sysml");
    let mut f1 = fs::File::create(&file1).unwrap();
    writeln!(f1, "part def Car;").unwrap();

    let file2 = temp_dir.path().join("file2.sysml");
    let mut f2 = fs::File::create(&file2).unwrap();
    writeln!(f2, "part def Bike;").unwrap();

    let result = run_analysis(temp_dir.path(), false, false, None).unwrap();

    assert_eq!(result.file_count, 2);
    assert!(result.symbol_count >= 2);
}

#[test]
fn test_nonexistent_path() {
    let result = run_analysis(&PathBuf::from("/nonexistent/path"), false, false, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    let result = run_analysis(temp_dir.path(), false, false, None).unwrap();

    assert_eq!(result.file_count, 0);
    assert_eq!(result.symbol_count, 0);
}

#[test]
fn test_nested_directory_structure() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested directory structure
    let subdir1 = temp_dir.path().join("models");
    let subdir2 = temp_dir.path().join("models/vehicles");
    fs::create_dir_all(&subdir2).unwrap();

    // Create files in different directories
    let file1 = temp_dir.path().join("root.sysml");
    let mut f1 = fs::File::create(&file1).unwrap();
    writeln!(f1, "package RootPackage {{ }}").unwrap();

    let file2 = subdir1.join("model.sysml");
    let mut f2 = fs::File::create(&file2).unwrap();
    writeln!(f2, "part def Component;").unwrap();

    let file3 = subdir2.join("vehicle.sysml");
    let mut f3 = fs::File::create(&file3).unwrap();
    writeln!(f3, "part def Car;").unwrap();

    let result = run_analysis(temp_dir.path(), false, false, None).unwrap();

    assert_eq!(result.file_count, 3);
    assert!(result.symbol_count >= 3);
}

// ============================================================================
// STDLIB LOADING
// ============================================================================

#[test]
fn test_without_stdlib() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    assert_eq!(result.symbol_count, 1);
}

#[test]
fn test_custom_stdlib_path() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    // Create a custom stdlib directory
    let custom_stdlib = temp_dir.path().join("custom_stdlib");
    fs::create_dir_all(&custom_stdlib).unwrap();

    let stdlib_file = custom_stdlib.join("Base.sysml");
    let mut sf = fs::File::create(&stdlib_file).unwrap();
    writeln!(sf, "package Base {{ }}").unwrap();

    let result = run_analysis(&file_path, false, true, Some(&custom_stdlib)).unwrap();

    // File count includes both the input file AND custom stdlib files
    assert!(result.file_count >= 2);
}

#[test]
fn test_nonexistent_stdlib_path() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    let bad_stdlib = PathBuf::from("/nonexistent/stdlib");
    let result = run_analysis(&file_path, false, true, Some(&bad_stdlib));

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

// ============================================================================
// VERBOSE MODE
// ============================================================================

#[test]
fn test_verbose_mode() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    // Verbose mode should still succeed, just with more output
    let result = run_analysis(&file_path, true, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    assert!(result.symbol_count > 0);
}

// ============================================================================
// SYSML SYNTAX VARIATIONS
// ============================================================================

#[test]
fn test_complex_sysml() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("complex.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "package TestPackage {{").unwrap();
    writeln!(file, "    part def Vehicle {{").unwrap();
    writeln!(file, "        part engine;").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    // Should have: package, part def, part usage
    assert!(result.symbol_count >= 3);
}

#[test]
fn test_multiple_packages() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("multi.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "package Package1 {{ }}").unwrap();
    writeln!(file, "package Package2 {{ }}").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    assert!(result.symbol_count >= 2);
}

#[test]
fn test_with_imports() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("imports.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "package Test {{").unwrap();
    writeln!(file, "    import OtherPackage::*;").unwrap();
    writeln!(file, "    part def Vehicle;").unwrap();
    writeln!(file, "}}").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    // package + part def (import doesn't create a symbol)
    assert!(result.symbol_count >= 2);
}

#[test]
fn test_with_specialization() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("specialize.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "package Vehicles {{").unwrap();
    writeln!(file, "    part def Vehicle;").unwrap();
    writeln!(file, "    part def Car :> Vehicle;").unwrap();
    writeln!(file, "}}").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    assert!(result.symbol_count >= 3);
    assert_eq!(result.error_count, 0);
}

#[test]
fn test_with_attributes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("attrs.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "package Test {{").unwrap();
    writeln!(file, "    part def Vehicle {{").unwrap();
    writeln!(file, "        attribute mass;").unwrap();
    writeln!(file, "        attribute speed;").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    // package + part def + 2 attributes
    assert!(result.symbol_count >= 4);
}

#[test]
fn test_with_ports() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("ports.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "package Test {{").unwrap();
    writeln!(file, "    port def FuelPort;").unwrap();
    writeln!(file, "    part def Engine {{").unwrap();
    writeln!(file, "        port fuelIn : FuelPort;").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    assert!(result.symbol_count >= 4);
}

#[test]
fn test_with_connections() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("connections.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "package Test {{").unwrap();
    writeln!(file, "    part def A {{ port p; }}").unwrap();
    writeln!(file, "    part def B {{ port q; }}").unwrap();
    writeln!(file, "    part def System {{").unwrap();
    writeln!(file, "        part a : A;").unwrap();
    writeln!(file, "        part b : B;").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    assert!(result.symbol_count >= 7);
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn test_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.sysml");

    fs::File::create(&file_path).unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    assert_eq!(result.symbol_count, 0);
    assert_eq!(result.error_count, 0);
}

#[test]
fn test_comments_only() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("comments.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "// This is a comment").unwrap();
    writeln!(file, "/* Multi-line comment */").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    assert_eq!(result.symbol_count, 0);
}

#[test]
fn test_whitespace_only() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("whitespace.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "   ").unwrap();
    writeln!(file, "\t\t").unwrap();
    writeln!(file).unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    assert_eq!(result.symbol_count, 0);
}

#[test]
fn test_non_sysml_files_ignored() {
    let temp_dir = TempDir::new().unwrap();

    // Create a sysml file
    let sysml_file = temp_dir.path().join("model.sysml");
    let mut f1 = fs::File::create(&sysml_file).unwrap();
    writeln!(f1, "part def Vehicle;").unwrap();

    // Create non-sysml files that should be ignored
    let txt_file = temp_dir.path().join("readme.txt");
    let mut f2 = fs::File::create(&txt_file).unwrap();
    writeln!(f2, "This is a readme").unwrap();

    let rs_file = temp_dir.path().join("main.rs");
    let mut f3 = fs::File::create(&rs_file).unwrap();
    writeln!(f3, "fn main() {{}}").unwrap();

    let result = run_analysis(temp_dir.path(), false, false, None).unwrap();

    // Only the .sysml file should be loaded
    assert_eq!(result.file_count, 1);
}

#[test]
fn test_kerml_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.kerml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "classifier Vehicle;").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    assert!(result.symbol_count >= 1);
}

#[test]
fn test_mixed_sysml_kerml_directory() {
    let temp_dir = TempDir::new().unwrap();

    let sysml_file = temp_dir.path().join("model.sysml");
    let mut f1 = fs::File::create(&sysml_file).unwrap();
    writeln!(f1, "part def Vehicle;").unwrap();

    let kerml_file = temp_dir.path().join("kernel.kerml");
    let mut f2 = fs::File::create(&kerml_file).unwrap();
    writeln!(f2, "classifier Base;").unwrap();

    let result = run_analysis(temp_dir.path(), false, false, None).unwrap();

    assert_eq!(result.file_count, 2);
    assert!(result.symbol_count >= 2);
}

// ============================================================================
// DIAGNOSTIC TESTS
// ============================================================================

#[test]
fn test_unresolved_type_produces_error() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("unresolved.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "package Test {{").unwrap();
    writeln!(file, "    part p : NonExistentType;").unwrap();
    writeln!(file, "}}").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    assert_eq!(result.file_count, 1);
    // Should have an unresolved type error
    assert!(result.error_count > 0 || result.warning_count > 0);
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn test_cross_file_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // File 1 defines a type
    let file1 = temp_dir.path().join("types.sysml");
    let mut f1 = fs::File::create(&file1).unwrap();
    writeln!(f1, "package Types {{").unwrap();
    writeln!(f1, "    part def Engine;").unwrap();
    writeln!(f1, "}}").unwrap();

    // File 2 uses the type
    let file2 = temp_dir.path().join("vehicle.sysml");
    let mut f2 = fs::File::create(&file2).unwrap();
    writeln!(f2, "package Vehicles {{").unwrap();
    writeln!(f2, "    import Types::*;").unwrap();
    writeln!(f2, "    part def Car {{").unwrap();
    writeln!(f2, "        part engine : Engine;").unwrap();
    writeln!(f2, "    }}").unwrap();
    writeln!(f2, "}}").unwrap();

    let result = run_analysis(temp_dir.path(), false, false, None).unwrap();

    assert_eq!(result.file_count, 2);
    assert!(result.symbol_count >= 4);
}

#[test]
fn test_cross_file_resolution_no_errors() {
    // Test that cross-file type references resolve correctly without errors
    let temp_dir = TempDir::new().unwrap();

    // File 1 defines types in a package
    let file1 = temp_dir.path().join("definitions.sysml");
    let mut f1 = fs::File::create(&file1).unwrap();
    writeln!(f1, "package Definitions {{").unwrap();
    writeln!(f1, "    part def Engine {{").unwrap();
    writeln!(f1, "        attribute power;").unwrap();
    writeln!(f1, "    }}").unwrap();
    writeln!(f1, "    part def Wheel;").unwrap();
    writeln!(f1, "}}").unwrap();

    // File 2 imports and uses types from file 1
    let file2 = temp_dir.path().join("usage.sysml");
    let mut f2 = fs::File::create(&file2).unwrap();
    writeln!(f2, "package Usage {{").unwrap();
    writeln!(f2, "    import Definitions::*;").unwrap();
    writeln!(f2, "    part def Car {{").unwrap();
    writeln!(f2, "        part engine : Engine;").unwrap();
    writeln!(f2, "        part frontLeft : Wheel;").unwrap();
    writeln!(f2, "        part frontRight : Wheel;").unwrap();
    writeln!(f2, "    }}").unwrap();
    writeln!(f2, "}}").unwrap();

    let result = run_analysis(temp_dir.path(), false, false, None).unwrap();

    assert_eq!(result.file_count, 2);
    // Definitions: package, Engine, power, Wheel = 4
    // Usage: package, Car, engine, frontLeft, frontRight = 5
    assert!(result.symbol_count >= 9);
    // Cross-file references should resolve - no errors expected
    assert_eq!(
        result.error_count, 0,
        "Expected no errors but got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_cross_file_specialization_resolves() {
    // Test that specialization (:>) across files resolves correctly
    let temp_dir = TempDir::new().unwrap();

    // File 1 defines a base type
    let file1 = temp_dir.path().join("base.sysml");
    let mut f1 = fs::File::create(&file1).unwrap();
    writeln!(f1, "package Base {{").unwrap();
    writeln!(f1, "    part def Vehicle {{").unwrap();
    writeln!(f1, "        attribute mass;").unwrap();
    writeln!(f1, "    }}").unwrap();
    writeln!(f1, "}}").unwrap();

    // File 2 specializes the base type
    let file2 = temp_dir.path().join("specialized.sysml");
    let mut f2 = fs::File::create(&file2).unwrap();
    writeln!(f2, "package Specialized {{").unwrap();
    writeln!(f2, "    import Base::*;").unwrap();
    writeln!(f2, "    part def Car :> Vehicle {{").unwrap();
    writeln!(f2, "        attribute numDoors;").unwrap();
    writeln!(f2, "    }}").unwrap();
    writeln!(f2, "    part def Truck :> Vehicle {{").unwrap();
    writeln!(f2, "        attribute bedLength;").unwrap();
    writeln!(f2, "    }}").unwrap();
    writeln!(f2, "}}").unwrap();

    let result = run_analysis(temp_dir.path(), false, false, None).unwrap();

    assert_eq!(result.file_count, 2);
    // Specialization of Vehicle should resolve without errors
    assert_eq!(
        result.error_count, 0,
        "Specialization should resolve: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_cross_file_unresolved_produces_error() {
    // Test that unresolved cross-file references produce errors
    let temp_dir = TempDir::new().unwrap();

    // File 1 defines some types
    let file1 = temp_dir.path().join("types.sysml");
    let mut f1 = fs::File::create(&file1).unwrap();
    writeln!(f1, "package Types {{").unwrap();
    writeln!(f1, "    part def Engine;").unwrap();
    writeln!(f1, "}}").unwrap();

    // File 2 references a type that doesn't exist (no import, wrong name)
    let file2 = temp_dir.path().join("broken.sysml");
    let mut f2 = fs::File::create(&file2).unwrap();
    writeln!(f2, "package Broken {{").unwrap();
    writeln!(f2, "    // Missing import - should fail to resolve").unwrap();
    writeln!(f2, "    part car : NonExistentType;").unwrap();
    writeln!(f2, "}}").unwrap();

    let result = run_analysis(temp_dir.path(), false, false, None).unwrap();

    assert_eq!(result.file_count, 2);
    // Should have an error for unresolved type
    assert!(
        result.error_count > 0 || result.warning_count > 0,
        "Expected error for unresolved type reference"
    );
}

#[test]
#[ignore] // Requires local sysml.library - run manually with `cargo test -- --ignored`
fn test_stdlib_type_resolution() {
    // Test that types from stdlib resolve correctly
    // Note: This test loads the full stdlib which takes ~50s
    let temp_dir = TempDir::new().unwrap();

    let file_path = temp_dir.path().join("with_stdlib.sysml");
    let mut file = fs::File::create(&file_path).unwrap();
    // Use types from the standard library
    writeln!(file, "package MyModel {{").unwrap();
    writeln!(file, "    import ScalarValues::*;").unwrap();
    writeln!(file, "    part def Sensor {{").unwrap();
    writeln!(file, "        attribute reading : Real;").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();

    // Load with stdlib enabled
    let result = run_analysis(&file_path, false, true, None).unwrap();

    // Should have loaded our file plus stdlib files
    assert!(result.file_count >= 1);
    // Real from ScalarValues should resolve - no errors for that reference
    // Note: There might be other diagnostics, but the Real type should resolve
    let real_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("Real"))
        .collect();
    assert!(
        real_errors.is_empty(),
        "Real type from stdlib should resolve: {:?}",
        real_errors
    );
}

#[test]
#[ignore] // Requires local sysml.library - run manually with `cargo test -- --ignored`
fn test_stdlib_specialization() {
    // Test that specializing stdlib types works correctly
    let temp_dir = TempDir::new().unwrap();

    let file_path = temp_dir.path().join("stdlib_extend.sysml");
    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "package Extensions {{").unwrap();
    writeln!(file, "    import Parts::*;").unwrap();
    writeln!(file, "    part def MyPart :> Part {{").unwrap();
    writeln!(file, "        attribute name;").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();

    // Load with stdlib enabled
    let result = run_analysis(&file_path, false, true, None).unwrap();

    assert!(result.file_count >= 1);
    // Specializing Part from stdlib should work
    let part_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("Part"))
        .collect();
    assert!(
        part_errors.is_empty(),
        "Part type from stdlib should resolve for specialization: {:?}",
        part_errors
    );
}

#[test]
fn test_diagnostics_have_location() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("error.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "package Test {{").unwrap();
    writeln!(file, "    part p : Unknown;").unwrap();
    writeln!(file, "}}").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    // If there are diagnostics, they should have valid locations
    for diag in &result.diagnostics {
        assert!(diag.line > 0);
        assert!(diag.col > 0);
        assert!(!diag.file.is_empty());
        assert!(!diag.message.is_empty());
    }
}

// ============================================================================
// RESULT STRUCT TESTS
// ============================================================================

#[test]
fn test_result_counts_match_diagnostics() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();

    // Count errors and warnings from diagnostics
    let error_count = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, syster::hir::Severity::Error))
        .count();
    let warning_count = result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, syster::hir::Severity::Warning))
        .count();

    assert_eq!(result.error_count, error_count);
    assert_eq!(result.warning_count, warning_count);
}

// ============================================================================
// EXPORT TESTS
// ============================================================================

#[test]
fn test_export_ast_single_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle {{ attribute mass; }}").unwrap();

    let json = export_ast(&file_path, false, false, None).unwrap();

    // Parse the JSON to verify structure
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(parsed["files"].is_array());
    assert_eq!(parsed["files"].as_array().unwrap().len(), 1);

    let file_entry = &parsed["files"][0];
    assert!(file_entry["path"].as_str().unwrap().contains("test.sysml"));
    assert!(file_entry["symbols"].is_array());

    let symbols = file_entry["symbols"].as_array().unwrap();
    assert!(symbols.len() >= 2); // Vehicle + mass

    // Check Vehicle symbol
    let vehicle = symbols.iter().find(|s| s["name"] == "Vehicle").unwrap();
    assert_eq!(vehicle["kind"], "PartDef");
    assert_eq!(vehicle["qualified_name"], "Vehicle");
}

#[test]
fn test_export_ast_directory() {
    let temp_dir = TempDir::new().unwrap();

    let file1 = temp_dir.path().join("types.sysml");
    let mut f1 = fs::File::create(&file1).unwrap();
    writeln!(f1, "part def Engine;").unwrap();

    let file2 = temp_dir.path().join("vehicle.sysml");
    let mut f2 = fs::File::create(&file2).unwrap();
    writeln!(f2, "part def Car;").unwrap();

    let json = export_ast(temp_dir.path(), false, false, None).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["files"].as_array().unwrap().len(), 2);
}

#[test]
fn test_export_ast_includes_supertypes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();
    writeln!(file, "part def Car :> Vehicle;").unwrap();

    let json = export_ast(&file_path, false, false, None).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let symbols = parsed["files"][0]["symbols"].as_array().unwrap();
    let car = symbols.iter().find(|s| s["name"] == "Car").unwrap();

    assert!(car["supertypes"].is_array());
    let supertypes = car["supertypes"].as_array().unwrap();
    assert!(
        supertypes
            .iter()
            .any(|s| s.as_str().unwrap().contains("Vehicle"))
    );
}

#[test]
fn test_export_ast_includes_location() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    let json = export_ast(&file_path, false, false, None).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let vehicle = &parsed["files"][0]["symbols"][0];

    // Should have location info (1-indexed)
    assert!(vehicle["start_line"].as_u64().unwrap() >= 1);
    assert!(vehicle["start_col"].as_u64().unwrap() >= 1);
    assert!(vehicle["end_line"].as_u64().unwrap() >= 1);
    assert!(vehicle["end_col"].as_u64().unwrap() >= 1);
}

#[test]
fn test_export_json_result() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part def Vehicle;").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();
    let json = export_json(&result).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["file_count"], 1);
    assert!(parsed["symbol_count"].as_u64().unwrap() >= 1);
    assert_eq!(parsed["error_count"], 0);
    assert_eq!(parsed["warning_count"], 0);
    assert!(parsed["diagnostics"].is_array());
}

#[test]
fn test_export_json_with_errors() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part p : UnknownType;").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();
    let json = export_json(&result).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Should have diagnostics
    let diagnostics = parsed["diagnostics"].as_array().unwrap();
    assert!(!diagnostics.is_empty());

    // Check diagnostic structure
    let diag = &diagnostics[0];
    assert!(diag["file"].is_string());
    assert!(diag["line"].is_number());
    assert!(diag["col"].is_number());
    assert!(diag["message"].is_string());
    assert!(diag["severity"].is_string());
}

#[test]
fn test_export_json_diagnostics_have_severity_string() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sysml");

    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "part p : UnknownType;").unwrap();

    let result = run_analysis(&file_path, false, false, None).unwrap();
    let json = export_json(&result).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let diagnostics = parsed["diagnostics"].as_array().unwrap();

    if !diagnostics.is_empty() {
        let severity = diagnostics[0]["severity"].as_str().unwrap();
        // Severity should be a readable string, not a number
        assert!(["error", "warning", "info", "hint"].contains(&severity));
    }
}

#[test]
fn test_export_ast_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.sysml");

    fs::File::create(&file_path).unwrap();

    let json = export_ast(&file_path, false, false, None).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["files"].as_array().unwrap().len(), 1);
    assert!(parsed["files"][0]["symbols"].as_array().unwrap().is_empty());
}

// ============================================================================
// INTERCHANGE EXPORT TESTS
// ============================================================================

#[cfg(feature = "interchange")]
mod interchange_tests {
    use super::*;
    use syster_cli::export_model;

    #[test]
    fn test_export_model_xmi() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.sysml");

        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "package TestPackage;").unwrap();

        let xmi_bytes =
            export_model(&file_path, "xmi", false, false, None, false).expect("Should export XMI");

        // Verify it's valid XML
        let xmi_str = String::from_utf8(xmi_bytes).expect("Should be valid UTF-8");
        assert!(xmi_str.contains("<?xml"), "Should have XML declaration");
        assert!(xmi_str.contains("XMI"), "Should have XMI element");
        assert!(
            xmi_str.contains("TestPackage"),
            "Should contain package name"
        );
    }

    #[test]
    fn test_export_model_kpar() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.sysml");

        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "package TestPackage;").unwrap();

        let kpar_bytes = export_model(&file_path, "kpar", false, false, None, false)
            .expect("Should export KPAR");

        // Verify it starts with ZIP magic number (PK)
        assert!(kpar_bytes.len() > 2, "Should have content");
        assert_eq!(&kpar_bytes[0..2], b"PK", "Should be a ZIP file");
    }

    #[test]
    fn test_export_model_jsonld() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.sysml");

        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "package TestPackage;").unwrap();

        let jsonld_bytes = export_model(&file_path, "jsonld", false, false, None, false)
            .expect("Should export JSON-LD");

        // Verify it's valid JSON
        let jsonld_str = String::from_utf8(jsonld_bytes).expect("Should be valid UTF-8");
        let _parsed: serde_json::Value =
            serde_json::from_str(&jsonld_str).expect("Should be valid JSON");
    }

    #[test]
    fn test_export_model_invalid_format() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.sysml");

        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "package Test;").unwrap();

        let result = export_model(&file_path, "invalid", false, false, None, false);
        assert!(result.is_err(), "Should fail with invalid format");
    }

    #[test]
    fn test_import_model_xmi() {
        use syster_cli::import_model;

        let temp_dir = TempDir::new().unwrap();

        // Create a valid XMI file
        let xmi_path = temp_dir.path().join("test.xmi");
        let xmi_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001" xmlns:sysml="http://www.omg.org/spec/SysML/20230201">
  <sysml:Package xmi:id="TestPackage" name="TestPackage"/>
</xmi:XMI>"#;
        fs::write(&xmi_path, xmi_content).unwrap();

        let result = import_model(&xmi_path, None, false).expect("Should import XMI");

        assert!(result.element_count > 0, "Should have elements");
        assert!(result.error_count == 0, "Should have no errors");
    }

    #[test]
    fn test_import_model_invalid_file() {
        use syster_cli::import_model;

        let temp_dir = TempDir::new().unwrap();
        let invalid_path = temp_dir.path().join("test.xmi");
        fs::write(&invalid_path, "not valid xml").unwrap();

        // Should not panic on invalid input - either error or empty result
        let result = import_model(&invalid_path, None, false);
        // We just want to make sure it doesn't crash
        match result {
            Err(_) => {} // Error is acceptable
            Ok(_r) => {
                // Empty result is also acceptable for invalid XML
                // (the parser is lenient and may just return no elements)
            }
        }
    }

    /// Test that XMI roundtrip preserves element IDs via import/export.
    #[test]
    fn test_xmi_roundtrip_preserves_ids() {
        use syster::ide::AnalysisHost;
        use syster_cli::{export_from_host, import_model_into_host};

        let temp_dir = TempDir::new().unwrap();

        // Create an XMI file with known IDs
        let xmi_path = temp_dir.path().join("original.xmi");
        let original_xmi = r#"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001" xmlns:sysml="http://www.omg.org/spec/SysML/20230201">
  <sysml:Package xmi:id="pkg-uuid-12345" name="TestPkg" qualifiedName="TestPkg">
    <ownedMember>
      <sysml:PartDefinition xmi:id="part-uuid-67890" name="Widget" qualifiedName="TestPkg::Widget"/>
    </ownedMember>
  </sysml:Package>
</xmi:XMI>"#;
        fs::write(&xmi_path, original_xmi).unwrap();

        // Import XMI into host
        let mut host = AnalysisHost::new();
        let import_result =
            import_model_into_host(&mut host, &xmi_path, None, false).expect("Should import XMI");
        assert_eq!(import_result.element_count, 2);

        // Export from host back to XMI
        let roundtrip_xmi =
            export_from_host(&mut host, "xmi", false, true).expect("Should export XMI");

        let roundtrip_str = String::from_utf8(roundtrip_xmi).expect("Should be valid UTF-8");

        // Verify the original IDs are preserved
        assert!(
            roundtrip_str.contains("pkg-uuid-12345"),
            "Package ID should be preserved. Got:\n{}",
            roundtrip_str
        );
        assert!(
            roundtrip_str.contains("part-uuid-67890"),
            "Part ID should be preserved. Got:\n{}",
            roundtrip_str
        );
    }

    /// Test that exporting a file that references stdlib Real type works correctly.
    #[test]
    fn test_export_with_stdlib_real_reference() {
        use std::process::Command;
        use syster_cli::export_model;

        let temp_dir = TempDir::new().unwrap();

        // Create a SysML file that references Real from the stdlib
        let sysml_path = temp_dir.path().join("model.sysml");
        let sysml_content = r#"package TestModel {
    attribute def Temperature :> ScalarValues::Real;
    part def Sensor {
        attribute temp : Temperature;
    }
}"#;
        fs::write(&sysml_path, sysml_content).unwrap();

        // Try to use the local stdlib first, otherwise clone from GitHub
        let local_stdlib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("base/sysml.library");

        let stdlib_dir = if local_stdlib.exists() {
            local_stdlib
        } else {
            // Clone the stdlib from GitHub into the temp directory
            let stdlib_clone_dir = temp_dir.path().join("sysml-release");
            let status = Command::new("git")
                .args([
                    "clone",
                    "--depth=1",
                    "https://github.com/Systems-Modeling/SysML-v2-Release.git",
                    stdlib_clone_dir.to_str().unwrap(),
                ])
                .status()
                .expect("Failed to run git clone");

            if !status.success() {
                panic!("Failed to clone SysML-v2-Release repository");
            }

            stdlib_clone_dir.join("sysml.library")
        };

        // Export with stdlib
        let xmi_bytes = export_model(&sysml_path, "xmi", false, true, Some(&stdlib_dir), false)
            .expect("Should export XMI with stdlib reference");

        let xmi_str = String::from_utf8(xmi_bytes).expect("Should be valid UTF-8");

        // Verify the export contains our model elements
        assert!(
            xmi_str.contains("TestModel"),
            "Should contain TestModel package"
        );
        assert!(
            xmi_str.contains("Temperature"),
            "Should contain Temperature attribute def"
        );
        assert!(xmi_str.contains("Sensor"), "Should contain Sensor part def");
        assert!(xmi_str.contains("temp"), "Should contain temp attribute");

        // Verify it's valid XMI structure
        assert!(
            xmi_str.contains("xmi:version") || xmi_str.contains("xmi:XMI"),
            "Should be valid XMI with version or XMI root"
        );
        assert!(
            xmi_str.contains("xmlns:sysml"),
            "Should have SysML namespace"
        );
    }

    /// Test that exporting to JSON-LD format works with stdlib references.
    #[test]
    fn test_export_jsonld_with_stdlib_reference() {
        use std::process::Command;
        use syster_cli::export_model;

        let temp_dir = TempDir::new().unwrap();

        // Create a SysML file that references Real from the stdlib
        let sysml_path = temp_dir.path().join("model.sysml");
        let sysml_content = r#"package SensorSystem {
    import ScalarValues::*;
    
    attribute def Voltage :> Real;
    
    part def VoltageSensor {
        attribute reading : Voltage;
        attribute maxVoltage : Voltage;
    }
}"#;
        fs::write(&sysml_path, sysml_content).unwrap();

        // Try to use the local stdlib first, otherwise clone from GitHub
        let local_stdlib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("base/sysml.library");

        let stdlib_dir = if local_stdlib.exists() {
            local_stdlib
        } else {
            let stdlib_clone_dir = temp_dir.path().join("sysml-release");
            let status = Command::new("git")
                .args([
                    "clone",
                    "--depth=1",
                    "https://github.com/Systems-Modeling/SysML-v2-Release.git",
                    stdlib_clone_dir.to_str().unwrap(),
                ])
                .status()
                .expect("Failed to run git clone");

            if !status.success() {
                panic!("Failed to clone SysML-v2-Release repository");
            }

            stdlib_clone_dir.join("sysml.library")
        };

        // Export to JSON-LD
        let jsonld_bytes = export_model(
            &sysml_path,
            "json-ld",
            false,
            true,
            Some(&stdlib_dir),
            false,
        )
        .expect("Should export JSON-LD with stdlib reference");

        let jsonld_str = String::from_utf8(jsonld_bytes).expect("Should be valid UTF-8");

        // Print the JSON-LD output for inspection
        println!(
            "=== JSON-LD Output ===\n{}\n=== End JSON-LD ===",
            jsonld_str
        );

        // Verify it's valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&jsonld_str).expect("Should be valid JSON");

        // Verify it has JSON-LD context
        assert!(
            parsed.get("@context").is_some() || jsonld_str.contains("@context"),
            "Should have JSON-LD @context"
        );

        // Verify the export contains our model elements
        assert!(
            jsonld_str.contains("SensorSystem"),
            "Should contain SensorSystem package"
        );
        assert!(
            jsonld_str.contains("Voltage"),
            "Should contain Voltage attribute def"
        );
        assert!(
            jsonld_str.contains("VoltageSensor"),
            "Should contain VoltageSensor part def"
        );
    }

    /// Test that exporting to KPAR (ZIP archive) format works with stdlib references.
    #[test]
    fn test_export_kpar_with_stdlib_reference() {
        use std::process::Command;
        use syster_cli::export_model;

        let temp_dir = TempDir::new().unwrap();

        // Create a SysML file with multiple definitions
        let sysml_path = temp_dir.path().join("model.sysml");
        let sysml_content = r#"package VehicleSystem {
    import ScalarValues::*;
    
    attribute def Speed :> Real;
    attribute def Mass :> Real;
    
    part def Vehicle {
        attribute currentSpeed : Speed;
        attribute totalMass : Mass;
    }
    
    part def Car :> Vehicle {
        attribute numDoors : Integer;
    }
}"#;
        fs::write(&sysml_path, sysml_content).unwrap();

        // Try to use the local stdlib first, otherwise clone from GitHub
        let local_stdlib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("base/sysml.library");

        let stdlib_dir = if local_stdlib.exists() {
            local_stdlib
        } else {
            let stdlib_clone_dir = temp_dir.path().join("sysml-release");
            let status = Command::new("git")
                .args([
                    "clone",
                    "--depth=1",
                    "https://github.com/Systems-Modeling/SysML-v2-Release.git",
                    stdlib_clone_dir.to_str().unwrap(),
                ])
                .status()
                .expect("Failed to run git clone");

            if !status.success() {
                panic!("Failed to clone SysML-v2-Release repository");
            }

            stdlib_clone_dir.join("sysml.library")
        };

        // Export to KPAR
        let kpar_bytes = export_model(&sysml_path, "kpar", false, true, Some(&stdlib_dir), false)
            .expect("Should export KPAR with stdlib reference");

        // Verify it's a valid ZIP file (check magic bytes)
        assert!(kpar_bytes.len() > 4, "Should have content");
        assert_eq!(
            &kpar_bytes[0..2],
            b"PK",
            "Should be a ZIP file (PK magic bytes)"
        );

        // Verify it has reasonable size (should contain XMI data)
        assert!(
            kpar_bytes.len() > 1000,
            "KPAR should contain substantial data"
        );
    }

    /// Test that export filters out stdlib by default, but includes it with self_contained=true.
    ///
    /// Run with: cargo test --features interchange test_export_filters_stdlib -- --nocapture
    #[test]
    fn test_export_filters_stdlib() {
        use std::process::Command;
        use syster_cli::export_model;

        let temp_dir = TempDir::new().unwrap();

        // Create a simple SysML file that uses stdlib types (Real)
        let sysml_path = temp_dir.path().join("model.sysml");
        let sysml_content = r#"package FilterTest {
    import ISQ::*;
    
    attribute def Temperature :> Real;
    part def Sensor {
        attribute temp : Temperature;
    }
}"#;
        fs::write(&sysml_path, sysml_content).unwrap();

        // Try to use the local stdlib first, otherwise clone from GitHub
        let local_stdlib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("base/sysml.library");

        let stdlib_dir = if local_stdlib.exists() {
            local_stdlib
        } else {
            let stdlib_clone_dir = temp_dir.path().join("sysml-release");
            let status = Command::new("git")
                .args([
                    "clone",
                    "--depth=1",
                    "https://github.com/Systems-Modeling/SysML-v2-Release.git",
                    stdlib_clone_dir.to_str().unwrap(),
                ])
                .status()
                .expect("Failed to run git clone");

            if !status.success() {
                panic!("Failed to clone SysML-v2-Release repository");
            }

            stdlib_clone_dir.join("sysml.library")
        };

        // Export WITHOUT self_contained (default) - should NOT include stdlib
        let filtered_xmi = export_model(&sysml_path, "xmi", false, true, Some(&stdlib_dir), false)
            .expect("Should export filtered XMI");

        let filtered_str = String::from_utf8(filtered_xmi.clone()).expect("Should be valid UTF-8");

        // Verify our model elements ARE present
        assert!(
            filtered_str.contains("FilterTest"),
            "Should contain FilterTest package"
        );
        assert!(
            filtered_str.contains("Temperature"),
            "Should contain Temperature"
        );
        assert!(filtered_str.contains("Sensor"), "Should contain Sensor");

        // Verify stdlib elements are NOT present (spot check)
        assert!(
            !filtered_str.contains("ISQSpaceTime"),
            "Should NOT contain ISQSpaceTime (stdlib)"
        );
        assert!(
            !filtered_str.contains("ScalarValues"),
            "Should NOT contain ScalarValues (stdlib)"
        );
        assert!(
            !filtered_str.contains("standard library"),
            "Should NOT contain 'standard library' markers"
        );

        // Export WITH self_contained - should include stdlib
        let full_xmi = export_model(&sysml_path, "xmi", false, true, Some(&stdlib_dir), true)
            .expect("Should export self-contained XMI");

        let full_str = String::from_utf8(full_xmi.clone()).expect("Should be valid UTF-8");

        // Verify stdlib elements ARE present when self_contained
        assert!(
            full_str.contains("FilterTest"),
            "Should contain FilterTest package"
        );

        // The self-contained export should be much larger than the filtered one
        assert!(
            full_xmi.len() > filtered_xmi.len() * 5,
            "Self-contained export ({} bytes) should be much larger than filtered ({} bytes)",
            full_xmi.len(),
            filtered_xmi.len()
        );
    }

    /// Test that exporting to YAML format works correctly.
    #[test]
    fn test_export_yaml() {
        use std::process::Command;
        use syster_cli::export_model;

        let temp_dir = TempDir::new().unwrap();

        // Create a complex SysML file with many element types
        let sysml_path = temp_dir.path().join("model.sysml");
        let sysml_content = r#"package AutomotiveSystem {
    doc /* This is a comprehensive vehicle model 
           demonstrating various SysML v2 features. */
    
    import ScalarValues::*;
    
    // Attribute definitions specializing Real
    attribute def Mass :> Real;
    attribute def Velocity :> Real;
    attribute def Temperature :> Real;
    
    // Enumeration
    enum def EngineState {
        off;
        starting;
        running;
        stopping;
    }
    
    // Port definitions
    port def FuelPort {
        attribute flowRate : Real;
        in attribute fuelIn : Real;
        out attribute fuelOut : Real;
    }
    
    port def ElectricalPort {
        attribute voltage : Real;
        attribute current : Real;
    }
    
    port def MechanicalPort {
        attribute torque : Real;
        attribute rpm : Real;
    }
    
    // Interface definition
    interface def PowerInterface {
        end supplierPort : ElectricalPort;
        end consumerPort : ElectricalPort;
    }
    
    // Part definitions with features
    abstract part def Component {
        attribute mass : Mass;
        attribute serialNumber : String;
    }
    
    part def Cylinder :> Component {
        attribute bore : Real;
        attribute stroke : Real;
        attribute compressionRatio : Real;
    }
    
    part def FuelInjector :> Component {
        attribute sprayAngle : Real;
        port fuelIn : FuelPort;
    }
    
    part def Engine :> Component {
        attribute displacement : Real;
        attribute maxPower : Real;
        attribute state : EngineState;
        
        port fuelIntake : FuelPort;
        port electricalConn : ElectricalPort;
        
        // Nested parts
        part cylinder[4] : Cylinder;
        part fuelInjector[4] : FuelInjector;
        
        // State machine
        state def EngineStates {
            entry state off;
            state starting;
            state running;
            state stopping;
            
            transition off_to_starting
                first off
                then starting;
            
            transition starting_to_running
                first starting
                then running;
            
            transition running_to_stopping
                first running
                then stopping;
            
            transition stopping_to_off
                first stopping
                then off;
        }
    }
    
    part def Transmission :> Component {
        attribute gearCount : Integer;
        attribute currentGear : Integer;
        port inputShaft : MechanicalPort;
        port outputShaft : MechanicalPort;
    }
    
    part def Battery :> Component {
        attribute capacity : Real;
        attribute voltage : Real;
        attribute chargeLevel : Real;
        port powerOut : ElectricalPort;
    }
    
    // Connection definition
    connection def PowerConnection :> PowerInterface {
        end supplierPort : ElectricalPort;
        end consumerPort : ElectricalPort;
    }
    
    // Top-level vehicle assembly
    part def Vehicle :> Component {
        attribute vin : String;
        attribute modelYear : Integer;
        attribute totalMass : Mass;
        attribute currentVelocity : Velocity;
        
        // Major subsystems
        part engine : Engine;
        part transmission : Transmission;
        part battery : Battery;
        
        // Connections between parts
        connection enginePower : PowerConnection
            connect battery.powerOut to engine.electricalConn;
    }
    
    // Use case for vehicle operation
    use case def DriveVehicle {
        subject vehicle : Vehicle;
        
        actor driver;
        
        include use case startEngine;
        include use case accelerate;
        include use case brake;
        include use case stopEngine;
    }
    
    // Requirements
    requirement def SafetyRequirement {
        doc /* All safety requirements for the vehicle */
        
        attribute criticality : String;
    }
    
    requirement vehicleSafety : SafetyRequirement {
        doc /* The vehicle shall meet all applicable safety standards. */
    }
    
    // Analysis case
    analysis def ThermalAnalysis {
        subject vehicle : Vehicle;
        
        return result : Real;
    }
    
    // View and viewpoint
    viewpoint def SystemArchitectViewpoint {
        doc /* Stakeholder view for system architects */
    }
    
    view def VehicleOverview {
        expose Vehicle;
        expose Engine;
        expose Transmission;
    }
}"#;
        fs::write(&sysml_path, sysml_content).unwrap();

        // Get stdlib path
        let local_stdlib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("base/sysml.library");

        let stdlib_dir = if local_stdlib.exists() {
            local_stdlib
        } else {
            let stdlib_clone_dir = temp_dir.path().join("sysml-release");
            let status = Command::new("git")
                .args([
                    "clone",
                    "--depth=1",
                    "https://github.com/Systems-Modeling/SysML-v2-Release.git",
                    stdlib_clone_dir.to_str().unwrap(),
                ])
                .status()
                .expect("Failed to run git clone");

            if !status.success() {
                panic!("Failed to clone SysML-v2-Release repository");
            }

            stdlib_clone_dir.join("sysml.library")
        };

        // Export to YAML (with stdlib loaded but filtered out)
        let yaml_bytes = export_model(&sysml_path, "yaml", false, true, Some(&stdlib_dir), false)
            .expect("Should export to YAML");

        let yaml_str = String::from_utf8(yaml_bytes).expect("Should be valid UTF-8");

        // Print the YAML output for inspection
        println!("=== YAML Output ===\n{}\n=== End YAML ===", yaml_str);

        // Verify YAML structure - basic types
        assert!(yaml_str.contains("'@type': Package"), "Should have Package type");
        assert!(
            yaml_str.contains("'@type': PartDefinition"),
            "Should have PartDefinition type"
        );
        assert!(
            yaml_str.contains("'@type': PortDefinition"),
            "Should have PortDefinition type"
        );
        assert!(
            yaml_str.contains("'@type': AttributeDefinition"),
            "Should have AttributeDefinition type"
        );
        assert!(
            yaml_str.contains("'@type': InterfaceDefinition"),
            "Should have InterfaceDefinition type"
        );
        assert!(
            yaml_str.contains("'@type': ConnectionDefinition"),
            "Should have ConnectionDefinition type"
        );
        assert!(
            yaml_str.contains("'@type': RequirementDefinition"),
            "Should have RequirementDefinition type"
        );
        assert!(
            yaml_str.contains("'@type': UseCaseDefinition"),
            "Should have UseCaseDefinition type"
        );
        assert!(
            yaml_str.contains("'@type': ViewDefinition"),
            "Should have ViewDefinition type"
        );
        assert!(
            yaml_str.contains("'@type': ViewpointDefinition"),
            "Should have ViewpointDefinition type"
        );

        // Verify element names are present
        assert!(
            yaml_str.contains("name: AutomotiveSystem"),
            "Should contain AutomotiveSystem package"
        );
        assert!(
            yaml_str.contains("name: Vehicle"),
            "Should contain Vehicle part def"
        );
        assert!(
            yaml_str.contains("name: Engine"),
            "Should contain Engine part def"
        );
        assert!(
            yaml_str.contains("name: Transmission"),
            "Should contain Transmission part def"
        );
        assert!(
            yaml_str.contains("name: Battery"),
            "Should contain Battery part def"
        );
        assert!(
            yaml_str.contains("name: FuelPort"),
            "Should contain FuelPort port def"
        );
        assert!(
            yaml_str.contains("name: ElectricalPort"),
            "Should contain ElectricalPort port def"
        );
        assert!(
            yaml_str.contains("name: PowerInterface"),
            "Should contain PowerInterface interface def"
        );
        assert!(
            yaml_str.contains("name: DriveVehicle"),
            "Should contain DriveVehicle use case"
        );
        assert!(
            yaml_str.contains("name: SafetyRequirement"),
            "Should contain SafetyRequirement requirement def"
        );

        // Verify ownership structure
        assert!(
            yaml_str.contains("owner:"),
            "Should have owner references"
        );
        assert!(
            yaml_str.contains("ownedMember:"),
            "Should have ownedMember arrays"
        );
        
        // Verify qualified names are present
        assert!(
            yaml_str.contains("qualifiedName: AutomotiveSystem::Vehicle"),
            "Should have qualified name for Vehicle"
        );
        assert!(
            yaml_str.contains("qualifiedName: AutomotiveSystem::Engine"),
            "Should have qualified name for Engine"
        );
        
        // Verify stdlib import is present
        assert!(
            yaml_str.contains("name: ScalarValues"),
            "Should contain ScalarValues import"
        );
        
        // Verify specialization of Real is captured
        // Mass, Velocity, Temperature all specialize Real from ScalarValues
        assert!(
            yaml_str.contains("qualifiedName: AutomotiveSystem::Mass"),
            "Should have Mass attribute def"
        );
        assert!(
            yaml_str.contains("qualifiedName: AutomotiveSystem::Velocity"),
            "Should have Velocity attribute def"
        );
        assert!(
            yaml_str.contains("qualifiedName: AutomotiveSystem::Temperature"),
            "Should have Temperature attribute def"
        );

        // Verify specialization relationships are present (new format uses separate relationship objects)
        assert!(
            yaml_str.contains("'@type': Specialization"),
            "Should have Specialization relationships"
        );

        // Verify it parses back (round-trip)
        use syster::interchange::{ModelFormat, Yaml};
        let model = Yaml
            .read(yaml_str.as_bytes())
            .expect("Should parse YAML back");
        
        // Complex model should have many elements
        assert!(
            model.elements.len() >= 20,
            "Should have at least 20 elements, got {}",
            model.elements.len()
        );
        
        // Print element count by type for inspection
        println!("Total elements in YAML model: {}", model.elements.len());
    }

    /// Test YAML export with stdlib references.
    #[test]
    fn test_export_yaml_with_stdlib_reference() {
        use std::process::Command;
        use syster_cli::export_model;

        let temp_dir = TempDir::new().unwrap();

        // Create a SysML file that references Real from the stdlib
        let sysml_path = temp_dir.path().join("model.sysml");
        let sysml_content = r#"package MeasurementSystem {
    import ScalarValues::*;
    
    attribute def Temperature :> Real;
    
    part def Thermometer {
        attribute currentTemp : Temperature;
    }
}"#;
        fs::write(&sysml_path, sysml_content).unwrap();

        // Try to use the local stdlib
        let local_stdlib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("base/sysml.library");

        let stdlib_dir = if local_stdlib.exists() {
            local_stdlib
        } else {
            let stdlib_clone_dir = temp_dir.path().join("sysml-release");
            let status = Command::new("git")
                .args([
                    "clone",
                    "--depth=1",
                    "https://github.com/Systems-Modeling/SysML-v2-Release.git",
                    stdlib_clone_dir.to_str().unwrap(),
                ])
                .status()
                .expect("Failed to run git clone");

            if !status.success() {
                panic!("Failed to clone SysML-v2-Release repository");
            }

            stdlib_clone_dir.join("sysml.library")
        };

        // Export to YAML (filtered, no stdlib)
        let yaml_bytes = export_model(&sysml_path, "yaml", false, true, Some(&stdlib_dir), false)
            .expect("Should export YAML with stdlib reference");

        let yaml_str = String::from_utf8(yaml_bytes).expect("Should be valid UTF-8");

        println!(
            "=== YAML with stdlib ref ===\n{}\n=== End YAML ===",
            yaml_str
        );

        // Verify user elements are present
        assert!(
            yaml_str.contains("MeasurementSystem"),
            "Should contain MeasurementSystem package"
        );
        assert!(
            yaml_str.contains("Thermometer"),
            "Should contain Thermometer part def"
        );
        assert!(
            yaml_str.contains("Temperature"),
            "Should contain Temperature attribute def"
        );

        // Verify stdlib package definitions are NOT included (only the import reference)
        // The import itself will contain "ScalarValues" in its name, but the
        // actual ScalarValues package from stdlib should not be exported
        assert!(
            !yaml_str.contains("'@type': LibraryPackage"),
            "Should NOT contain LibraryPackage (stdlib)"
        );

        // Count element types - should only have user elements
        let part_def_count = yaml_str.matches("'@type': PartDefinition").count();
        let attr_def_count = yaml_str.matches("'@type': AttributeDefinition").count();
        assert_eq!(part_def_count, 1, "Should have exactly 1 PartDefinition");
        assert_eq!(
            attr_def_count, 1,
            "Should have exactly 1 AttributeDefinition"
        );
    }
}
