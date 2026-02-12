//! Tauri commands for Codex CLI management

use crate::platform::silent_command;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::config::{ensure_codex_cli_dir, get_codex_cli_binary_path};
use crate::http_server::EmitExt;

/// GitHub API URL for OpenAI Codex releases
const CODEX_RELEASES_API: &str = "https://api.github.com/repos/openai/codex/releases";

/// Extract semver version number from a version string
/// Handles formats like: "1.0.0", "v1.0.0", "codex 1.0.0"
fn extract_version_number(version_str: &str) -> String {
    // Try to find a semver-like pattern (digits.digits.digits)
    for word in version_str.split_whitespace() {
        let trimmed = word.trim_start_matches('v');
        // Check if it looks like a version number (starts with digit, contains dots)
        if trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && trimmed.contains('.')
        {
            return trimmed.to_string();
        }
    }
    // Fallback: return original string
    version_str.to_string()
}

/// Status of the Codex CLI installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexCliStatus {
    /// Whether Codex CLI is installed
    pub installed: bool,
    /// Installed version (if any)
    pub version: Option<String>,
    /// Path to the CLI binary (if installed)
    pub path: Option<String>,
}

/// Information about a Codex CLI release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexReleaseInfo {
    /// Version string (e.g., "1.0.0")
    pub version: String,
    /// Git tag name (e.g., "v1.0.0")
    pub tag_name: String,
    /// Publication date in ISO format
    pub published_at: String,
    /// Whether this is a prerelease
    pub prerelease: bool,
}

/// Progress event for CLI installation
#[derive(Debug, Clone, Serialize)]
pub struct CodexInstallProgress {
    /// Current stage of installation
    pub stage: String,
    /// Progress message
    pub message: String,
    /// Percentage complete (0-100)
    pub percent: u8,
}

/// GitHub API release response structure
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    published_at: String,
    prerelease: bool,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Check if Codex CLI is installed and get its status
#[tauri::command]
pub async fn check_codex_cli_installed(app: AppHandle) -> Result<CodexCliStatus, String> {
    log::trace!("Checking Codex CLI installation status");

    let binary_path = get_codex_cli_binary_path(&app)?;

    if !binary_path.exists() {
        log::trace!("Codex CLI not found at {:?}", binary_path);
        return Ok(CodexCliStatus {
            installed: false,
            version: None,
            path: None,
        });
    }

    // Try to get the version by running codex --version
    // Use the binary directly - shell wrapper causes PowerShell parsing issues on Windows
    let version = match silent_command(&binary_path).arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                log::trace!("Codex CLI raw version output: {}", version_str);
                let version = extract_version_number(&version_str);
                log::trace!("Codex CLI parsed version: {}", version);
                Some(version)
            } else {
                log::warn!("Failed to get Codex CLI version");
                None
            }
        }
        Err(e) => {
            log::warn!("Failed to execute Codex CLI: {}", e);
            None
        }
    };

    Ok(CodexCliStatus {
        installed: true,
        version,
        path: Some(binary_path.to_string_lossy().to_string()),
    })
}

/// Get available Codex CLI versions from GitHub releases API
#[tauri::command]
pub async fn get_available_codex_versions() -> Result<Vec<CodexReleaseInfo>, String> {
    log::trace!("Fetching available Codex CLI versions from GitHub API");

    let client = reqwest::Client::builder()
        .user_agent("Jean-App/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get(CODEX_RELEASES_API)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch releases: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let releases: Vec<GitHubRelease> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub API response: {e}"))?;

    // Convert to our format, filtering to releases with assets for our platform
    let versions: Vec<CodexReleaseInfo> = releases
        .into_iter()
        .filter(|r| !r.assets.is_empty())
        .take(5) // Only take 5 most recent
        .map(|r| {
            // Remove 'v' prefix from tag_name for version
            let version = r
                .tag_name
                .strip_prefix('v')
                .unwrap_or(&r.tag_name)
                .to_string();
            CodexReleaseInfo {
                version,
                tag_name: r.tag_name,
                published_at: r.published_at,
                prerelease: r.prerelease,
            }
        })
        .collect();

    log::trace!("Found {} Codex CLI versions", versions.len());
    Ok(versions)
}

/// Get the platform string for the current system (for Codex releases)
fn get_codex_platform() -> Result<(&'static str, &'static str), String> {
    // Returns (platform_string, archive_extension)
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok(("aarch64-apple-darwin", "tar.gz"));
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Ok(("x86_64-apple-darwin", "tar.gz"));
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Ok(("x86_64-unknown-linux-musl", "tar.gz"));
    }

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return Ok(("aarch64-unknown-linux-musl", "tar.gz"));
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Ok(("x86_64-pc-windows-msvc", "zip"));
    }

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        return Ok(("aarch64-pc-windows-msvc", "zip"));
    }

    #[allow(unreachable_code)]
    Err("Unsupported platform".to_string())
}

