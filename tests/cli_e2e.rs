use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Creates a unique temporary directory path for integration tests.
fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX_EPOCH")
        .as_nanos();

    std::env::temp_dir().join(format!("marie_c_compiler_test_{nanos}"))
}

/// Verifies the CLI compiles an input file and writes default `.mas` output.
#[test]
fn cli_compiles_input_to_default_mas_file() {
    let bin = env!("CARGO_BIN_EXE_marie-c-compiler");
    let temp_dir = unique_temp_dir();
    fs::create_dir_all(&temp_dir).expect("should create temp directory");

    let input_path = temp_dir.join("hello.i");
    fs::write(&input_path, "int main(void) { return 0; }").expect("should write input file");

    let output = Command::new(bin)
        .arg(&input_path)
        .output()
        .expect("should execute compiler binary");

    assert!(
        output.status.success(),
        "compiler failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_path = temp_dir.join("hello.mas");
    assert!(
        output_path.exists(),
        "expected output file at {output_path:?}"
    );

    let output_contents = fs::read_to_string(&output_path).expect("should read output file");
    assert!(output_contents.contains("HALT"));

    fs::remove_dir_all(&temp_dir).expect("should clean temp directory");
}
