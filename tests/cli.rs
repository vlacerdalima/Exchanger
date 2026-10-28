use std::process::Command;

#[test]
fn parser_prints_orders_and_errors() {
    let output = Command::new(env!("CARGO_BIN_EXE_matching-engine"))
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/parser.txt"))
        .output()
        .expect("failed to run the binary");

    assert!(output.status.success());
    insta::assert_snapshot!("stdout", String::from_utf8_lossy(&output.stdout));
    insta::assert_snapshot!("stderr", String::from_utf8_lossy(&output.stderr));
}