/// Install Codex CLI by downloading from GitHub releases
#[tauri::command]
pub async fn install_codex_cli(app: AppHandle, version: Option<String>) -> Result<(), String> {
    log::trace!("Installing Codex CLI, version: {:?}", version);

    // Check if any Codex processes are running - cannot replace binary while in use
    let running_sessions = crate::chat::registry::get_running_sessions();
    if !running_sessions.is_empty() {
        let count = running_sessions.len();
        return Err(format!(
            "Cannot install Codex CLI while {} Claude {} running. Please stop all active sessions first.",
            count,
            if count == 1 { "session is" } else { "sessions are" }
        ));
    }

    let cli_dir = ensure_codex_cli_dir(&app)?;
    let binary_path = get_codex_cli_binary_path(&app)?;

    // Emit progress: starting
    emit_progress(&app, "starting", "Preparing installation...", 0);

    // Determine version (use provided or fetch latest)
    let version = match version {
        Some(v) => v,
        None => fetch_latest_codex_version().await?,
    };

    // Detect platform
    let (platform, archive_ext) = get_codex_platform()?;
    log::trace!("Installing version {} for platform {}", version, platform);

    // Build download URL
    // Format: https://github.com/openai/codex/releases/download/v{version}/codex-{platform}.{ext}
    let archive_name = format!("codex-{}.{}", platform, archive_ext);
    let download_url = format!(
        "https://github.com/openai/codex/releases/download/v{}/{}",
        version, archive_name
    );
    log::trace!("Downloading from: {}", download_url);

    // Emit progress: downloading
    emit_progress(&app, "downloading", "Downloading Codex CLI...", 20);

    // Download the archive
    let client = reqwest::Client::builder()
        .user_agent("Jean-App/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download Codex CLI: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download Codex CLI: HTTP {}",
            response.status()
        ));
    }

    let archive_content = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read archive content: {e}"))?;

    log::trace!("Downloaded {} bytes", archive_content.len());

    // Emit progress: extracting
    emit_progress(&app, "extracting", "Extracting archive...", 40);

    // Create temp directory for extraction
    let temp_dir = cli_dir.join("temp");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {e}"))?;

    // Extract the archive
    let extracted_binary_path = if archive_ext == "zip" {
        extract_zip(&archive_content, &temp_dir, &version, platform)?
    } else {
        extract_tar_gz(&archive_content, &temp_dir, &version, platform)?
    };

    // Emit progress: installing
    emit_progress(&app, "installing", "Installing Codex CLI...", 60);

    // Move binary to final location
    std::fs::copy(&extracted_binary_path, &binary_path)
        .map_err(|e| format!("Failed to copy binary: {e}"))?;

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Emit progress: verifying
    emit_progress(&app, "verifying", "Verifying installation...", 80);

    // Make sure the binary is executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&binary_path)
            .map_err(|e| format!("Failed to get binary metadata: {e}"))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&binary_path, perms)
            .map_err(|e| format!("Failed to set binary permissions: {e}"))?;
    }

    // Verify the binary works
    log::trace!("Verifying binary at {:?}", binary_path);
    let version_output = silent_command(&binary_path)
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to verify Codex CLI: {e}"))?;

    if !version_output.status.success() {
        let stderr = String::from_utf8_lossy(&version_output.stderr);
        let stdout = String::from_utf8_lossy(&version_output.stdout);
        log::error!(
            "Codex CLI verification failed - exit code: {:?}, stdout: {}, stderr: {}",
            version_output.status.code(),
            stdout,
            stderr
        );
        return Err(format!(
            "Codex CLI binary verification failed: {}",
            if !stderr.is_empty() {
                stderr.to_string()
            } else {
                "Unknown error".to_string()
            }
        ));
    }

    let installed_version = String::from_utf8_lossy(&version_output.stdout)
        .trim()
        .to_string();
    log::trace!("Verified Codex CLI version: {}", installed_version);

    // Remove macOS quarantine attribute to allow execution
    #[cfg(target_os = "macos")]
    {
        log::trace!("Removing quarantine attribute from {:?}", binary_path);
        let _ = silent_command("xattr")
            .args(["-d", "com.apple.quarantine"])
            .arg(&binary_path)
            .output();
    }

    // Emit progress: complete
    emit_progress(&app, "complete", "Installation complete!", 100);

    log::trace!("Codex CLI installed successfully at {:?}", binary_path);
    Ok(())
}

/// Fetch the latest Codex CLI version from GitHub API
async fn fetch_latest_codex_version() -> Result<String, String> {
    log::trace!("Fetching latest Codex CLI version");

    let client = reqwest::Client::builder()
        .user_agent("Jean-App/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get(format!("{CODEX_RELEASES_API}/latest"))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch latest release: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch latest release: HTTP {}",
            response.status()
        ));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {e}"))?;

    let version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();
    log::trace!("Latest Codex CLI version: {}", version);
    Ok(version)
}

