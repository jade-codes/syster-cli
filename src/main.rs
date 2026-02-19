//! syster CLI - Command-line interface for SysML v2 and KerML analysis

use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;
use syster::hir::Severity;
use syster_cli::{DiagnosticInfo, export_ast, export_json, run_analysis};
#[cfg(feature = "interchange")]
use syster_cli::{add_member, inspect_element, query_model, remove_member, rename_element};
#[cfg(feature = "interchange")]
use syster_cli::{decompile_model, export_model, import_model, import_model_into_host};

/// Output format for export commands
#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    /// Human-readable text format
    Text,
    /// JSON format
    Json,
}

/// Interchange format for model export
#[cfg(feature = "interchange")]
#[derive(Clone, Copy, Debug, ValueEnum)]
enum InterchangeFormat {
    /// XML Model Interchange
    Xmi,
    /// Kernel Package Archive (ZIP)
    Kpar,
    /// JSON-LD
    JsonLd,
    /// YAML
    Yaml,
}

#[derive(Parser)]
#[command(name = "syster")]
#[command(about = "SysML v2 parser and semantic analyzer", long_about = None)]
#[command(version)]
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

    /// Path to custom standard library (default: sysml.library)
    #[arg(long, value_name = "PATH")]
    stdlib_path: Option<PathBuf>,

    /// Export AST (abstract syntax tree) for all files
    #[arg(long)]
    export_ast: bool,

    /// Export analysis results as JSON
    #[arg(long)]
    json: bool,

    /// Export model to interchange format (xmi, kpar, jsonld)
    #[cfg(feature = "interchange")]
    #[arg(long, value_name = "FORMAT")]
    export: Option<InterchangeFormat>,

    /// Import and validate an interchange file (xmi, kpar, jsonld)
    #[cfg(feature = "interchange")]
    #[arg(long)]
    import: bool,

    /// Import interchange file into workspace for analysis (preserves element IDs)
    #[cfg(feature = "interchange")]
    #[arg(long)]
    import_workspace: bool,

    /// Decompile interchange file to SysML text + metadata
    #[cfg(feature = "interchange")]
    #[arg(long)]
    decompile: bool,

    /// Include standard library in export (self-contained output)
    #[cfg(feature = "interchange")]
    #[arg(long)]
    self_contained: bool,

    /// Query model elements by name (substring match)
    #[cfg(feature = "interchange")]
    #[arg(long, value_name = "NAME")]
    query: Option<String>,

    /// List all model elements (use with --kind to filter)
    #[cfg(feature = "interchange")]
    #[arg(long)]
    list: bool,

    /// Filter query results by metaclass kind (e.g., PartDefinition, PartUsage)
    #[cfg(feature = "interchange")]
    #[arg(long, value_name = "KIND")]
    kind: Option<String>,

    /// Inspect a specific element by name or qualified name
    #[cfg(feature = "interchange")]
    #[arg(long, value_name = "NAME")]
    inspect: Option<String>,

    /// Rename an element: --rename OLD=NEW
    #[cfg(feature = "interchange")]
    #[arg(long, value_name = "OLD=NEW")]
    rename: Option<String>,

    /// Add a member to an element: --add-member PARENT:KIND:NAME[:TYPE]
    #[cfg(feature = "interchange")]
    #[arg(long, value_name = "PARENT:KIND:NAME[:TYPE]")]
    add_member: Option<String>,

    /// Remove an element by name or qualified name
    #[cfg(feature = "interchange")]
    #[arg(long, value_name = "NAME")]
    remove_member: Option<String>,

    /// Write output to file instead of stdout
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.verbose {
        eprintln!("Analyzing: {}", cli.input.display());
    }

    // Handle decompile (convert XMI to SysML text)
    #[cfg(feature = "interchange")]
    if cli.decompile {
        match decompile_model(&cli.input, None, cli.verbose) {
            Ok(result) => {
                eprintln!(
                    "✓ Decompiled {} elements from {}",
                    result.element_count, result.source_path
                );

                // Write SysML file
                let sysml_path = cli
                    .output
                    .clone()
                    .unwrap_or_else(|| cli.input.with_extension("sysml"));
                if let Err(e) = std::fs::write(&sysml_path, &result.sysml_text) {
                    eprintln!("error: failed to write {}: {}", sysml_path.display(), e);
                    return ExitCode::FAILURE;
                }
                eprintln!("  Wrote: {}", sysml_path.display());

                // Write metadata file
                let metadata_path = sysml_path.with_extension("metadata.json");
                if let Err(e) = std::fs::write(&metadata_path, &result.metadata_json) {
                    eprintln!("error: failed to write {}: {}", metadata_path.display(), e);
                    return ExitCode::FAILURE;
                }
                eprintln!("  Wrote: {}", metadata_path.display());

                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Handle query / list (semantic model query)
    #[cfg(feature = "interchange")]
    if cli.query.is_some() || cli.list {
        let name_filter = cli.query.as_deref();
        let kind_filter = cli.kind.as_deref();

        match query_model(
            &cli.input,
            name_filter,
            kind_filter,
            None, // qualified name — use --inspect for that
            cli.verbose,
        ) {
            Ok(result) => {
                if cli.json {
                    let json = serde_json::to_string_pretty(&result)
                        .expect("failed to serialize query result");
                    write_output(&json, cli.output.as_ref());
                } else {
                    println!("Found {} element(s):", result.match_count);
                    for el in &result.elements {
                        let kind = &el.kind;
                        let name = el.name.as_deref().unwrap_or("(anonymous)");
                        let qn = el
                            .qualified_name
                            .as_deref()
                            .map(|q| format!("  ({})", q))
                            .unwrap_or_default();
                        let typing = if !el.typed_by.is_empty() {
                            format!(" : {}", el.typed_by.join(", "))
                        } else {
                            String::new()
                        };
                        let supers = if !el.supertypes.is_empty() {
                            format!(" :> {}", el.supertypes.join(", "))
                        } else {
                            String::new()
                        };
                        let abs = if el.is_abstract { " [abstract]" } else { "" };
                        let members = if el.owned_member_count > 0 {
                            format!(" ({} members)", el.owned_member_count)
                        } else {
                            String::new()
                        };
                        println!("  {kind} {name}{typing}{supers}{abs}{members}{qn}");
                    }
                }
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Handle inspect (detailed element view)
    #[cfg(feature = "interchange")]
    if let Some(ref target) = cli.inspect {
        match inspect_element(&cli.input, target, cli.verbose) {
            Ok(result) => {
                if cli.json {
                    let json = serde_json::to_string_pretty(&result)
                        .expect("failed to serialize inspect result");
                    write_output(&json, cli.output.as_ref());
                } else {
                    let el = &result.element;
                    let name = el.name.as_deref().unwrap_or("(anonymous)");
                    println!("{} {}", el.kind, name);
                    if let Some(qn) = &el.qualified_name {
                        println!("  qualified: {}", qn);
                    }
                    if let Some(owner) = &el.owner {
                        println!("  owner: {}", owner);
                    }
                    if el.is_abstract {
                        println!("  abstract: true");
                    }
                    if !el.typed_by.is_empty() {
                        println!("  typed by: {}", el.typed_by.join(", "));
                    }
                    if !el.supertypes.is_empty() {
                        println!("  specializes: {}", el.supertypes.join(", "));
                    }
                    if let Some(doc) = &el.documentation {
                        println!("  doc: {}", doc);
                    }

                    if !result.children.is_empty() {
                        println!("\n  Children ({}):", result.children.len());
                        for child in &result.children {
                            let cn = child.name.as_deref().unwrap_or("(anonymous)");
                            let ct = if !child.typed_by.is_empty() {
                                format!(": {}", child.typed_by.join(", "))
                            } else {
                                String::new()
                            };
                            println!("    {} {}{}", child.kind, cn, ct);
                        }
                    }

                    if !result.relationships_from.is_empty() {
                        println!(
                            "\n  Relationships from ({}):",
                            result.relationships_from.len()
                        );
                        for rel in &result.relationships_from {
                            println!("    {} -> {} ({})", rel.source, rel.target, rel.kind);
                        }
                    }

                    if !result.relationships_to.is_empty() {
                        println!("\n  Relationships to ({}):", result.relationships_to.len());
                        for rel in &result.relationships_to {
                            println!("    {} -> {} ({})", rel.source, rel.target, rel.kind);
                        }
                    }
                }
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Handle rename (semantic rename + re-render)
    #[cfg(feature = "interchange")]
    if let Some(ref rename_spec) = cli.rename {
        let parts: Vec<&str> = rename_spec.splitn(2, '=').collect();
        if parts.len() != 2 {
            eprintln!("error: --rename requires OLD=NEW format (e.g., --rename Vehicle=Car)");
            return ExitCode::FAILURE;
        }
        let old_name = parts[0];
        let new_name = parts[1];

        match rename_element(&cli.input, old_name, new_name, cli.verbose) {
            Ok(result) => {
                let out_path = cli.output.clone().unwrap_or_else(|| cli.input.clone());

                // Write SysML text
                if cli.output.is_some() {
                    write_output(&result.rendered_text, cli.output.as_ref());
                } else if let Err(e) = std::fs::write(&out_path, &result.rendered_text) {
                    eprintln!("error: failed to write {}: {}", out_path.display(), e);
                    return ExitCode::FAILURE;
                }

                eprintln!(
                    "✓ Renamed '{}' -> '{}', wrote to {}",
                    result.old_name,
                    result.new_name,
                    out_path.display()
                );

                // Write updated metadata JSON alongside
                if let Some(metadata_json) = &result.metadata_json {
                    let metadata_path = out_path.with_extension("metadata.json");
                    if let Err(e) = std::fs::write(&metadata_path, metadata_json) {
                        eprintln!(
                            "warning: failed to write metadata {}: {}",
                            metadata_path.display(),
                            e
                        );
                    } else {
                        eprintln!("  Wrote: {}", metadata_path.display());
                    }
                }

                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Handle add-member (add a child element)
    #[cfg(feature = "interchange")]
    if let Some(ref add_spec) = cli.add_member {
        // Format: PARENT:KIND:NAME or PARENT:KIND:NAME:TYPE
        let parts: Vec<&str> = add_spec.splitn(4, ':').collect();
        if parts.len() < 3 {
            eprintln!("error: --add-member requires PARENT:KIND:NAME[:TYPE] format");
            eprintln!("  Example: --add-member Car:PartUsage:turbo");
            eprintln!("  Example: --add-member Car:PartUsage:turbo:Engine");
            return ExitCode::FAILURE;
        }
        let parent = parts[0];
        let kind = parts[1];
        let name = parts[2];
        let type_name = parts.get(3).copied();

        match add_member(&cli.input, parent, name, kind, type_name, cli.verbose) {
            Ok(result) => {
                let out_path = cli.output.clone().unwrap_or_else(|| cli.input.clone());

                if cli.output.is_some() {
                    write_output(&result.rendered_text, cli.output.as_ref());
                } else if let Err(e) = std::fs::write(&out_path, &result.rendered_text) {
                    eprintln!("error: failed to write {}: {}", out_path.display(), e);
                    return ExitCode::FAILURE;
                }

                eprintln!(
                    "✓ Added {} '{}' to '{}', wrote to {}",
                    result.member_kind,
                    result.member_name,
                    result.parent_name,
                    out_path.display()
                );

                if let Some(metadata_json) = &result.metadata_json {
                    let metadata_path = out_path.with_extension("metadata.json");
                    if let Err(e) = std::fs::write(&metadata_path, metadata_json) {
                        eprintln!(
                            "warning: failed to write metadata {}: {}",
                            metadata_path.display(),
                            e
                        );
                    } else {
                        eprintln!("  Wrote: {}", metadata_path.display());
                    }
                }

                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Handle remove-member (remove an element)
    #[cfg(feature = "interchange")]
    if let Some(ref target) = cli.remove_member {
        match remove_member(&cli.input, target, cli.verbose) {
            Ok(result) => {
                let out_path = cli.output.clone().unwrap_or_else(|| cli.input.clone());

                if cli.output.is_some() {
                    write_output(&result.rendered_text, cli.output.as_ref());
                } else if let Err(e) = std::fs::write(&out_path, &result.rendered_text) {
                    eprintln!("error: failed to write {}: {}", out_path.display(), e);
                    return ExitCode::FAILURE;
                }

                eprintln!(
                    "✓ Removed '{}', wrote to {}",
                    result.removed_name,
                    out_path.display()
                );

                if let Some(metadata_json) = &result.metadata_json {
                    let metadata_path = out_path.with_extension("metadata.json");
                    if let Err(e) = std::fs::write(&metadata_path, metadata_json) {
                        eprintln!(
                            "warning: failed to write metadata {}: {}",
                            metadata_path.display(),
                            e
                        );
                    } else {
                        eprintln!("  Wrote: {}", metadata_path.display());
                    }
                }

                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Handle interchange import (validate only)
    #[cfg(feature = "interchange")]
    if cli.import {
        match import_model(&cli.input, None, cli.verbose) {
            Ok(result) => {
                println!(
                    "✓ Imported {} elements, {} relationships",
                    result.element_count, result.relationship_count
                );
                if result.error_count > 0 || result.warning_count > 0 {
                    let total = result.error_count + result.warning_count;
                    eprintln!("  {} validation issue(s):", total);
                    for msg in &result.messages {
                        eprintln!("    {}", msg);
                    }
                }
                if result.error_count > 0 {
                    return ExitCode::FAILURE;
                }
                if result.warning_count > 0 {
                    return ExitCode::from(2);
                }
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Handle import into workspace (for analysis) + optional export
    #[cfg(feature = "interchange")]
    if cli.import_workspace {
        use syster::ide::AnalysisHost;
        use syster::project::StdLibLoader;
        use syster_cli::export_from_host;

        let mut host = AnalysisHost::new();

        // Load stdlib if enabled
        if !cli.no_stdlib {
            let mut loader = StdLibLoader::new();
            if let Err(e) = loader.ensure_loaded_into_host(&mut host) {
                eprintln!("warning: failed to load stdlib: {}", e);
            }
        }

        // Import the XMI/KPAR model into workspace
        match import_model_into_host(&mut host, &cli.input, None, cli.verbose) {
            Ok(result) => {
                // If --export is also specified, export from the imported workspace
                if let Some(format) = &cli.export {
                    // Use stderr for status when exporting (stdout is for data)
                    eprintln!(
                        "✓ Imported {} elements ({} symbols) into workspace",
                        result.element_count, result.element_count
                    );

                    let analysis = host.analysis();
                    let all_symbols: Vec<_> = analysis.symbol_index().all_symbols().collect();
                    eprintln!("  Total symbols in workspace: {}", all_symbols.len());
                    eprintln!("  Element IDs preserved: ✓");

                    let format_str = match format {
                        InterchangeFormat::Xmi => "xmi",
                        InterchangeFormat::Kpar => "kpar",
                        InterchangeFormat::JsonLd => "jsonld",
                        InterchangeFormat::Yaml => "yaml",
                    };

                    match export_from_host(&mut host, format_str, cli.verbose, cli.self_contained) {
                        Ok(bytes) => {
                            write_bytes_output(&bytes, cli.output.as_ref());
                            return ExitCode::SUCCESS;
                        }
                        Err(e) => {
                            eprintln!("error: {}", e);
                            return ExitCode::FAILURE;
                        }
                    }
                }

                // No export - use stdout for status
                println!(
                    "✓ Imported {} elements ({} symbols) into workspace",
                    result.element_count, result.element_count
                );

                // Run analysis on imported symbols
                let analysis = host.analysis();
                let all_symbols: Vec<_> = analysis.symbol_index().all_symbols().collect();

                println!("  Total symbols in workspace: {}", all_symbols.len());
                println!("  Element IDs preserved: ✓");

                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Handle interchange export
    #[cfg(feature = "interchange")]
    if let Some(format) = &cli.export {
        let format_str = match format {
            InterchangeFormat::Xmi => "xmi",
            InterchangeFormat::Kpar => "kpar",
            InterchangeFormat::JsonLd => "jsonld",
            InterchangeFormat::Yaml => "yaml",
        };

        match export_model(
            &cli.input,
            format_str,
            cli.verbose,
            !cli.no_stdlib,
            cli.stdlib_path.as_deref(),
            cli.self_contained,
        ) {
            Ok(bytes) => {
                write_bytes_output(&bytes, cli.output.as_ref());
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Handle AST export
    if cli.export_ast {
        match export_ast(
            &cli.input,
            cli.verbose,
            !cli.no_stdlib,
            cli.stdlib_path.as_deref(),
        ) {
            Ok(ast_output) => {
                write_output(&ast_output, cli.output.as_ref());
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    match run_analysis(
        &cli.input,
        cli.verbose,
        !cli.no_stdlib,
        cli.stdlib_path.as_deref(),
    ) {
        Ok(result) => {
            // Handle JSON export
            if cli.json {
                match export_json(&result) {
                    Ok(json) => {
                        write_output(&json, cli.output.as_ref());
                        return if result.error_count == 0 {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::FAILURE
                        };
                    }
                    Err(e) => {
                        eprintln!("error: {}", e);
                        return ExitCode::FAILURE;
                    }
                }
            }

            // Print diagnostics (normal mode)
            for diag in &result.diagnostics {
                print_diagnostic(diag);
            }

            // Print summary
            if result.error_count == 0 {
                println!(
                    "✓ Analyzed {} files: {} symbols, {} warnings",
                    result.file_count, result.symbol_count, result.warning_count
                );
                ExitCode::SUCCESS
            } else {
                eprintln!(
                    "✗ Analyzed {} files: {} errors, {} warnings",
                    result.file_count, result.error_count, result.warning_count
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

/// Write output to file or stdout
fn write_output(content: &str, output_path: Option<&PathBuf>) {
    match output_path {
        Some(path) => {
            if let Err(e) = std::fs::write(path, content) {
                eprintln!("error: failed to write output: {}", e);
            }
        }
        None => {
            println!("{}", content);
        }
    }
}

/// Write binary output to file or stdout
fn write_bytes_output(content: &[u8], output_path: Option<&PathBuf>) {
    use std::io::Write;
    match output_path {
        Some(path) => {
            if let Err(e) = std::fs::write(path, content) {
                eprintln!("error: failed to write output: {}", e);
            }
        }
        None => {
            // For stdout, try to write as string if valid UTF-8, otherwise raw bytes
            if let Ok(s) = std::str::from_utf8(content) {
                println!("{}", s);
            } else {
                let mut stdout = std::io::stdout();
                if let Err(e) = stdout.write_all(content) {
                    eprintln!("error: failed to write output: {}", e);
                }
            }
        }
    }
}

/// Print a diagnostic message in a compiler-like format.
fn print_diagnostic(diag: &DiagnosticInfo) {
    let prefix = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
        Severity::Hint => "hint",
    };

    let code_suffix = diag
        .code
        .as_ref()
        .map(|c| format!("[{}]", c))
        .unwrap_or_default();

    eprintln!(
        "{}{}: {}:{}:{}: {}",
        prefix, code_suffix, diag.file, diag.line, diag.col, diag.message
    );
}
