//! syster-cli library - Core analysis functionality
//!
//! This module provides the `run_analysis` function for parsing and analyzing
//! SysML v2 and KerML files using the syster-base library.

use serde::Serialize;
use std::path::{Path, PathBuf};
use syster::hir::{Severity, check_file};
use syster::ide::AnalysisHost;
use walkdir::WalkDir;

/// Result of analyzing SysML/KerML files.
#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    /// Number of files analyzed.
    pub file_count: usize,
    /// Total number of symbols found.
    pub symbol_count: usize,
    /// Number of errors found.
    pub error_count: usize,
    /// Number of warnings found.
    pub warning_count: usize,
    /// All diagnostics collected.
    pub diagnostics: Vec<DiagnosticInfo>,
}

/// A diagnostic message with location information.
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticInfo {
    /// File path containing the diagnostic.
    pub file: String,
    /// Start line (1-indexed).
    pub line: u32,
    /// Start column (1-indexed).
    pub col: u32,
    /// End line (1-indexed).
    pub end_line: u32,
    /// End column (1-indexed).
    pub end_col: u32,
    /// The diagnostic message.
    pub message: String,
    /// Severity level.
    #[serde(serialize_with = "serialize_severity")]
    pub severity: Severity,
    /// Optional error code.
    pub code: Option<String>,
}

/// Serialize Severity as a string
fn serialize_severity<S>(severity: &Severity, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let s = match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
        Severity::Hint => "hint",
    };
    serializer.serialize_str(s)
}

/// Run analysis on input file or directory.
///
/// # Arguments
/// * `input` - Path to a file or directory to analyze
/// * `verbose` - Enable verbose output
/// * `load_stdlib` - Whether to load the standard library
/// * `stdlib_path` - Optional custom path to the standard library
///
/// # Returns
/// An `AnalysisResult` with file count, symbol count, and diagnostics.
pub fn run_analysis(
    input: &Path,
    verbose: bool,
    load_stdlib: bool,
    stdlib_path: Option<&Path>,
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
    let error_count = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .count();
    let warning_count = diagnostics
        .iter()
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

/// Load input file or directory.
pub fn load_input(host: &mut AnalysisHost, input: &Path, verbose: bool) -> Result<(), String> {
    if input.is_file() {
        load_file(host, input, verbose)
    } else if input.is_dir() {
        load_directory(host, input, verbose)
    } else {
        Err(format!("Path does not exist: {}", input.display()))
    }
}

/// Load a single file into the analysis host.
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
        eprintln!(
            "parse error: {}:{}:{}: {}",
            path.display(),
            err.position.line,
            err.position.column,
            err.message
        );
    }

    Ok(())
}

/// Load all SysML/KerML files from a directory.
fn load_directory(host: &mut AnalysisHost, dir: &Path, verbose: bool) -> Result<(), String> {
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

/// Check if a path is a SysML or KerML file.
fn is_sysml_file(path: &Path) -> bool {
    path.is_file()
        && matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("sysml") | Some("kerml")
        )
}