/// Extract codex binary from a zip archive (Windows)
fn extract_zip(
    archive_content: &[u8],
    temp_dir: &std::path::Path,
    _version: &str,
    platform: &str,
) -> Result<std::path::PathBuf, String> {
    use std::io::Cursor;

    let cursor = Cursor::new(archive_content);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to open zip archive: {e}"))?;

    // Extract all files
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {e}"))?;

        let outpath = match file.enclosed_name() {
            Some(path) => temp_dir.join(path),
            None => continue,
        };

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)
                        .map_err(|e| format!("Failed to create parent directory: {e}"))?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {e}"))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {e}"))?;
        }
    }

    // The binary is named codex-{platform} (e.g., codex-x86_64-pc-windows-msvc.exe)
    #[cfg(not(target_os = "windows"))]
    let binary_name = format!("codex-{}", platform);
    #[cfg(target_os = "windows")]
    let binary_name = format!("codex-{}.exe", platform);

    let binary_path = temp_dir.join(&binary_name);

    if !binary_path.exists() {
        // Fallback: try to find any executable in the archive
        for entry in std::fs::read_dir(temp_dir)
            .map_err(|e| format!("Failed to read temp directory: {e}"))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                #[cfg(not(target_os = "windows"))]
                let is_executable = true;
                #[cfg(target_os = "windows")]
                let is_executable = path.extension().map_or(false, |ext| ext == "exe");

                if is_executable {
                    return Ok(path);
                }
            }
        }
        return Err(format!("Binary not found in archive at {:?}", binary_path));
    }

    Ok(binary_path)
}

/// Extract codex binary from a tar.gz archive (macOS, Linux)
fn extract_tar_gz(
    archive_content: &[u8],
    temp_dir: &std::path::Path,
    _version: &str,
    platform: &str,
) -> Result<std::path::PathBuf, String> {
    use flate2::read::GzDecoder;
    use std::io::Cursor;
    use tar::Archive;

    let cursor = Cursor::new(archive_content);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);

    archive
        .unpack(temp_dir)
        .map_err(|e| format!("Failed to extract tar.gz archive: {e}"))?;

    // The binary is named codex-{platform} (e.g., codex-x86_64-unknown-linux-musl)
    let binary_name = format!("codex-{}", platform);
    let binary_path = temp_dir.join(&binary_name);

    if !binary_path.exists() {
        return Err(format!("Binary not found in archive at {:?}", binary_path));
    }

    Ok(binary_path)
}

/// Result of checking Codex CLI authentication status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexAuthStatus {
    /// Whether the CLI is authenticated
    pub authenticated: bool,
    /// Error message if authentication check failed
    pub error: Option<String>,
}

/// Check if Codex CLI is authenticated by running a simple command
#[tauri::command]
pub async fn check_codex_cli_auth(app: AppHandle) -> Result<CodexAuthStatus, String> {
    log::trace!("Checking Codex CLI authentication status");

    let binary_path = get_codex_cli_binary_path(&app)?;

    if !binary_path.exists() {
        return Ok(CodexAuthStatus {
            authenticated: false,
            error: Some("Codex CLI not installed".to_string()),
        });
    }

    // Run codex --help to check if binary is functional
    // This doesn't require authentication but verifies the CLI works
    log::trace!("Running auth check: {:?} --help", binary_path);

    let output = silent_command(&binary_path)
        .arg("--help")
        .output()
        .map_err(|e| format!("Failed to execute Codex CLI: {e}"))?;

    // Check if we can access the OPENAI_API_KEY environment variable
    // For authenticated usage, users need to either:
    // 1. Run `codex login` (interactive)
    // 2. Set OPENAI_API_KEY environment variable
    // 3. Use `codex login --api-key <key>`

    let has_api_key = std::env::var("OPENAI_API_KEY").is_ok();

    if output.status.success() {
        log::trace!("Codex CLI is functional");
        Ok(CodexAuthStatus {
            // We consider it "auth ready" if the CLI works
            // Actual auth will be verified during first use
            authenticated: has_api_key,
            error: if has_api_key {
                None
            } else {
                Some("API key not set. Run 'codex login' or set OPENAI_API_KEY environment variable.".to_string())
            },
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        log::warn!("Codex CLI auth check failed: {}", stderr);
        Ok(CodexAuthStatus {
            authenticated: false,
            error: Some(stderr),
        })
    }
}

/// Helper function to emit installation progress events
fn emit_progress(app: &AppHandle, stage: &str, message: &str, percent: u8) {
    let progress = CodexInstallProgress {
        stage: stage.to_string(),
        message: message.to_string(),
        percent,
    };

    if let Err(e) = app.emit_all("codex-cli:install-progress", &progress) {
        log::warn!("Failed to emit install progress: {}", e);
    }
}
