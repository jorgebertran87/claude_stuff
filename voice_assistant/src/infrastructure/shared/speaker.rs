//! Shared speaker utilities: Bluetooth speaker management.

use std::process::{Command, Stdio};

/// Disconnect the Bluetooth speaker specified by `BT_SPEAKER_MAC`.
pub fn disconnect_bt_speaker() {
    let mac = match std::env::var("BT_SPEAKER_MAC") {
        Ok(m) if !m.is_empty() => m,
        _ => {
            eprintln!("[bt: BT_SPEAKER_MAC not set, skipping disconnect]");
            return;
        }
    };
    eprintln!("[bt: disconnecting {mac} after inactivity]");
    let _ = Command::new("bluetoothctl")
        .args(["disconnect", &mac])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
