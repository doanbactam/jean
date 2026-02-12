//! Configuration and path management for the embedded Codex CLI

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Directory name for storing the Codex CLI binary
pub const CODEX_CLI_DIR_NAME: &str = "codex-cli";

/// Name of the Codex CLI binary
#[cfg(not(target_os = "windows"))]
pub const CODEX_CLI_BINARY_NAME: &str = "codex";

#[cfg(target_os = "windows")]
pub const CODEX_CLI_BINARY_NAME: &str = "codex.exe";

/// Get the directory where Codex CLI is installed
///
/// Returns: `~/Library/Application Support/jean/codex-cli/` (macOS)
///          `~/.local/share/jean/codex-cli/` (Linux)
///          `%APPDATA%/jean/codex-cli/` (Windows)
pub fn get_codex_cli_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;
    Ok(app_data_dir.join(CODEX_CLI_DIR_NAME))
}

/// Get the full path to the Codex CLI binary
///
/// Returns: `~/Library/Application Support/jean/codex-cli/codex` (macOS/Linux)
///          `%APPDATA%/jean/codex-cli/codex.exe` (Windows)
pub fn get_codex_cli_binary_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(get_codex_cli_dir(app)?.join(CODEX_CLI_BINARY_NAME))
}

/// Resolve the `codex` binary to use for commands.
///
/// Returns the embedded binary path if it exists, otherwise falls back to `"codex"` from PATH.
/// This ensures commands work whether `codex` was installed via the app or system-wide.
pub fn resolve_codex_binary(app: &AppHandle) -> PathBuf {
    if let Ok(embedded) = get_codex_cli_binary_path(app) {
        if embedded.exists() {
            return embedded;
        }
    }
    PathBuf::from("codex")
}

/// Ensure the CLI directory exists, creating it if necessary
pub fn ensure_codex_cli_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let cli_dir = get_codex_cli_dir(app)?;
    std::fs::create_dir_all(&cli_dir)
        .map_err(|e| format!("Failed to create Codex CLI directory: {e}"))?;
    Ok(cli_dir)
}
