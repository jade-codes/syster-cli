//! syster CLI - Command-line interface for SysML v2 and KerML analysis

use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;
use syster::hir::Severity;
use syster_cli::{run_analysis, export_ast, export_json, DiagnosticInfo};
#[cfg(feature = "interchange")]
use syster_cli::{export_model, import_model};

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

    /// Write output to file instead of stdout
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.verbose {
        println!("Analyzing: {}", cli.input.display());
    }

    // Handle interchange import
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

    // Handle interchange export
    #[cfg(feature = "interchange")]
    if let Some(format) = &cli.export {
        let format_str = match format {
            InterchangeFormat::Xmi => "xmi",
            InterchangeFormat::Kpar => "kpar",
            InterchangeFormat::JsonLd => "jsonld",
        };
        
        match export_model(&cli.input, format_str, cli.verbose, !cli.no_stdlib, cli.stdlib_path.as_ref()) {
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
        match export_ast(&cli.input, cli.verbose, !cli.no_stdlib, cli.stdlib_path.as_ref()) {
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
        cli.stdlib_path.as_ref(),
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
