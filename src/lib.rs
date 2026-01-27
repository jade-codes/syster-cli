use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::{StdLibLoader, WorkspaceLoader};

#[derive(Debug)]
pub struct AnalysisResult {
    pub file_count: usize,
    pub symbol_count: usize,
}

pub fn run_analysis(
    input: &PathBuf,
    verbose: bool,
    load_stdlib: bool,
    stdlib_path: Option<&PathBuf>,
) -> Result<AnalysisResult, String> {
    if verbose {
        println!("Analyzing: {}", input.display());
    }

    let loader = WorkspaceLoader::new();
    let mut host = AnalysisHost::new();

    if load_stdlib {
        if let Some(path) = stdlib_path {
            let mut stdlib_loader = StdLibLoader::with_path(path.clone());
            if let Err(err) = stdlib_loader.ensure_loaded_into_host(&mut host) {
                return Err(format!("Error loading stdlib: {}", err));
            }
        }
    }

    if input.is_file() {
        loader.load_file_into_host(input, &mut host)?
    } else if input.is_dir() {
        loader.load_directory_into_host(input, &mut host)?
    } else {
        return Err(format!("Input path does not exist: {}", input.display()));
    }

    let analysis = host.analysis();

    if verbose {
        println!("Populating symbol tables...");
    }

    let symbol_count = analysis.symbol_index().all_symbols().count();
    let file_count = host.file_count();

    Ok(AnalysisResult {
        file_count,
        symbol_count,
    })
}
