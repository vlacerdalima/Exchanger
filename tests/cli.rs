use std::process::Command;

#[test]
fn basic_example_prints_events() {
    let output = Command::new(env!("CARGO_BIN_EXE_matching-engine"))
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/basic.txt"))
        .output()
        .expect("failed to run the binary");

    assert!(output.status.success());
    insta::assert_snapshot!("basic_stdout", String::from_utf8_lossy(&output.stdout));
}
