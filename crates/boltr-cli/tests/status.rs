use std::process::Command;

#[test]
fn status_creates_and_reads_empty_store() {
    let data_dir = std::env::temp_dir().join(format!(
        "boltr-cli-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&data_dir).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_boltr-cli"))
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Total artifacts: 0"));
}
