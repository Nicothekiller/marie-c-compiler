use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use marie_c_compiler::compiler::Compiler;

#[derive(Parser, Debug)]
#[command(name = "marie-c-compiler")]
#[command(about = "Compile preprocessed C into Marie assembly")]
struct Cli {
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,
}

/// Entry point for the command-line compiler executable.
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

/// Parses CLI arguments, compiles input source, and writes the output file.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let output_path = cli
        .output
        .unwrap_or_else(|| default_output_path(&cli.input));

    let source = fs::read_to_string(&cli.input)?;
    let compiler = Compiler::new();
    let marie_asm = compiler.compile_source(&source)?;

    fs::write(&output_path, marie_asm)?;
    println!("wrote {}", output_path.display());

    Ok(())
}

/// Returns the default output path by replacing the input extension with `.mas`.
fn default_output_path(input_path: &Path) -> PathBuf {
    let mut output = input_path.to_path_buf();
    output.set_extension("mas");
    output
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::default_output_path;

    /// Verifies default output path uses the `.mas` extension.
    #[test]
    fn default_output_extension_is_mas() {
        let output = default_output_path(Path::new("examples/hello.i"));
        assert_eq!(output, Path::new("examples/hello.mas"));
    }
}
