//! syster CLI - Command-line interface for SysML v2 and KerML analysis

use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;
use syster::hir::Severity;
use syster_cli::{DiagnosticInfo, export_ast, export_json, run_analysis};
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

    /// Write output to file instead of stdout
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.verbose {
        println!("Analyzing: {}", cli.input.display());
    }

    // Handle decompile (convert XMI to SysML text)
    #[cfg(feature = "interchange")]
    if cli.decompile {
        match decompile_model(&cli.input, None, cli.verbose) {
            Ok(result) => {
                println!(
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
                println!("  Wrote: {}", sysml_path.display());

                // Write metadata file
                let metadata_path = sysml_path.with_extension("metadata.json");
                if let Err(e) = std::fs::write(&metadata_path, &result.metadata_json) {
                    eprintln!("error: failed to write {}: {}", metadata_path.display(), e);
                    return ExitCode::FAILURE;
                }
                println!("  Wrote: {}", metadata_path.display());

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
                if result.error_count > 0 {
                    eprintln!("  {} validation issues:", result.error_count);
                    for msg in &result.messages {
                        eprintln!("    {}", msg);
                    }
                    return ExitCode::FAILURE;
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
