//! Integration tests for CLI import-workspace command.
//!
//! Tests that the `--import-workspace` flag properly loads XMI/KPAR files
//! into the analysis workspace with preserved element IDs.

#[cfg(feature = "interchange")]
#[test]
fn test_cli_import_workspace_xmi() {
    use std::fs;
    use std::process::Command;
    use syster::interchange::{Element, ElementId, ElementKind, Model, ModelFormat, Xmi};
    use tempfile::TempDir;

    // Create a simple XMI file
    let temp_dir = TempDir::new().unwrap();
    let xmi_path = temp_dir.path().join("test_model.xmi");

    let mut model = Model::new();
    let pkg = Element::new(ElementId::new("xmi-pkg-001"), ElementKind::Package)
        .with_name("TestPackage")
        .with_qualified_name("TestPackage");
    model.add_element(pkg);

    let part = Element::new(ElementId::new("xmi-part-001"), ElementKind::PartDefinition)
        .with_name("TestPart")
        .with_qualified_name("TestPackage::TestPart")
        .with_owner(ElementId::new("xmi-pkg-001"));
    model.add_element(part);

    // Write XMI file
    let xmi_bytes = Xmi.write(&model).expect("Should write XMI");
    fs::write(&xmi_path, xmi_bytes).expect("Should write file");

    // Run CLI command
    let output = Command::new(env!("CARGO_BIN_EXE_syster"))
        .arg("--import-workspace")
        .arg(&xmi_path)
        .arg("--no-stdlib") // Skip stdlib for faster test
        .output()
        .expect("Should run CLI");

    // Check output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    assert!(output.status.success(), "CLI should exit successfully");
    assert!(
        stdout.contains("Imported"),
        "Should report imported elements"
    );
    assert!(
        stdout.contains("symbols") && stdout.contains("workspace"),
        "Should mention symbols and workspace"
    );
    assert!(
        stdout.contains("Element IDs preserved"),
        "Should preserve element IDs"
    );
}

#[cfg(feature = "interchange")]
#[test]
fn test_cli_import_workspace_with_stdlib() {
    use std::fs;
    use std::process::Command;
    use syster::interchange::{Element, ElementId, ElementKind, Model, ModelFormat, Xmi};
    use tempfile::TempDir;

    // Create a simple XMI file
    let temp_dir = TempDir::new().unwrap();
    let xmi_path = temp_dir.path().join("model.xmi");

    let mut model = Model::new();
    let pkg = Element::new(ElementId::new("pkg-1"), ElementKind::Package)
        .with_name("MyModel")
        .with_qualified_name("MyModel");
    model.add_element(pkg);

    let xmi_bytes = Xmi.write(&model).expect("Should write XMI");
    fs::write(&xmi_path, xmi_bytes).expect("Should write file");

    // Run with stdlib enabled (default)
    let output = Command::new(env!("CARGO_BIN_EXE_syster"))
        .arg("--import-workspace")
        .arg(&xmi_path)
        .arg("--verbose")
        .output()
        .expect("Should run CLI");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "CLI should succeed with stdlib");
    assert!(
        stdout.contains("Imported 1 elements"),
        "Should import model"
    );
    assert!(
        stdout.contains("Total symbols in workspace"),
        "Should show workspace stats"
    );
}

#[cfg(feature = "interchange")]
#[test]
fn test_cli_import_vs_import_workspace() {
    use std::fs;
    use std::process::Command;
    use syster::interchange::{Element, ElementId, ElementKind, Model, ModelFormat, Xmi};
    use tempfile::TempDir;

    // Create XMI file
    let temp_dir = TempDir::new().unwrap();
    let xmi_path = temp_dir.path().join("test.xmi");

    let mut model = Model::new();
    let pkg = Element::new(ElementId::new("id-1"), ElementKind::Package)
        .with_name("Pkg")
        .with_qualified_name("Pkg");
    model.add_element(pkg);

    let xmi_bytes = Xmi.write(&model).expect("Should write XMI");
    fs::write(&xmi_path, xmi_bytes).expect("Should write file");

    // Test --import (validation only)
    let output_import = Command::new(env!("CARGO_BIN_EXE_syster"))
        .arg("--import")
        .arg(&xmi_path)
        .output()
        .expect("Should run --import");

    let stdout_import = String::from_utf8_lossy(&output_import.stdout);
    assert!(output_import.status.success());
    assert!(stdout_import.contains("Imported 1 elements"));
    assert!(
        !stdout_import.contains("workspace"),
        "--import should not mention workspace"
    );

    // Test --import-workspace (load into workspace)
    let output_workspace = Command::new(env!("CARGO_BIN_EXE_syster"))
        .arg("--import-workspace")
        .arg(&xmi_path)
        .arg("--no-stdlib")
        .output()
        .expect("Should run --import-workspace");

    let stdout_workspace = String::from_utf8_lossy(&output_workspace.stdout);
    assert!(output_workspace.status.success());
    assert!(
        stdout_workspace.contains("workspace"),
        "--import-workspace should mention workspace"
    );
    assert!(stdout_workspace.contains("Element IDs preserved"));
}
