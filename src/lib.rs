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
fn load_input(host: &mut AnalysisHost, input: &Path, verbose: bool) -> Result<(), String> {
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

    for (path, _) in host.files() {
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
    input: &PathBuf,
    verbose: bool,
    load_stdlib: bool,
    stdlib_path: Option<&PathBuf>,
) -> Result<String, String> {
    let mut host = AnalysisHost::new();

    if load_stdlib {
        load_stdlib_files(&mut host, stdlib_path, verbose)?;
    }

    load_input(&mut host, input, verbose)?;
    let _analysis = host.analysis();

    let mut files = Vec::new();

    // Only export user files, not stdlib
    for (path, _) in host.files() {
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
    input: &PathBuf,
    format: &str,
    verbose: bool,
    load_stdlib: bool,
    stdlib_path: Option<&PathBuf>,
) -> Result<Vec<u8>, String> {
    use syster::interchange::{
        JsonLd, Kpar, ModelFormat, Xmi, model_from_symbols, restore_ids_from_symbols,
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
        let mut loader = WorkspaceLoader::new();

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

    // 4. Get all symbols from the index
    let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();

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
            model.relationships.len()
        );
    }

    // 8. Serialize to requested format
    match format.to_lowercase().as_str() {
        "xmi" => Xmi.write(&model).map_err(|e| e.to_string()),
        "kpar" => Kpar.write(&model).map_err(|e| e.to_string()),
        "jsonld" | "json-ld" => JsonLd.write(&model).map_err(|e| e.to_string()),
        _ => Err(format!(
            "Unsupported format: {}. Use xmi, kpar, or jsonld.",
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
    /// Number of validation errors.
    pub error_count: usize,
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
    input: &PathBuf,
    format: Option<&str>,
    verbose: bool,
) -> Result<ImportResult, String> {
    use syster::interchange::{JsonLd, Kpar, ModelFormat, Xmi, detect_format, symbols_from_model};

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

    // Convert model to symbols
    let symbols = symbols_from_model(&model);
    let symbol_count = symbols.len();

    if verbose {
        println!(
            "Converted {} elements to {} symbols",
            model.elements.len(),
            symbol_count
        );
    }

    // Add symbols to host
    host.add_symbols_from_model(symbols);

    if verbose {
        println!("Loaded symbols into workspace with preserved element IDs");
    }

    Ok(ImportResult {
        element_count: model.elements.len(),
        relationship_count: model.relationships.len(),
        error_count: 0,
        messages: vec![format!("Successfully imported {} symbols", symbol_count)],
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
    input: &PathBuf,
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

    // Check for orphan relationships (references to non-existent elements)
    for rel in &model.relationships {
        if model.elements.get(&rel.source).is_none() {
            messages.push(format!(
                "Warning: Relationship source '{}' not found",
                rel.source
            ));
            error_count += 1;
        }
        if model.elements.get(&rel.target).is_none() {
            messages.push(format!(
                "Warning: Relationship target '{}' not found",
                rel.target
            ));
            error_count += 1;
        }
    }

    if verbose {
        println!(
            "Imported: {} elements, {} relationships, {} validation issues",
            model.elements.len(),
            model.relationships.len(),
            error_count
        );
        for msg in &messages {
            println!("  {}", msg);
        }
    }

    Ok(ImportResult {
        element_count: model.elements.len(),
        relationship_count: model.relationships.len(),
        error_count,
        messages,
    })
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
    input: &PathBuf,
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
