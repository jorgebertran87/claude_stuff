//! Unit tests for CLI argument parsing.
//! Detroit School: pure-function tests, no mocking needed.

use voice_assistant::cli::{parse_args, CliArgs};

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// ── ListenMode ────────────────────────────────────────────────────────────────

#[test]
fn returns_listen_mode_when_no_flags_given() {
    let result = parse_args(&args(&["voice_assistant"])).unwrap();
    assert!(matches!(result, CliArgs::ListenMode));
}

#[test]
fn returns_listen_mode_when_args_is_empty() {
    let result = parse_args(&[]).unwrap();
    assert!(matches!(result, CliArgs::ListenMode));
}

#[test]
fn returns_listen_mode_when_unrelated_flags_given() {
    let result = parse_args(&args(&["voice_assistant", "--verbose"])).unwrap();
    assert!(matches!(result, CliArgs::ListenMode));
}

// ── DirectOrder ───────────────────────────────────────────────────────────────

#[test]
fn returns_direct_order_when_flag_and_value_present() {
    let result = parse_args(&args(&["voice_assistant", "--order", "pon música"])).unwrap();
    match result {
        CliArgs::DirectOrder(o) => assert_eq!(o, "pon música"),
        CliArgs::ListenMode => panic!("expected DirectOrder"),
    }
}

#[test]
fn direct_order_value_preserves_spaces_and_unicode() {
    let result = parse_args(&args(&["voice_assistant", "--order", "¿qué tiempo hace hoy?"])).unwrap();
    match result {
        CliArgs::DirectOrder(o) => assert_eq!(o, "¿qué tiempo hace hoy?"),
        CliArgs::ListenMode => panic!("expected DirectOrder"),
    }
}

#[test]
fn order_flag_found_among_other_flags() {
    let result = parse_args(&args(&[
        "voice_assistant", "--verbose", "--order", "listar archivos", "--dry-run",
    ])).unwrap();
    match result {
        CliArgs::DirectOrder(o) => assert_eq!(o, "listar archivos"),
        CliArgs::ListenMode => panic!("expected DirectOrder"),
    }
}

// ── TelegramMode ──────────────────────────────────────────────────────────────

#[test]
fn returns_telegram_mode_when_telegram_flag_given() {
    let result = parse_args(&args(&["voice_assistant", "--telegram"])).unwrap();
    assert!(matches!(result, CliArgs::TelegramMode));
}

#[test]
fn telegram_flag_works_among_other_unrelated_flags() {
    let result = parse_args(&args(&["voice_assistant", "--verbose", "--telegram"])).unwrap();
    assert!(matches!(result, CliArgs::TelegramMode));
}

#[test]
fn order_flag_takes_priority_over_telegram_flag() {
    let result = parse_args(&args(&[
        "voice_assistant", "--order", "test", "--telegram"
    ])).unwrap();
    assert!(matches!(result, CliArgs::DirectOrder(_)));
}

// ── Error cases ───────────────────────────────────────────────────────────────

#[test]
fn returns_error_when_order_flag_has_no_value() {
    let result = parse_args(&args(&["voice_assistant", "--order"]));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--order"));
}
