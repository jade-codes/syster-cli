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
            export_model(&file_path, "xmi", false, false, None).expect("Should export XMI");

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

        let kpar_bytes =
            export_model(&file_path, "kpar", false, false, None).expect("Should export KPAR");

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

        let jsonld_bytes =
            export_model(&file_path, "jsonld", false, false, None).expect("Should export JSON-LD");

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

        let result = export_model(&file_path, "invalid", false, false, None);
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
}
