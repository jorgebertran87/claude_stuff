//! Integration tests for the --order flag.
//! Spawns the real binary and inspects stdout/stderr/exit-code.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_voice_assistant");

// ── helpers ───────────────────────────────────────────────────────────────────

fn run(extra_args: &[&str]) -> std::process::Output {
    Command::new(BIN)
        .args(extra_args)
        .output()
        .expect("failed to spawn voice_assistant binary")
}

// ── --order flag ──────────────────────────────────────────────────────────────

#[test]
fn order_flag_prints_order_line_to_stdout() {
    let output = run(&["--order", "listar archivos"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Order: \"listar archivos\""),
        "expected 'Order: ...' in stdout, got: {stdout}"
    );
}

#[test]
fn order_flag_prints_claudito_response_to_stdout() {
    let output = run(&["--order", "listar archivos"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Claudito:"),
        "expected 'Claudito:' in stdout, got: {stdout}"
    );
}

#[test]
fn order_flag_exits_without_blocking_in_listen_loop() {
    // If it entered the listen loop it would block forever waiting for audio.
    // Simply completing means it took the direct-order path and returned.
    let output = run(&["--order", "test order"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Order:"), "stdout: {stdout}");
}

#[test]
fn order_flag_exits_with_code_zero() {
    let output = run(&["--order", "hola"]);
    assert!(
        output.status.success(),
        "expected exit code 0, got: {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );
}

// ── missing value error ───────────────────────────────────────────────────────

#[test]
fn order_flag_without_value_exits_nonzero() {
    let output = run(&["--order"]);
    assert!(
        !output.status.success(),
        "expected non-zero exit code when --order has no value"
    );
}

#[test]
fn order_flag_without_value_prints_error_to_stderr() {
    let output = run(&["--order"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--order"),
        "expected error mentioning '--order' in stderr, got: {stderr}"
    );
}
