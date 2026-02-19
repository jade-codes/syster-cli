//! Integration tests for CLI semantic model commands:
//! `--query`, `--list`, `--inspect`, `--rename`, `--add-member`, `--remove-member`,
//! `--import`, `--import-workspace`, and metadata round-trip.

#[cfg(feature = "interchange")]
mod semantic_commands {
    use std::fs;
    use std::io::Write;
    use syster_cli::{add_member, inspect_element, query_model, remove_member, rename_element};
    use tempfile::TempDir;

    /// Helper: write a SysML file with a standard vehicle model.
    fn write_vehicle_model(dir: &std::path::Path) -> std::path::PathBuf {
        let path = dir.join("model.sysml");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"package VehicleModel {{
    part def Vehicle {{
        attribute mass : Real;
    }}
    part def Engine {{
        attribute horsepower : Real;
    }}
    part def Wheel;
    part def Car :> Vehicle {{
        part engine : Engine;
        part wheels : Wheel[4];
    }}
    part myCar : Car;
}}"#
        )
        .unwrap();
        path
    }

    // ========================================================================
    // QUERY / LIST
    // ========================================================================

    #[test]
    fn test_list_all_elements() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = query_model(&path, None, None, None, false).unwrap();

        // Should find the package + 4 definitions + 3 usages + 2 attributes = 10
        assert!(
            result.match_count >= 8,
            "Expected at least 8 elements, got {}",
            result.match_count
        );

        let names: Vec<_> = result
            .elements
            .iter()
            .filter_map(|e| e.name.as_deref())
            .collect();
        assert!(names.contains(&"VehicleModel"), "Should contain package");
        assert!(names.contains(&"Vehicle"), "Should contain Vehicle");
        assert!(names.contains(&"Car"), "Should contain Car");
        assert!(names.contains(&"myCar"), "Should contain myCar");
    }

    #[test]
    fn test_query_by_name_substring() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = query_model(&path, Some("Car"), None, None, false).unwrap();

        // "Car" appears in: Car (definition) and myCar (usage)
        assert_eq!(
            result.match_count, 2,
            "Should find 2 elements matching 'Car'"
        );
        let names: Vec<_> = result
            .elements
            .iter()
            .filter_map(|e| e.name.as_deref())
            .collect();
        assert!(names.contains(&"Car"));
        assert!(names.contains(&"myCar"));
    }

    #[test]
    fn test_query_by_name_no_match() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = query_model(&path, Some("Nonexistent"), None, None, false).unwrap();
        assert_eq!(result.match_count, 0);
    }

    #[test]
    fn test_query_filter_by_kind() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = query_model(&path, None, Some("PartDefinition"), None, false).unwrap();

        assert_eq!(
            result.match_count, 4,
            "Should find 4 PartDefinitions (Vehicle, Engine, Wheel, Car)"
        );
        for el in &result.elements {
            assert_eq!(el.kind, "PartDefinition");
        }
    }

    #[test]
    fn test_query_name_and_kind_combined() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = query_model(&path, Some("engine"), Some("PartUsage"), None, false).unwrap();

        assert_eq!(result.match_count, 1);
        assert_eq!(result.elements[0].name.as_deref(), Some("engine"));
        assert_eq!(result.elements[0].kind, "PartUsage");
    }

    // ========================================================================
    // INSPECT
    // ========================================================================

    #[test]
    fn test_inspect_by_name() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = inspect_element(&path, "Car", false).unwrap();

        assert_eq!(result.element.name.as_deref(), Some("Car"));
        assert_eq!(result.element.kind, "PartDefinition");
        assert_eq!(
            result.element.qualified_name.as_deref(),
            Some("VehicleModel::Car")
        );
        assert_eq!(result.element.owner.as_deref(), Some("VehicleModel"));

        // Car has supertypes = [Vehicle]
        assert!(
            result.element.supertypes.contains(&"Vehicle".to_string()),
            "Car should specialize Vehicle"
        );

        // Car has 2 children: engine, wheels
        assert_eq!(result.children.len(), 2, "Car should have 2 children");
        let child_names: Vec<_> = result
            .children
            .iter()
            .filter_map(|c| c.name.as_deref())
            .collect();
        assert!(child_names.contains(&"engine"));
        assert!(child_names.contains(&"wheels"));
    }

    #[test]
    fn test_inspect_by_qualified_name() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = inspect_element(&path, "VehicleModel::Car", false).unwrap();

        assert_eq!(result.element.name.as_deref(), Some("Car"));
        assert_eq!(
            result.element.qualified_name.as_deref(),
            Some("VehicleModel::Car")
        );
    }

    #[test]
    fn test_inspect_relationships() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = inspect_element(&path, "Car", false).unwrap();

        // Car -> Vehicle (Specialization)
        assert!(
            !result.relationships_from.is_empty(),
            "Car should have outgoing relationships"
        );
        let spec = result
            .relationships_from
            .iter()
            .find(|r| r.kind == "Specialization");
        assert!(spec.is_some(), "Car should have a Specialization");
        assert_eq!(spec.unwrap().target, "Vehicle");

        // myCar -> Car (FeatureTyping)
        assert!(
            !result.relationships_to.is_empty(),
            "Car should have incoming relationships"
        );
        let typing = result
            .relationships_to
            .iter()
            .find(|r| r.kind == "FeatureTyping");
        assert!(typing.is_some(), "Car should be typed-to by myCar");
        assert_eq!(typing.unwrap().source, "myCar");
    }

    #[test]
    fn test_inspect_not_found() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = inspect_element(&path, "Nonexistent", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_inspect_json_serializable() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = inspect_element(&path, "Car", false).unwrap();

        // Verify it serializes to valid JSON
        let json = serde_json::to_string_pretty(&result).unwrap();
        assert!(json.contains("\"name\": \"Car\""));
        assert!(json.contains("\"kind\": \"PartDefinition\""));
        assert!(json.contains("\"qualified_name\": \"VehicleModel::Car\""));
    }

    // ========================================================================
    // RENAME
    // ========================================================================

    #[test]
    fn test_rename_definition() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = rename_element(&path, "Vehicle", "Automobile", false).unwrap();

        assert_eq!(result.old_name, "Vehicle");
        assert_eq!(result.new_name, "Automobile");
        assert!(
            result.rendered_text.contains("part def Automobile"),
            "Definition should be renamed"
        );
        // The specialization reference should also update
        assert!(
            result.rendered_text.contains("Automobile"),
            "References should be updated"
        );
    }

    #[test]
    fn test_rename_produces_valid_sysml() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = rename_element(&path, "Vehicle", "Automobile", false).unwrap();

        // Write renamed text and re-parse to verify it's valid SysML
        let renamed_path = dir.path().join("renamed.sysml");
        fs::write(&renamed_path, &result.rendered_text).unwrap();

        let query_result =
            query_model(&renamed_path, Some("Automobile"), None, None, false).unwrap();
        assert!(
            query_result.match_count >= 1,
            "Renamed element should be findable"
        );
    }

    #[test]
    fn test_rename_not_found() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = rename_element(&path, "Nonexistent", "NewName", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    // ========================================================================
    // METADATA ROUND-TRIP (ID preservation)
    // ========================================================================

    #[test]
    fn test_query_loads_companion_metadata() {
        use syster::interchange::metadata::{ElementMeta, ImportMetadata, SourceInfo};

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Create a companion metadata.json with known IDs
        let mut metadata = ImportMetadata::new().with_source(SourceInfo::from_path("test.xmi"));
        metadata.add_element(
            "VehicleModel::Car",
            ElementMeta::with_id("stable-id-car-001"),
        );
        metadata.add_element(
            "VehicleModel::Vehicle",
            ElementMeta::with_id("stable-id-vehicle-001"),
        );

        let metadata_json = serde_json::to_string_pretty(&metadata).unwrap();
        let metadata_path = path.with_extension("metadata.json");
        fs::write(&metadata_path, &metadata_json).unwrap();

        // Query — should pick up the metadata IDs
        let result = query_model(&path, Some("Car"), None, None, false).unwrap();

        let car = result
            .elements
            .iter()
            .find(|e| e.name.as_deref() == Some("Car"))
            .unwrap();
        assert_eq!(
            car.id, "stable-id-car-001",
            "Car should have the ID from metadata.json"
        );
    }

    #[test]
    fn test_inspect_loads_companion_metadata() {
        use syster::interchange::metadata::{ElementMeta, ImportMetadata, SourceInfo};

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Create companion metadata
        let mut metadata = ImportMetadata::new().with_source(SourceInfo::from_path("test.xmi"));
        metadata.add_element(
            "VehicleModel::Car",
            ElementMeta::with_id("inspect-id-car-999"),
        );

        let metadata_json = serde_json::to_string_pretty(&metadata).unwrap();
        fs::write(path.with_extension("metadata.json"), &metadata_json).unwrap();

        let result = inspect_element(&path, "Car", false).unwrap();
        assert_eq!(
            result.element.id, "inspect-id-car-999",
            "Inspect should use metadata ID"
        );
    }

    #[test]
    fn test_rename_preserves_metadata_ids() {
        use syster::interchange::metadata::{ElementMeta, ImportMetadata, SourceInfo};

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Create companion metadata with a known ID for Vehicle
        let mut metadata = ImportMetadata::new().with_source(SourceInfo::from_path("test.xmi"));
        metadata.add_element(
            "VehicleModel::Vehicle",
            ElementMeta::with_id("stable-vehicle-uuid"),
        );
        metadata.add_element("VehicleModel::Car", ElementMeta::with_id("stable-car-uuid"));

        let metadata_json = serde_json::to_string_pretty(&metadata).unwrap();
        fs::write(path.with_extension("metadata.json"), &metadata_json).unwrap();

        // Rename Vehicle -> Automobile
        let result = rename_element(&path, "Vehicle", "Automobile", false).unwrap();

        // The renamed result should include updated metadata
        assert!(
            result.metadata_json.is_some(),
            "Rename should produce metadata JSON"
        );

        let updated_meta: ImportMetadata =
            serde_json::from_str(result.metadata_json.as_ref().unwrap()).unwrap();

        // The old key "VehicleModel::Vehicle" should be gone (now "VehicleModel::Automobile")
        assert!(
            updated_meta.get_element("VehicleModel::Vehicle").is_none(),
            "Old qualified name should be gone"
        );

        // Automobile should have Vehicle's original ID
        let automobile = updated_meta.get_element("VehicleModel::Automobile");
        assert!(
            automobile.is_some(),
            "Renamed element should be in metadata under new qualified name"
        );
        assert_eq!(
            automobile.unwrap().original_id.as_deref(),
            Some("stable-vehicle-uuid"),
            "Renamed element should preserve the original UUID"
        );

        // Car should still have its original ID
        let car = updated_meta.get_element("VehicleModel::Car");
        assert!(car.is_some(), "Untouched elements should remain");
        assert_eq!(
            car.unwrap().original_id.as_deref(),
            Some("stable-car-uuid"),
            "Untouched element should keep its UUID"
        );
    }

    #[test]
    fn test_rename_then_inspect_preserves_id() {
        use syster::interchange::metadata::{ElementMeta, ImportMetadata, SourceInfo};

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Set up metadata
        let mut metadata = ImportMetadata::new().with_source(SourceInfo::from_path("test.xmi"));
        metadata.add_element(
            "VehicleModel::Car",
            ElementMeta::with_id("round-trip-car-id"),
        );
        let metadata_json = serde_json::to_string_pretty(&metadata).unwrap();
        fs::write(path.with_extension("metadata.json"), &metadata_json).unwrap();

        // Rename Car -> SportsCar
        let result = rename_element(&path, "Car", "SportsCar", false).unwrap();

        // Write renamed SysML + metadata to new files
        let renamed_path = dir.path().join("renamed.sysml");
        fs::write(&renamed_path, &result.rendered_text).unwrap();
        fs::write(
            renamed_path.with_extension("metadata.json"),
            result.metadata_json.as_ref().unwrap(),
        )
        .unwrap();

        // Inspect SportsCar from the renamed file — should have the same ID
        let inspect_result = inspect_element(&renamed_path, "SportsCar", false).unwrap();
        assert_eq!(
            inspect_result.element.id, "round-trip-car-id",
            "ID should survive rename -> write -> re-inspect round-trip"
        );
    }

    #[test]
    fn test_no_metadata_file_still_works() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // No metadata file — should still work, just with generated IDs
        let result = query_model(&path, Some("Car"), None, None, false).unwrap();
        assert_eq!(result.match_count, 2);

        let result = inspect_element(&path, "Car", false).unwrap();
        assert_eq!(result.element.name.as_deref(), Some("Car"));

        let result = rename_element(&path, "Vehicle", "Automobile", false).unwrap();
        assert!(result.rendered_text.contains("Automobile"));

        // Without metadata input, rename should still produce metadata output
        assert!(result.metadata_json.is_some());
    }

    // ========================================================================
    // CLI BINARY TESTS (end-to-end via std::process::Command)
    // ========================================================================

    #[test]
    fn test_cli_list_command() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--list"])
            .arg(&path)
            .output()
            .expect("Should run CLI");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "CLI should succeed");
        assert!(stdout.contains("element(s)"), "Should report element count");
        assert!(stdout.contains("Car"), "Should list Car");
        assert!(stdout.contains("Vehicle"), "Should list Vehicle");
    }

    #[test]
    fn test_cli_query_command() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--query", "Car"])
            .arg(&path)
            .output()
            .expect("Should run CLI");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success());
        assert!(stdout.contains("Found 2 element(s)"));
    }

    #[test]
    fn test_cli_inspect_command() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--inspect", "Car"])
            .arg(&path)
            .output()
            .expect("Should run CLI");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success());
        assert!(stdout.contains("PartDefinition Car"));
        assert!(stdout.contains("specializes: Vehicle"));
        assert!(stdout.contains("Children"));
    }

    #[test]
    fn test_cli_inspect_json() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--inspect", "Car", "--json"])
            .arg(&path)
            .output()
            .expect("Should run CLI");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success());

        // Should be valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&stdout).expect("Should be valid JSON");
        assert_eq!(parsed["element"]["name"], "Car");
        assert_eq!(parsed["element"]["kind"], "PartDefinition");
    }

    #[test]
    fn test_cli_rename_command() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());
        let output_path = dir.path().join("renamed.sysml");

        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--rename", "Vehicle=Automobile", "-o"])
            .arg(&output_path)
            .arg(&path)
            .output()
            .expect("Should run CLI");

        let _stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "CLI should succeed: {}", stderr);
        assert!(stderr.contains("Renamed 'Vehicle' -> 'Automobile'"));

        // Verify the output file
        let renamed_text = fs::read_to_string(&output_path).unwrap();
        assert!(renamed_text.contains("part def Automobile"));

        // Verify metadata file was created alongside
        let metadata_path = output_path.with_extension("metadata.json");
        assert!(
            metadata_path.exists(),
            "Metadata JSON should be written alongside renamed SysML"
        );
    }

    #[test]
    fn test_cli_rename_metadata_round_trip() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Step 1: Get Car's initial ID via inspect --json
        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--inspect", "Car", "--json"])
            .arg(&path)
            .output()
            .expect("Should run");
        let initial: serde_json::Value =
            serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
        let _initial_id = initial["element"]["id"].as_str().unwrap();

        // Step 2: Rename and capture the generated metadata
        let renamed_path = dir.path().join("step2.sysml");
        let _output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--rename", "Car=SportsCar", "-o"])
            .arg(&renamed_path)
            .arg(&path)
            .output()
            .expect("Should run");

        // Step 3: Inspect SportsCar from the renamed file
        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--inspect", "SportsCar", "--json"])
            .arg(&renamed_path)
            .output()
            .expect("Should run");
        let after: serde_json::Value =
            serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
        let after_id = after["element"]["id"].as_str().unwrap();

        // The metadata file should exist and map SportsCar to an ID
        let metadata_path = renamed_path.with_extension("metadata.json");
        assert!(metadata_path.exists(), "Metadata should exist");

        let meta_text = fs::read_to_string(&metadata_path).unwrap();
        let meta: serde_json::Value = serde_json::from_str(&meta_text).unwrap();

        // SportsCar's ID in the metadata should match the inspect result
        let meta_id = meta["elements"]["VehicleModel::SportsCar"]["originalId"]
            .as_str()
            .unwrap();
        assert_eq!(
            after_id, meta_id,
            "Inspect ID should match metadata originalId"
        );
    }

    // ========================================================================
    // ADD MEMBER
    // ========================================================================

    #[test]
    fn test_add_member_part_usage() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = add_member(&path, "Car", "turbo", "PartUsage", None, false).unwrap();

        assert_eq!(result.parent_name, "Car");
        assert_eq!(result.member_name, "turbo");
        assert!(!result.member_id.is_empty(), "Should have a generated ID");
        assert!(
            result.rendered_text.contains("turbo"),
            "Rendered text should contain new member"
        );
    }

    #[test]
    fn test_add_member_with_type() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = add_member(&path, "Car", "turbo", "PartUsage", Some("Engine"), false).unwrap();

        assert!(
            result.rendered_text.contains("turbo"),
            "Should contain new member name"
        );
    }

    #[test]
    fn test_add_member_preserves_existing() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = add_member(&path, "Car", "turbo", "PartUsage", None, false).unwrap();

        // Write output and reparse to verify existing members survive
        let out_path = dir.path().join("added.sysml");
        fs::write(&out_path, &result.rendered_text).unwrap();

        let inspect = inspect_element(&out_path, "Car", false).unwrap();

        let child_names: Vec<_> = inspect
            .children
            .iter()
            .filter_map(|c| c.name.as_deref())
            .collect();
        assert!(child_names.contains(&"engine"), "engine should still exist");
        assert!(child_names.contains(&"wheels"), "wheels should still exist");
        assert!(child_names.contains(&"turbo"), "turbo should be added");
        assert_eq!(inspect.children.len(), 3, "Should have 3 children now");
    }

    #[test]
    fn test_add_member_parent_not_found() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = add_member(&path, "Nonexistent", "foo", "PartUsage", None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_add_member_invalid_kind() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = add_member(&path, "Car", "foo", "InvalidKind", None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown element kind"));
    }

    #[test]
    fn test_add_member_metadata_preserves_ids() {
        use syster::interchange::metadata::{ElementMeta, ImportMetadata, SourceInfo};

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Write companion metadata with known IDs
        let mut metadata = ImportMetadata::new().with_source(SourceInfo::from_path("test.xmi"));
        metadata.add_element(
            "VehicleModel::Car",
            ElementMeta::with_id("stable-car-add-test"),
        );
        metadata.add_element(
            "VehicleModel::Engine",
            ElementMeta::with_id("stable-engine-add-test"),
        );
        fs::write(
            path.with_extension("metadata.json"),
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
        .unwrap();

        // Add a new member
        let result = add_member(&path, "Car", "turbo", "PartUsage", None, false).unwrap();

        // Verify metadata is produced
        assert!(result.metadata_json.is_some());
        let updated: ImportMetadata =
            serde_json::from_str(result.metadata_json.as_ref().unwrap()).unwrap();

        // Car should keep its original ID
        let car = updated.get_element("VehicleModel::Car");
        assert!(car.is_some());
        assert_eq!(
            car.unwrap().original_id.as_deref(),
            Some("stable-car-add-test"),
            "Car ID should be preserved after adding a member"
        );

        // Engine should keep its original ID
        let engine = updated.get_element("VehicleModel::Engine");
        assert!(engine.is_some());
        assert_eq!(
            engine.unwrap().original_id.as_deref(),
            Some("stable-engine-add-test"),
            "Engine ID should be preserved"
        );

        // New turbo should have a new ID in the metadata
        let turbo = updated.get_element("VehicleModel::Car::turbo");
        assert!(turbo.is_some(), "New member should appear in metadata");
        assert!(
            turbo.unwrap().original_id.is_some(),
            "New member should have an ID"
        );
    }

    #[test]
    fn test_add_then_inspect_round_trip() {
        use syster::interchange::metadata::{ElementMeta, ImportMetadata, SourceInfo};

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Set up metadata
        let mut metadata = ImportMetadata::new().with_source(SourceInfo::from_path("test.xmi"));
        metadata.add_element(
            "VehicleModel::Car",
            ElementMeta::with_id("car-roundtrip-id"),
        );
        fs::write(
            path.with_extension("metadata.json"),
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
        .unwrap();

        // Add a member
        let result = add_member(&path, "Car", "turbo", "PartUsage", None, false).unwrap();

        // Write both files
        let out_path = dir.path().join("added.sysml");
        fs::write(&out_path, &result.rendered_text).unwrap();
        fs::write(
            out_path.with_extension("metadata.json"),
            result.metadata_json.as_ref().unwrap(),
        )
        .unwrap();

        // Inspect Car — should still have its original ID
        let inspect = inspect_element(&out_path, "Car", false).unwrap();
        assert_eq!(
            inspect.element.id, "car-roundtrip-id",
            "Car ID should survive add-member → write → inspect round-trip"
        );
    }

    // ========================================================================
    // REMOVE MEMBER
    // ========================================================================

    #[test]
    fn test_remove_member() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = remove_member(&path, "VehicleModel::Car::wheels", false).unwrap();

        assert_eq!(result.removed_name, "wheels");
        assert!(
            !result.rendered_text.contains("wheels"),
            "wheels should be removed from output"
        );
        // engine should still be there
        assert!(
            result.rendered_text.contains("engine"),
            "engine should survive removal"
        );
    }

    #[test]
    fn test_remove_by_simple_name() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Remove Wheel (by simple name)
        let result = remove_member(&path, "Wheel", false).unwrap();
        assert_eq!(result.removed_name, "Wheel");
    }

    #[test]
    fn test_remove_not_found() {
        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        let result = remove_member(&path, "Nonexistent", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_remove_preserves_sibling_ids() {
        use syster::interchange::metadata::{ElementMeta, ImportMetadata, SourceInfo};

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Set up metadata for engine (sibling of wheels)
        let mut metadata = ImportMetadata::new().with_source(SourceInfo::from_path("test.xmi"));
        metadata.add_element(
            "VehicleModel::Car::engine",
            ElementMeta::with_id("stable-engine-id"),
        );
        metadata.add_element(
            "VehicleModel::Car::wheels",
            ElementMeta::with_id("stable-wheels-id"),
        );
        metadata.add_element("VehicleModel::Car", ElementMeta::with_id("stable-car-id"));
        fs::write(
            path.with_extension("metadata.json"),
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
        .unwrap();

        // Remove wheels
        let result = remove_member(&path, "VehicleModel::Car::wheels", false).unwrap();
        let updated: ImportMetadata =
            serde_json::from_str(result.metadata_json.as_ref().unwrap()).unwrap();

        // Engine should still have its ID
        let engine = updated.get_element("VehicleModel::Car::engine");
        assert!(engine.is_some());
        assert_eq!(
            engine.unwrap().original_id.as_deref(),
            Some("stable-engine-id"),
            "Sibling engine should keep its ID after removing wheels"
        );

        // Car should still have its ID
        let car = updated.get_element("VehicleModel::Car");
        assert!(car.is_some());
        assert_eq!(
            car.unwrap().original_id.as_deref(),
            Some("stable-car-id"),
            "Parent Car should keep its ID after removing a child"
        );

        // Wheels should be gone from metadata
        assert!(
            updated.get_element("VehicleModel::Car::wheels").is_none(),
            "Removed element should not appear in metadata"
        );
    }

    #[test]
    fn test_remove_then_add_preserves_ids() {
        use syster::interchange::metadata::{ElementMeta, ImportMetadata, SourceInfo};

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Set up metadata
        let mut metadata = ImportMetadata::new().with_source(SourceInfo::from_path("test.xmi"));
        metadata.add_element("VehicleModel::Car", ElementMeta::with_id("car-combo-id"));
        metadata.add_element(
            "VehicleModel::Car::engine",
            ElementMeta::with_id("engine-combo-id"),
        );
        fs::write(
            path.with_extension("metadata.json"),
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
        .unwrap();

        // Step 1: Remove wheels
        let result = remove_member(&path, "VehicleModel::Car::wheels", false).unwrap();
        let step1_path = dir.path().join("step1.sysml");
        fs::write(&step1_path, &result.rendered_text).unwrap();
        fs::write(
            step1_path.with_extension("metadata.json"),
            result.metadata_json.as_ref().unwrap(),
        )
        .unwrap();

        // Step 2: Add turbo
        let result = add_member(&step1_path, "Car", "turbo", "PartUsage", None, false).unwrap();
        let step2_path = dir.path().join("step2.sysml");
        fs::write(&step2_path, &result.rendered_text).unwrap();
        fs::write(
            step2_path.with_extension("metadata.json"),
            result.metadata_json.as_ref().unwrap(),
        )
        .unwrap();

        // Verify: Car and engine IDs preserved through both operations
        let inspect = inspect_element(&step2_path, "Car", false).unwrap();
        assert_eq!(
            inspect.element.id, "car-combo-id",
            "Car ID should survive remove + add combo"
        );

        let child_names: Vec<_> = inspect
            .children
            .iter()
            .filter_map(|c| c.name.as_deref())
            .collect();
        assert!(child_names.contains(&"engine"), "engine should survive");
        assert!(child_names.contains(&"turbo"), "turbo should be added");
        assert!(!child_names.contains(&"wheels"), "wheels should be gone");

        // Engine should still have its stable ID
        let engine_inspect =
            inspect_element(&step2_path, "VehicleModel::Car::engine", false).unwrap();
        assert_eq!(
            engine_inspect.element.id, "engine-combo-id",
            "Engine ID should survive remove + add combo"
        );
    }

    // ========================================================================
    // CLI BINARY TESTS for add-member and remove
    // ========================================================================

    #[test]
    fn test_cli_add_member_command() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());
        let output_path = dir.path().join("added.sysml");

        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args([
                "--no-stdlib",
                "--add-member",
                "Car:PartUsage:turbo:Engine",
                "-o",
            ])
            .arg(&output_path)
            .arg(&path)
            .output()
            .expect("Should run CLI");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "CLI should succeed: {}", stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Added PartUsage 'turbo' to 'Car'"));

        let text = fs::read_to_string(&output_path).unwrap();
        assert!(text.contains("turbo"));

        // Metadata should be created
        assert!(output_path.with_extension("metadata.json").exists());
    }

    #[test]
    fn test_cli_remove_command() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());
        let output_path = dir.path().join("removed.sysml");

        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args([
                "--no-stdlib",
                "--remove-member",
                "VehicleModel::Car::wheels",
                "-o",
            ])
            .arg(&output_path)
            .arg(&path)
            .output()
            .expect("Should run CLI");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "CLI should succeed: {}", stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Removed 'wheels'"));

        let text = fs::read_to_string(&output_path).unwrap();
        assert!(!text.contains("wheels"), "wheels should be removed");
        assert!(text.contains("engine"), "engine should remain");
    }

    // ========================================================================
    // IMPORT (validation-only, no stdlib)
    // ========================================================================

    #[test]
    fn test_import_without_stdlib_warns_on_missing_targets() {
        use syster_cli::{export_model, import_model};

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Export to XMI first
        let xmi_bytes = export_model(&path, "xmi", false, false, None, false).unwrap();
        let xmi_path = dir.path().join("model.xmi");
        fs::write(&xmi_path, &xmi_bytes).unwrap();

        // Import (validation only, no stdlib)
        let result = import_model(&xmi_path, None, false).unwrap();

        assert!(result.element_count > 0, "Should import elements");
        assert!(result.relationship_count > 0, "Should import relationships");
        // Missing targets (e.g. Real) are warnings, not errors
        assert_eq!(result.error_count, 0, "No hard errors expected");
        assert!(
            result.warning_count > 0,
            "Should have warnings for missing stdlib types"
        );
        // Check the warning messages mention the missing types
        let all_messages = result.messages.join(" ");
        assert!(
            all_messages.contains("Warning"),
            "Messages should contain warnings"
        );
    }

    #[test]
    fn test_cli_import_exits_2_with_warnings() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Export to XMI
        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--export", "xmi", "-o"])
            .arg(dir.path().join("model.xmi"))
            .arg(&path)
            .output()
            .expect("Should run export");
        assert!(
            output.status.success(),
            "Export should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Import the XMI (no stdlib → warnings about missing targets)
        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--import"])
            .arg(dir.path().join("model.xmi"))
            .output()
            .expect("Should run import");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert_eq!(
            output.status.code(),
            Some(2),
            "Import with warnings should exit 2, stdout: {}, stderr: {}",
            stdout,
            stderr
        );
        assert!(
            stdout.contains("Imported"),
            "Should report success: {}",
            stdout
        );
        assert!(
            stderr.contains("Warning"),
            "Should print warnings to stderr: {}",
            stderr
        );
    }

    #[test]
    fn test_cli_import_exits_0_when_clean() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();

        // Write a model with no type references (so no missing targets)
        let path = dir.path().join("simple.sysml");
        fs::write(
            &path,
            "package P {\n    part def A;\n    part def B :> A;\n}\n",
        )
        .unwrap();

        // Export to XMI
        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--export", "xmi", "-o"])
            .arg(dir.path().join("simple.xmi"))
            .arg(&path)
            .output()
            .expect("Should run export");
        assert!(
            output.status.success(),
            "Export should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Import back — should be fully clean (exit 0)
        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--no-stdlib", "--import"])
            .arg(dir.path().join("simple.xmi"))
            .output()
            .expect("Should run import");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "Clean import should exit 0, stdout: {}, stderr: {}",
            stdout,
            stderr
        );
        assert!(
            stdout.contains("Imported"),
            "Should report success: {}",
            stdout
        );
    }

    // ========================================================================
    // IMPORT-WORKSPACE (loads stdlib, resolves all types)
    // ========================================================================

    #[test]
    fn test_cli_import_workspace_with_stdlib_exits_0() {
        use std::process::Command;

        let dir = TempDir::new().unwrap();
        let path = write_vehicle_model(dir.path());

        // Export to XMI (with stdlib so types are resolved)
        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--export", "xmi", "-o"])
            .arg(dir.path().join("model.xmi"))
            .arg(&path)
            .output()
            .expect("Should run export");
        assert!(
            output.status.success(),
            "Export should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Import into workspace (loads stdlib, preserves IDs)
        let output = Command::new(env!("CARGO_BIN_EXE_syster"))
            .args(["--import-workspace"])
            .arg(dir.path().join("model.xmi"))
            .output()
            .expect("Should run import-workspace");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "import-workspace should exit 0, stdout: {}, stderr: {}",
            stdout,
            stderr
        );
        assert!(
            stdout.contains("Imported"),
            "Should report import success: {}",
            stdout
        );
        assert!(
            stdout.contains("Element IDs preserved"),
            "Should confirm ID preservation: {}",
            stdout
        );
    }
}