/// Load standard library files.
pub fn load_stdlib_files(
    host: &mut AnalysisHost,
    custom_path: Option<&Path>,
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
        PathBuf::from("../base/sysml.library"),
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

/// Collect diagnostics from all files in the host.
fn collect_diagnostics(host: &AnalysisHost) -> Vec<DiagnosticInfo> {
    let mut all_diagnostics = Vec::new();

    for path in host.files().keys() {
        if let Some(file_id) = host.get_file_id_for_path(path) {
            let file_path = path.to_string_lossy().to_string();
            let diagnostics = check_file(host.symbol_index(), file_id);

            for diag in diagnostics {
                all_diagnostics.push(DiagnosticInfo {
                    file: file_path.clone(),
                    line: diag.start_line + 1, // 1-indexed for display
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
    all_diagnostics.sort_by(|a, b| (&a.file, a.line, a.col).cmp(&(&b.file, b.line, b.col)));

    all_diagnostics
}

// ============================================================================
// EXPORT FUNCTIONS
// ============================================================================

/// A symbol for JSON export (simplified from HirSymbol).
#[derive(Debug, Serialize)]
pub struct ExportSymbol {
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub file: String,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub supertypes: Vec<String>,
}

/// AST export result.
#[derive(Debug, Serialize)]
pub struct AstExport {
    pub files: Vec<FileAst>,
}

/// AST for a single file.
#[derive(Debug, Serialize)]
pub struct FileAst {
    pub path: String,
    pub symbols: Vec<ExportSymbol>,
}

/// Export AST (symbols) for all files.
pub fn export_ast(
    input: &Path,
    verbose: bool,
    load_stdlib: bool,
    stdlib_path: Option<&Path>,
) -> Result<String, String> {
    let mut host = AnalysisHost::new();

    if load_stdlib {
        load_stdlib_files(&mut host, stdlib_path, verbose)?;
    }

    load_input(&mut host, input, verbose)?;
    let _analysis = host.analysis();

    let mut files = Vec::new();

    // Only export user files, not stdlib
    for path in host.files().keys() {
        let path_str = path.to_string_lossy().to_string();

        // Skip stdlib files
        if path_str.contains("sysml.library") {
            continue;
        }

        if let Some(file_id) = host.get_file_id_for_path(path) {
            let symbols: Vec<ExportSymbol> = host
                .symbol_index()
                .symbols_in_file(file_id)
                .into_iter()
                .map(|sym| ExportSymbol {
                    name: sym.name.to_string(),
                    qualified_name: sym.qualified_name.to_string(),
                    kind: format!("{:?}", sym.kind),
                    file: path_str.clone(),
                    start_line: sym.start_line + 1,
                    start_col: sym.start_col + 1,
                    end_line: sym.end_line + 1,
                    end_col: sym.end_col + 1,
                    doc: sym.doc.as_ref().map(|d| d.to_string()),
                    supertypes: sym.supertypes.iter().map(|s| s.to_string()).collect(),
                })
                .collect();

            files.push(FileAst {
                path: path_str,
                symbols,
            });
        }
    }

    // Sort files by path for consistent output
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let export = AstExport { files };
    serde_json::to_string_pretty(&export).map_err(|e| format!("Failed to serialize AST: {}", e))
}

/// Export analysis result as JSON.
pub fn export_json(result: &AnalysisResult) -> Result<String, String> {
    serde_json::to_string_pretty(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

// ============================================================================
// INTERCHANGE EXPORT
// ============================================================================

/// Export a model to an interchange format.
///
/// Supported formats:
/// - `xmi` - XML Model Interchange
/// - `kpar` - Kernel Package Archive (ZIP)
/// - `jsonld` - JSON-LD
///
/// # Arguments
/// * `input` - Path to a file or directory to analyze
/// * `format` - Output format (xmi, kpar, jsonld)
/// * `verbose` - Enable verbose output
/// * `load_stdlib` - Whether to load the standard library
/// * `stdlib_path` - Optional custom path to the standard library
///
/// # Returns
/// The serialized model as bytes.
#[cfg(feature = "interchange")]
pub fn export_model(
    input: &Path,
    format: &str,
    verbose: bool,
    load_stdlib: bool,
    stdlib_path: Option<&Path>,
    self_contained: bool,
) -> Result<Vec<u8>, String> {
    use syster::interchange::{
        JsonLd, Kpar, ModelFormat, Xmi, Yaml, model_from_symbols, restore_ids_from_symbols,
    };

    let mut host = AnalysisHost::new();

    // 1. Load stdlib if requested
    if load_stdlib {
        load_stdlib_files(&mut host, stdlib_path, verbose)?;
    }

    // 2. Load input file(s)
    load_input(&mut host, input, verbose)?;

    // 2.5. Load metadata if present (for ID preservation on round-trip)
    #[cfg(feature = "interchange")]
    {
        use syster::project::WorkspaceLoader;
        let loader = WorkspaceLoader::new();

        // If input is a file, check for companion metadata
        if input.is_file() {
            let parent_dir = input.parent().unwrap_or(input);
            if let Err(e) = loader.load_metadata_from_directory(parent_dir, &mut host) {
                if verbose {
                    eprintln!("Note: Could not load metadata: {}", e);
                }
            } else if verbose {
                println!("Loaded metadata from {}", parent_dir.display());
            }
        } else if input.is_dir() {
            // For directories, load metadata from that directory
            if let Err(e) = loader.load_metadata_from_directory(input, &mut host) {
                if verbose {
                    eprintln!("Note: Could not load metadata: {}", e);
                }
            } else if verbose {
                println!("Loaded metadata from {}", input.display());
            }
        }
    }

    // 3. Trigger index rebuild
    let analysis = host.analysis();

    // 4. Get symbols from the index (filtered unless self_contained)
    let symbols: Vec<_> = if self_contained {
        // Include all symbols (user model + stdlib)
        analysis.symbol_index().all_symbols().cloned().collect()
    } else {
        // Filter to only user files (exclude stdlib)
        analysis
            .symbol_index()
            .all_symbols()
            .filter(|sym| {
                // Get the file path for this symbol and check if it's stdlib
                if let Some(path) = analysis.get_file_path(sym.file) {
                    !path.contains("sysml.library")
                } else {
                    true // Include if we can't determine the path
                }
            })
            .cloned()
            .collect()
    };

    if verbose {
        println!(
            "Collecting {} symbols (self_contained={})",
            symbols.len(),
            self_contained
        );
    }

    // 5. Convert to interchange model
    let mut model = model_from_symbols(&symbols);

    // 6. Restore original element IDs from symbols (if they exist)
    model = restore_ids_from_symbols(model, analysis.symbol_index());
    if verbose {
        println!("Restored element IDs from symbol database");
    }

    if verbose {
        println!(
            "Exported model: {} elements, {} relationships",
            model.elements.len(),
            model.relationship_count()
        );
    }

    // 8. Serialize to requested format
    match format.to_lowercase().as_str() {
        "xmi" => Xmi.write(&model).map_err(|e| e.to_string()),
        "kpar" => Kpar.write(&model).map_err(|e| e.to_string()),
        "jsonld" | "json-ld" => JsonLd.write(&model).map_err(|e| e.to_string()),
        "yaml" | "yml" => Yaml.write(&model).map_err(|e| e.to_string()),
        _ => Err(format!(
            "Unsupported format: {}. Use xmi, kpar, jsonld, or yaml.",
            format
        )),
    }
}

/// Export model from an existing AnalysisHost to an interchange format.
///
/// This allows exporting from a pre-populated host (e.g., after import_model_into_host).
/// Element IDs are preserved from the symbol database.
///
/// Set `self_contained` to true to include all symbols (including stdlib),
/// or false to only include user model symbols.
#[cfg(feature = "interchange")]
pub fn export_from_host(
    host: &mut AnalysisHost,
    format: &str,
    verbose: bool,
    self_contained: bool,
) -> Result<Vec<u8>, String> {
    use syster::interchange::{
        JsonLd, Kpar, ModelFormat, Xmi, Yaml, model_from_symbols, restore_ids_from_symbols,
    };

    let analysis = host.analysis();
    let symbols: Vec<_> = if self_contained {
        analysis.symbol_index().all_symbols().cloned().collect()
    } else {
        analysis
            .symbol_index()
            .all_symbols()
            .filter(|sym| {
                if let Some(path) = analysis.get_file_path(sym.file) {
                    !path.contains("sysml.library")
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    };

    if verbose {
        println!(
            "Collecting {} symbols (self_contained={})",
            symbols.len(),
            self_contained
        );
    }

    let mut model = model_from_symbols(&symbols);
    model = restore_ids_from_symbols(model, analysis.symbol_index());

    if verbose {
        println!(
            "Exported model: {} elements, {} relationships",
            model.elements.len(),
            model.relationship_count()
        );
    }

    match format.to_lowercase().as_str() {
        "xmi" => Xmi.write(&model).map_err(|e| e.to_string()),
        "kpar" => Kpar.write(&model).map_err(|e| e.to_string()),
        "jsonld" | "json-ld" => JsonLd.write(&model).map_err(|e| e.to_string()),
        "yaml" | "yml" => Yaml.write(&model).map_err(|e| e.to_string()),
        _ => Err(format!(
            "Unsupported format: {}. Use xmi, kpar, jsonld, or yaml.",
            format
        )),
    }
}

/// Result of importing a model from an interchange format.
#[cfg(feature = "interchange")]
#[derive(Debug)]
pub struct ImportResult {
    /// Number of elements imported.
    pub element_count: usize,
    /// Number of relationships imported.
    pub relationship_count: usize,
    /// Number of validation errors (hard failures).
    pub error_count: usize,
    /// Number of validation warnings (non-fatal).
    pub warning_count: usize,
    /// Validation messages.
    pub messages: Vec<String>,
}

/// Import a model from an interchange format file (validation only).
///
/// This validates the model but doesn't load it into a workspace.
/// For importing into a workspace, use `import_model_into_host()`.
///
/// Supported formats are detected from file extension:
/// - `.xmi` - XML Model Interchange
/// - `.kpar` - Kernel Package Archive (ZIP)
/// - `.jsonld`, `.json` - JSON-LD
///
/// # Arguments
/// * `input` - Path to the interchange file
/// * `format` - Optional format override (otherwise detected from extension)
/// * `verbose` - Enable verbose output
///
/// # Returns
/// An `ImportResult` with element count and symbol info.
#[cfg(feature = "interchange")]
pub fn import_model_into_host(
    host: &mut AnalysisHost,
    input: &Path,
    format: Option<&str>,
    verbose: bool,
) -> Result<ImportResult, String> {
    use syster::interchange::{JsonLd, Kpar, ModelFormat, Xmi, detect_format};

    // Read the input file
    let bytes =
        std::fs::read(input).map_err(|e| format!("Failed to read {}: {}", input.display(), e))?;

    // Determine format
    let format_str = format.map(String::from).unwrap_or_else(|| {
        input
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("xmi")
            .to_string()
    });

    if verbose {
        println!(
            "Importing {} as {} into workspace",
            input.display(),
            format_str
        );
    }

    // Parse the model
    let model = match format_str.to_lowercase().as_str() {
        "xmi" | "sysmlx" | "kermlx" => Xmi.read(&bytes).map_err(|e| e.to_string())?,
        "kpar" => Kpar.read(&bytes).map_err(|e| e.to_string())?,
        "jsonld" | "json-ld" | "json" => JsonLd.read(&bytes).map_err(|e| e.to_string())?,
        _ => {
            // Try to detect from file extension
            if let Some(format_impl) = detect_format(input) {
                format_impl.read(&bytes).map_err(|e| e.to_string())?
            } else {
                return Err(format!(
                    "Unknown format: {}. Use xmi, sysmlx, kermlx, kpar, or jsonld.",
                    format_str
                ));
            }
        }
    };

    let element_count = model.elements.len();
    let relationship_count = model.relationship_count();

    if verbose {
        println!(
            "Parsed {} elements and {} relationships",
            element_count, relationship_count
        );
    }

    // Add model to host using the new add_model API
    // This decompiles the model to SysML and parses it, preserving element IDs
    let virtual_path = input.to_string_lossy().to_string() + ".sysml";
    let errors = host.add_model(&model, &virtual_path);

    if verbose {
        if errors.is_empty() {
            println!("Loaded model into workspace with preserved element IDs");
        } else {
            println!("Loaded model with {} parse warnings", errors.len());
        }
    }

    Ok(ImportResult {
        element_count,
        relationship_count,
        error_count: errors.len(),
        warning_count: 0,
        messages: vec![format!("Successfully imported {} elements", element_count)],
    })
}

/// Import and validate a model from an interchange format (legacy version).
///
/// This validates the model but doesn't load it into a workspace.
/// For importing into a workspace, use `import_model_into_host()`.
///
/// # Arguments
/// * `input` - Path to the model file
/// * `format` - Optional format override (xmi, kpar, jsonld)
/// * `verbose` - Enable verbose output
///
/// # Returns
/// An `ImportResult` with element count and validation info.
#[cfg(feature = "interchange")]
pub fn import_model(
    input: &Path,
    format: Option<&str>,
    verbose: bool,
) -> Result<ImportResult, String> {
    use syster::interchange::{JsonLd, Kpar, ModelFormat, Xmi, detect_format};

    // Read the input file
    let bytes =
        std::fs::read(input).map_err(|e| format!("Failed to read {}: {}", input.display(), e))?;

    // Determine format
    let format_str = format.map(String::from).unwrap_or_else(|| {
        input
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("xmi")
            .to_string()
    });

    if verbose {
        println!("Importing {} as {}", input.display(), format_str);
    }

    // Parse the model
    let model = match format_str.to_lowercase().as_str() {
        "xmi" | "sysmlx" | "kermlx" => Xmi.read(&bytes).map_err(|e| e.to_string())?,
        "kpar" => Kpar.read(&bytes).map_err(|e| e.to_string())?,
        "jsonld" | "json-ld" | "json" => JsonLd.read(&bytes).map_err(|e| e.to_string())?,
        _ => {
            // Try to detect from file extension
            if let Some(format_impl) = detect_format(input) {
                format_impl.read(&bytes).map_err(|e| e.to_string())?
            } else {
                return Err(format!(
                    "Unknown format: {}. Use xmi, sysmlx, kermlx, kpar, or jsonld.",
                    format_str
                ));
            }
        }
    };

    // Basic validation
    let mut messages = Vec::new();
    let mut error_count = 0;
    let mut warning_count = 0;

    // Check for orphan relationships (references to non-existent elements)
    // Missing targets are warnings (often stdlib types not included in export).
    // Missing sources are errors (indicates corrupt data).
    // Targets that resolve to `_external` stub elements also count as
    // warnings — they were unresolved external references at export time.
    for rel in model.iter_relationship_elements() {
        if let Some(rd) = &rel.relationship {
            if let Some(src) = rd.source.first() {
                if model.elements.get(src).is_none() {
                    messages.push(format!("Error: Relationship source '{}' not found", src));
                    error_count += 1;
                }
            }
            if let Some(tgt) = rd.target.first() {
                let target_el = model.elements.get(tgt);
                let is_missing = target_el.is_none();
                // Targets whose ID starts with `_ext_` are stub elements
                // created by model_from_symbols() for unresolved external
                // references (e.g. stdlib types not included in export).
                let is_external_stub = tgt.as_str().starts_with("_ext_");
                if is_missing || is_external_stub {
                    let name = target_el
                        .and_then(|el| el.name.as_deref())
                        .unwrap_or(tgt.as_str());
                    messages.push(format!(
                        "Warning: Relationship target '{}' not found (may be a stdlib type)",
                        name
                    ));
                    warning_count += 1;
                }
            }
        }
    }

    if verbose {
        println!(
            "Imported: {} elements, {} relationships, {} errors, {} warnings",
            model.elements.len(),
            model.relationship_count(),
            error_count,
            warning_count
        );
        for msg in &messages {
            println!("  {}", msg);
        }
    }

    Ok(ImportResult {
        element_count: model.elements.len(),
        relationship_count: model.relationship_count(),
        error_count,
        warning_count,
        messages,
    })
}

// ============================================================================
// SEMANTIC MODEL COMMANDS (AnalysisHost-based)
// ============================================================================

/// Result of querying a model.
#[cfg(feature = "interchange")]
#[derive(Debug, Serialize)]
pub struct QueryResult {
    /// Number of elements matching the query.
    pub match_count: usize,
    /// The matching elements.
    pub elements: Vec<ElementInfo>,
}

/// Information about a single model element (JSON-serializable).
#[cfg(feature = "interchange")]
#[derive(Debug, Clone, Serialize)]
pub struct ElementInfo {
    /// Element ID.
    pub id: String,
    /// Declared name.
    pub name: Option<String>,
    /// Qualified name.
    pub qualified_name: Option<String>,
    /// Metaclass kind (e.g., "PartDefinition", "PartUsage").
    pub kind: String,
    /// Whether the element is abstract.
    pub is_abstract: bool,
    /// Owner element name (if any).
    pub owner: Option<String>,
    /// Number of owned members.
    pub owned_member_count: usize,
    /// Type names (for usages typed by a definition).
    pub typed_by: Vec<String>,
    /// Supertype names (for specializations).
    pub supertypes: Vec<String>,
    /// Documentation text.
    pub documentation: Option<String>,
}

/// Inspect result — detailed view of one element and its surroundings.
#[cfg(feature = "interchange")]
#[derive(Debug, Serialize)]
pub struct InspectResult {
    /// The inspected element.
    pub element: ElementInfo,
    /// Direct children.
    pub children: Vec<ElementInfo>,
    /// Relationships from this element.
    pub relationships_from: Vec<RelationshipInfo>,
    /// Relationships to this element.
    pub relationships_to: Vec<RelationshipInfo>,
}

/// A relationship in human-readable form.
#[cfg(feature = "interchange")]
#[derive(Debug, Clone, Serialize)]
pub struct RelationshipInfo {
    /// Relationship kind (e.g., "Specialization", "FeatureTyping").
    pub kind: String,
    /// Source element name.
    pub source: String,
    /// Target element name.
    pub target: String,
}

/// Result of a rename operation.
#[cfg(feature = "interchange")]
#[derive(Debug)]
pub struct RenameResult {
    /// Old name.
    pub old_name: String,
    /// New name.
    pub new_name: String,
    /// Rendered SysML text after rename.
    pub rendered_text: String,
    /// Updated metadata JSON (if metadata was loaded).
    pub metadata_json: Option<String>,
}

/// Load a SysML file into an `AnalysisHost` with companion metadata applied.
///
/// This is the standard entry-point for CLI commands that need a parsed
/// `Model` — it replaces the old `ModelHost::from_text()` pattern,
/// using `AnalysisHost` directly instead.
///
/// Returns the host (with model cached) and the raw source text.
#[cfg(feature = "interchange")]
fn load_model_from_file(
    input: &Path,
    verbose: bool,
) -> Result<(syster::ide::AnalysisHost, String), String> {
    let source = std::fs::read_to_string(input)
        .map_err(|e| format!("Failed to read {}: {}", input.display(), e))?;

    let path_str = input.to_string_lossy().to_string();
    let mut host = syster::ide::AnalysisHost::new();
    let errors = host.set_file_content(&path_str, &source);

    if !errors.is_empty() && verbose {
        eprintln!("Parse warnings: {}", errors.len());
    }

    // Apply companion metadata (restores original element IDs)
    // Take the model, apply metadata, put it back.
    let model = host.take_model().unwrap();
    let model = load_companion_metadata(input, model, verbose);
    host.set_model_cache(model);

    if verbose {
        eprintln!(
            "Loaded {} elements, {} relationships",
            host.model().element_count(),
            host.model().relationship_count()
        );
    }

    Ok((host, source))
}

/// Try to load a companion `.metadata.json` file for ID preservation.
///
/// Given `/path/to/model.sysml`, looks for `/path/to/model.metadata.json`.
/// If found, applies the original element IDs to the model via
/// `restore_element_ids`.
#[cfg(feature = "interchange")]
fn load_companion_metadata(
    input: &Path,
    model: syster::interchange::model::Model,
    verbose: bool,
) -> syster::interchange::model::Model {
    use syster::interchange::metadata::ImportMetadata;
    use syster::interchange::recompile::restore_element_ids;

    let metadata_path = input.with_extension("metadata.json");
    if metadata_path.exists() {
        match ImportMetadata::read_from_file(&metadata_path) {
            Ok(metadata) => {
                if verbose {
                    eprintln!(
                        "Loaded metadata from {} ({} elements)",
                        metadata_path.display(),
                        metadata.elements.len()
                    );
                }
                restore_element_ids(model, &metadata)
            }
            Err(e) => {
                if verbose {
                    eprintln!(
                        "Note: Could not load metadata from {}: {}",
                        metadata_path.display(),
                        e
                    );
                }
                model
            }
        }
    } else {
        model
    }
}

/// Build updated metadata JSON after a rename operation.
///
/// Takes the renamed model and produces a new `ImportMetadata` that maps
/// qualified names to their (preserved) element IDs.
#[cfg(feature = "interchange")]
fn build_updated_metadata(
    model: &syster::interchange::model::Model,
    source_path: &Path,
) -> Option<String> {
    use syster::interchange::metadata::{ElementMeta, ImportMetadata, SourceInfo};

    let mut metadata = ImportMetadata::new().with_source(SourceInfo::from_path(
        source_path.to_string_lossy().to_string(),
    ));

    for element in model.elements.values() {
        if let Some(qn) = &element.qualified_name {
            let meta = ElementMeta::with_id(element.id.as_str());
            metadata.add_element(qn.as_ref(), meta);
        }
    }

    serde_json::to_string_pretty(&metadata).ok()
}

/// Query a SysML model by name, kind, or qualified name.
///
/// Loads the file via `AnalysisHost` and searches for elements
/// matching the given criteria. If a companion `.metadata.json` exists,
/// original element IDs are restored.
///
/// # Arguments
/// * `input` - Path to a `.sysml` file
/// * `name` - Optional name filter (substring match)
/// * `kind` - Optional kind filter (e.g., "PartDefinition")
/// * `qualified_name` - Optional exact qualified name lookup
/// * `verbose` - Enable verbose output
#[cfg(feature = "interchange")]
pub fn query_model(
    input: &Path,
    name: Option<&str>,
    kind: Option<&str>,
    qualified_name: Option<&str>,
    verbose: bool,
) -> Result<QueryResult, String> {
    let (mut host, _source) = load_model_from_file(input, verbose)?;

    // Get the model reference and query through it directly
    let model = host.model();

    // Collect matching elements
    let views: Vec<_> = if let Some(qn) = qualified_name {
        // Exact qualified name lookup
        model.find_by_qualified_name(qn).into_iter().collect()
    } else {
        // Get all elements, then filter
        model
            .elements
            .values()
            .filter_map(|el| {
                let view = model.view(&el.id)?;

                // Name filter (substring)
                if let Some(n) = name {
                    let matches_name = view.name().map(|vn| vn.contains(n)).unwrap_or(false);
                    if !matches_name {
                        return None;
                    }
                }

                // Kind filter
                if let Some(k) = kind {
                    let kind_str = format!("{:?}", view.kind());
                    if !kind_str.eq_ignore_ascii_case(k) {
                        return None;
                    }
                }

                // Skip relationship-only elements if no explicit kind filter
                if kind.is_none() && view.kind().is_relationship() {
                    return None;
                }

                Some(view)
            })
            .collect()
    };

    let elements: Vec<ElementInfo> = views.iter().map(|v| element_info(v)).collect();

    Ok(QueryResult {
        match_count: elements.len(),
        elements,
    })
}

/// Inspect a single element by name or qualified name — shows the element
/// plus its children and relationships.
///
/// # Arguments
/// * `input` - Path to a `.sysml` file
/// * `target` - Name or qualified name of the element to inspect
/// * `verbose` - Enable verbose output
#[cfg(feature = "interchange")]
pub fn inspect_element(input: &Path, target: &str, verbose: bool) -> Result<InspectResult, String> {
    let (mut host, _source) = load_model_from_file(input, verbose)?;

    let model = host.model();

    // Try qualified name first, then plain name
    let view = model
        .find_by_qualified_name(target)
        .or_else(|| model.find_by_name(target).into_iter().next())
        .ok_or_else(|| format!("Element '{}' not found", target))?;

    if verbose {
        eprintln!("Found: {:?} '{}'", view.kind(), view.name().unwrap_or("?"));
    }

    let children: Vec<ElementInfo> = view
        .owned_members()
        .iter()
        .map(|c| element_info(c))
        .collect();

    let relationships_from: Vec<RelationshipInfo> = view
        .relationships_from()
        .iter()
        .map(|r| {
            let source_name = r
                .source()
                .and_then(|sid| model.view(sid))
                .and_then(|v| v.name().map(String::from))
                .unwrap_or_else(|| {
                    r.source()
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_default()
                });
            let target_name = r
                .target()
                .and_then(|tid| model.view(tid))
                .and_then(|v| v.name().map(String::from))
                .unwrap_or_else(|| {
                    r.target()
                        .map(|t| t.as_str().to_string())
                        .unwrap_or_default()
                });
            RelationshipInfo {
                kind: format!("{:?}", r.kind),
                source: source_name,
                target: target_name,
            }
        })
        .collect();

    let relationships_to: Vec<RelationshipInfo> = view
        .relationships_to()
        .iter()
        .map(|r| {
            let source_name = r
                .source()
                .and_then(|sid| model.view(sid))
                .and_then(|v| v.name().map(String::from))
                .unwrap_or_else(|| {
                    r.source()
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_default()
                });
            let target_name = r
                .target()
                .and_then(|tid| model.view(tid))
                .and_then(|v| v.name().map(String::from))
                .unwrap_or_else(|| {
                    r.target()
                        .map(|t| t.as_str().to_string())
                        .unwrap_or_default()
                });
            RelationshipInfo {
                kind: format!("{:?}", r.kind),
                source: source_name,
                target: target_name,
            }
        })
        .collect();

    Ok(InspectResult {
        element: element_info(&view),
        children,
        relationships_from,
        relationships_to,
    })
}

/// Rename an element in a SysML file and output the modified text.
///
/// Uses `AnalysisHost::apply_model_edit()` to perform the rename via
/// the semantic Model layer, with automatic text re-rendering and
/// SymbolIndex sync.
///
/// # Arguments
/// * `input` - Path to a `.sysml` file
/// * `target` - Name or qualified name of the element to rename
/// * `new_name` - The new name for the element
/// * `verbose` - Enable verbose output
#[cfg(feature = "interchange")]
pub fn rename_element(
    input: &Path,
    target: &str,
    new_name: &str,
    verbose: bool,
) -> Result<RenameResult, String> {
    let (mut host, _source) = load_model_from_file(input, verbose)?;
    let path_str = input.to_string_lossy().to_string();

    // Find the element (immutable access, then release borrow)
    let (old_name, element_id) = {
        let model = host.model();
        let view = model
            .find_by_qualified_name(target)
            .or_else(|| model.find_by_name(target).into_iter().next())
            .ok_or_else(|| format!("Element '{}' not found", target))?;
        let old = view.name().unwrap_or("(anonymous)").to_string();
        let id = view.id().clone();
        if verbose {
            eprintln!("Renaming {:?} '{}' -> '{}'", view.kind(), old, new_name);
        }
        (old, id)
    };

    // Apply rename via apply_model_edit (handles SourceMap, render, re-parse, ID restore)
    let result = host.apply_model_edit(&path_str, |model, tracker| {
        tracker.rename(model, &element_id, new_name);
    });

    if verbose {
        eprintln!("Renamed '{}' -> '{}'", old_name, new_name);
    }

    // Build updated metadata JSON (preserves element IDs after rename)
    let metadata_json = build_updated_metadata(host.model(), input);

    Ok(RenameResult {
        old_name,
        new_name: new_name.to_string(),
        rendered_text: result.rendered_text,
        metadata_json,
    })
}

/// Result of adding a member to an element.
#[cfg(feature = "interchange")]
#[derive(Debug)]
pub struct AddMemberResult {
    /// Parent element name.
    pub parent_name: String,
    /// Added element name.
    pub member_name: String,
    /// Added element kind.
    pub member_kind: String,
    /// Generated element ID for the new member.
    pub member_id: String,
    /// Rendered SysML text after the addition.
    pub rendered_text: String,
    /// Updated metadata JSON.
    pub metadata_json: Option<String>,
}

/// Result of removing an element.
#[cfg(feature = "interchange")]
#[derive(Debug)]
pub struct RemoveMemberResult {
    /// Removed element name.
    pub removed_name: String,
    /// Rendered SysML text after removal.
    pub rendered_text: String,
    /// Updated metadata JSON.
    pub metadata_json: Option<String>,
}

/// Add a new member (part, attribute, etc.) to an existing element.
///
/// Uses `AnalysisHost::apply_model_edit()` to add a child element via
/// the semantic Model layer, with automatic text re-rendering and
/// SymbolIndex sync.
///
/// # Arguments
/// * `input` - Path to a `.sysml` file
/// * `parent` - Name or qualified name of the parent element
/// * `member_name` - Name for the new member
/// * `member_kind` - Kind string (e.g., "PartUsage", "AttributeUsage", "PartDefinition")
/// * `type_name` - Optional type to assign (e.g., "Engine" for `part engine : Engine`)
/// * `verbose` - Enable verbose output
#[cfg(feature = "interchange")]
pub fn add_member(
    input: &Path,
    parent: &str,
    member_name: &str,
    member_kind: &str,
    type_name: Option<&str>,
    verbose: bool,
) -> Result<AddMemberResult, String> {
    use syster::interchange::model::{Element, ElementId, ElementKind as EK};

    let (mut host, _source) = load_model_from_file(input, verbose)?;
    let path_str = input.to_string_lossy().to_string();

    // Find the parent element (immutable access, then release borrow)
    let (parent_id, parent_display_name, new_qn) = {
        let model = host.model();
        let parent_view = model
            .find_by_qualified_name(parent)
            .or_else(|| model.find_by_name(parent).into_iter().next())
            .ok_or_else(|| format!("Parent element '{}' not found", parent))?;

        let pid = parent_view.id().clone();
        let pname = parent_view.name().unwrap_or("(anonymous)").to_string();
        let parent_qn = parent_view.qualified_name().unwrap_or(pname.as_str());
        let qn = format!("{}::{}", parent_qn, member_name);

        if verbose {
            eprintln!(
                "Adding {:?} '{}' to {:?} '{}'",
                parse_element_kind(member_kind)
                    .unwrap_or(syster::interchange::ElementKind::Package),
                member_name,
                parent_view.kind(),
                pname
            );
        }

        (pid, pname, qn)
    };

    // Parse the kind string
    let kind = parse_element_kind(member_kind)?;

    // Resolve type ID before entering the edit closure (if needed)
    let type_id = if let Some(tn) = type_name {
        let model = host.model();
        Some(
            model
                .find_by_qualified_name(tn)
                .or_else(|| model.find_by_name(tn).into_iter().next())
                .map(|v| v.id().clone())
                .ok_or_else(|| format!("Type '{}' not found", tn))?,
        )
    } else {
        None
    };

    // Create the new element ID upfront (needed for result)
    let new_id = ElementId::generate();
    let new_id_for_closure = new_id.clone();

    // Apply via apply_model_edit (handles SourceMap, render, re-parse, ID restore)
    let result = host.apply_model_edit(&path_str, move |model, tracker| {
        let element = Element::new(new_id_for_closure.clone(), kind)
            .with_name(member_name)
            .with_qualified_name(new_qn);
        tracker.add_element(model, element, Some(&parent_id));

        // If a type is specified, add a FeatureTyping relationship
        if let Some(tid) = type_id {
            let rel_id = ElementId::generate();
            tracker.add_relationship(model, rel_id, EK::FeatureTyping, new_id_for_closure, tid);
        }
    });

    if verbose {
        eprintln!(
            "Added {} '{}' to '{}'",
            member_kind, member_name, parent_display_name
        );
    }

    let metadata_json = build_updated_metadata(host.model(), input);

    Ok(AddMemberResult {
        parent_name: parent_display_name,
        member_name: member_name.to_string(),
        member_kind: member_kind.to_string(),
        member_id: new_id.as_str().to_string(),
        rendered_text: result.rendered_text,
        metadata_json,
    })
}

/// Remove an element from the model by name or qualified name.
///
/// Uses `AnalysisHost::apply_model_edit()` to remove the element via
/// the semantic Model layer, with automatic text re-rendering and
/// SymbolIndex sync.
///
/// # Arguments
/// * `input` - Path to a `.sysml` file
/// * `target` - Name or qualified name of the element to remove
/// * `verbose` - Enable verbose output
#[cfg(feature = "interchange")]
pub fn remove_member(
    input: &Path,
    target: &str,
    verbose: bool,
) -> Result<RemoveMemberResult, String> {
    let (mut host, _source) = load_model_from_file(input, verbose)?;
    let path_str = input.to_string_lossy().to_string();

    // Find the element (immutable access, then release borrow)
    let (element_id, removed_name) = {
        let model = host.model();
        let view = model
            .find_by_qualified_name(target)
            .or_else(|| model.find_by_name(target).into_iter().next())
            .ok_or_else(|| format!("Element '{}' not found", target))?;

        let id = view.id().clone();
        let name = view.name().unwrap_or("(anonymous)").to_string();
        if verbose {
            eprintln!("Removing {:?} '{}'", view.kind(), name);
        }
        (id, name)
    };

    // Apply removal via apply_model_edit (handles SourceMap, render, re-parse, ID restore)
    let result = host.apply_model_edit(&path_str, |model, tracker| {
        tracker.remove_element(model, &element_id);
    });

    if verbose {
        eprintln!("Removed '{}'", removed_name);
    }

    let metadata_json = build_updated_metadata(host.model(), input);

    Ok(RemoveMemberResult {
        removed_name,
        rendered_text: result.rendered_text,
        metadata_json,
    })
}

/// Parse a kind string into an ElementKind.
#[cfg(feature = "interchange")]
fn parse_element_kind(kind_str: &str) -> Result<syster::interchange::model::ElementKind, String> {
    use syster::interchange::model::ElementKind;

    match kind_str {
        // Namespaces and Packages
        "Namespace" | "namespace" => Ok(ElementKind::Namespace),
        "Package" | "package" => Ok(ElementKind::Package),
        "LibraryPackage" | "library package" => Ok(ElementKind::LibraryPackage),

        // KerML Classifiers
        "Class" | "class" => Ok(ElementKind::Class),
        "DataType" | "datatype" => Ok(ElementKind::DataType),
        "Structure" | "struct" => Ok(ElementKind::Structure),
        "Association" | "assoc" => Ok(ElementKind::Association),
        "AssociationStructure" | "assoc struct" => Ok(ElementKind::AssociationStructure),
        "Interaction" | "interaction" => Ok(ElementKind::Interaction),
        "Behavior" | "behavior" => Ok(ElementKind::Behavior),
        "Function" | "function" => Ok(ElementKind::Function),
        "Predicate" | "predicate" => Ok(ElementKind::Predicate),

        // SysML Definitions
        "PartDefinition" | "part def" => Ok(ElementKind::PartDefinition),
        "ItemDefinition" | "item def" => Ok(ElementKind::ItemDefinition),
        "ActionDefinition" | "action def" => Ok(ElementKind::ActionDefinition),
        "PortDefinition" | "port def" => Ok(ElementKind::PortDefinition),
        "AttributeDefinition" | "attribute def" => Ok(ElementKind::AttributeDefinition),
        "ConnectionDefinition" | "connection def" => Ok(ElementKind::ConnectionDefinition),
        "InterfaceDefinition" | "interface def" => Ok(ElementKind::InterfaceDefinition),
        "AllocationDefinition" | "allocation def" => Ok(ElementKind::AllocationDefinition),
        "RequirementDefinition" | "requirement def" => Ok(ElementKind::RequirementDefinition),
        "ConstraintDefinition" | "constraint def" => Ok(ElementKind::ConstraintDefinition),
        "StateDefinition" | "state def" => Ok(ElementKind::StateDefinition),
        "CalculationDefinition" | "calc def" => Ok(ElementKind::CalculationDefinition),
        "UseCaseDefinition" | "use case def" => Ok(ElementKind::UseCaseDefinition),
        "AnalysisCaseDefinition" | "analysis case def" => Ok(ElementKind::AnalysisCaseDefinition),
        "ConcernDefinition" | "concern def" => Ok(ElementKind::ConcernDefinition),
        "ViewDefinition" | "view def" => Ok(ElementKind::ViewDefinition),
        "ViewpointDefinition" | "viewpoint def" => Ok(ElementKind::ViewpointDefinition),
        "RenderingDefinition" | "rendering def" => Ok(ElementKind::RenderingDefinition),
        "EnumerationDefinition" | "enum def" => Ok(ElementKind::EnumerationDefinition),
        "MetadataDefinition" | "metadata def" => Ok(ElementKind::MetadataDefinition),

        // SysML Usages
        "PartUsage" | "part" => Ok(ElementKind::PartUsage),
        "ItemUsage" | "item" => Ok(ElementKind::ItemUsage),
        "ActionUsage" | "action" => Ok(ElementKind::ActionUsage),
        "PortUsage" | "port" => Ok(ElementKind::PortUsage),
        "AttributeUsage" | "attribute" => Ok(ElementKind::AttributeUsage),
        "ConnectionUsage" | "connection" => Ok(ElementKind::ConnectionUsage),
        "InterfaceUsage" | "interface" => Ok(ElementKind::InterfaceUsage),
        "AllocationUsage" | "allocation" => Ok(ElementKind::AllocationUsage),
        "RequirementUsage" | "requirement" => Ok(ElementKind::RequirementUsage),
        "ConstraintUsage" | "constraint" => Ok(ElementKind::ConstraintUsage),
        "StateUsage" | "state" => Ok(ElementKind::StateUsage),
        "TransitionUsage" | "transition" => Ok(ElementKind::TransitionUsage),
        "CalculationUsage" | "calc" => Ok(ElementKind::CalculationUsage),
        "ReferenceUsage" | "ref" => Ok(ElementKind::ReferenceUsage),
        "OccurrenceUsage" | "occurrence" => Ok(ElementKind::OccurrenceUsage),
        "FlowConnectionUsage" | "flow" => Ok(ElementKind::FlowConnectionUsage),
        "SuccessionFlowConnectionUsage" | "succession flow" => {
            Ok(ElementKind::SuccessionFlowConnectionUsage)
        }
        "MetadataUsage" | "metadata" => Ok(ElementKind::MetadataUsage),

        // KerML Features
        "Feature" | "feature" => Ok(ElementKind::Feature),
        "Step" | "step" => Ok(ElementKind::Step),
        "Connector" | "connector" => Ok(ElementKind::Connector),
        "BindingConnector" | "binding" => Ok(ElementKind::BindingConnector),
        "Succession" | "succession" => Ok(ElementKind::Succession),

        // Comments and documentation
        "Comment" | "comment" => Ok(ElementKind::Comment),
        "Documentation" | "doc" => Ok(ElementKind::Documentation),

        _ => Err(format!(
            "Unknown element kind: '{}'. Examples: part, part def, attribute, connection def, etc.",
            kind_str
        )),
    }
}

/// Helper: convert an ElementView to a serializable ElementInfo.
#[cfg(feature = "interchange")]
fn element_info(view: &syster::interchange::views::ElementView<'_>) -> ElementInfo {
    ElementInfo {
        id: view.id().as_str().to_string(),
        name: view.name().map(String::from),
        qualified_name: view.qualified_name().map(String::from),
        kind: format!("{:?}", view.kind()),
        is_abstract: view.is_abstract(),
        owner: view.owner().and_then(|o| o.name().map(String::from)),
        owned_member_count: view.owned_members().len(),
        typed_by: view
            .typing()
            .iter()
            .filter_map(|t| t.name().map(String::from))
            .collect(),
        supertypes: view
            .supertypes()
            .iter()
            .filter_map(|s| s.name().map(String::from))
            .collect(),
        documentation: view.documentation().map(String::from),
    }
}

/// Result of decompiling a model to SysML files.
#[cfg(feature = "interchange")]
#[derive(Debug)]
pub struct DecompileResult {
    /// Generated SysML text.
    pub sysml_text: String,
    /// Metadata JSON for preserving element IDs.
    pub metadata_json: String,
    /// Number of elements decompiled.
    pub element_count: usize,
    /// Source file path.
    pub source_path: String,
}

/// Decompile an interchange file to SysML text with metadata.
///
/// This function converts an XMI/KPAR/JSON-LD file to SysML text plus
/// a companion metadata JSON file that preserves element IDs for
/// lossless round-tripping.
///
/// # Arguments
/// * `input` - Path to the interchange file
/// * `format` - Optional format override (otherwise detected from extension)
/// * `verbose` - Enable verbose output
///
/// # Returns
/// A `DecompileResult` with SysML text and metadata JSON.
#[cfg(feature = "interchange")]
pub fn decompile_model(
    input: &Path,
    format: Option<&str>,
    verbose: bool,
) -> Result<DecompileResult, String> {
    use syster::interchange::{
        JsonLd, Kpar, ModelFormat, SourceInfo, Xmi, decompile_with_source, detect_format,
    };

    // Read the input file
    let bytes =
        std::fs::read(input).map_err(|e| format!("Failed to read {}: {}", input.display(), e))?;

    // Determine format
    let format_str = format.map(String::from).unwrap_or_else(|| {
        input
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("xmi")
            .to_string()
    });

    if verbose {
        println!("Decompiling {} as {}", input.display(), format_str);
    }

    // Parse the model
    let model = match format_str.to_lowercase().as_str() {
        "xmi" | "sysmlx" | "kermlx" => Xmi.read(&bytes).map_err(|e| e.to_string())?,
        "kpar" => Kpar.read(&bytes).map_err(|e| e.to_string())?,
        "jsonld" | "json-ld" | "json" => JsonLd.read(&bytes).map_err(|e| e.to_string())?,
        _ => {
            if let Some(format_impl) = detect_format(input) {
                format_impl.read(&bytes).map_err(|e| e.to_string())?
            } else {
                return Err(format!(
                    "Unknown format: {}. Use xmi, sysmlx, kermlx, kpar, or jsonld.",
                    format_str
                ));
            }
        }
    };

    let element_count = model.elements.len();

    // Create source info
    let source = SourceInfo::from_path(input.to_string_lossy()).with_format(&format_str);

    // Decompile to SysML
    let result = decompile_with_source(&model, source);

    if verbose {
        println!(
            "Decompiled: {} elements -> {} chars of SysML, {} metadata entries",
            element_count,
            result.text.len(),
            result.metadata.elements.len()
        );
    }

    // Serialize metadata to JSON
    let metadata_json = serde_json::to_string_pretty(&result.metadata)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

    Ok(DecompileResult {
        sysml_text: result.text,
        metadata_json,
        element_count,
        source_path: input.to_string_lossy().to_string(),
    })
}
