use std::process::Command;

#[test]
fn echo_imprime_linhas_nao_vazias() {
    let output = Command::new(env!("CARGO_BIN_EXE_matching-engine"))
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/echo.txt"))
        .output()
        .expect("falha ao executar o binário");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    insta::assert_snapshot!(String::from_utf8_lossy(&output.stdout));
}
