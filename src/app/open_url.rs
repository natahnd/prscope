//! Cross-platform opening of pull request URLs in the user's browser.

use std::io;
use std::path::Path;
use std::process::Command;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

const WSL_POWERSHELL_PATHS: &[&str] = &[
    "/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe",
    "/mnt/c/Program Files/PowerShell/7/pwsh.exe",
];

/// Opens a URL in the user's default browser.
///
/// On WSL, invokes Windows PowerShell by its mounted path with an encoded
/// command. This avoids depending on `powershell.exe` being present on the
/// Linux `PATH` or on environment variables crossing the WSL boundary.
///
/// # Arguments
///
/// * `url` - The URL to open (typically a pull request's web URL).
///
/// # Returns
///
/// `Ok(())` once a browser-opening command has been successfully launched.
///
/// # Errors
///
/// On WSL, a failed PowerShell attempt is not returned directly: this
/// function falls through to `open::that` instead. Returns `Err` only if
/// that final `open::that(url)` call fails to locate or launch a handler for
/// the URL.
pub fn open_url(url: &str) -> io::Result<()> {
    if is_wsl() {
        for ps in WSL_POWERSHELL_PATHS {
            if Path::new(ps).exists() {
                if open_via_powershell(ps, url).is_ok() {
                    return Ok(());
                }
            }
        }
    }
    open::that(url)
}

fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map(|v| {
            let lower = v.to_ascii_lowercase();
            lower.contains("microsoft") || lower.contains("wsl")
        })
        .unwrap_or(false)
}

fn open_via_powershell(powershell: &str, url: &str) -> io::Result<()> {
    let encoded = encode_powershell_start_command(url);
    let status = Command::new(powershell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encoded,
        ])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "PowerShell exited with {status} opening {url}"
        )))
    }
}

/// Encodes `Start "<url>"` as UTF-16LE Base64 for PowerShell's
/// `-EncodedCommand` argument.
///
/// # Arguments
///
/// * `url` - The URL to embed in the `Start` command.
///
/// # Returns
///
/// The Base64-encoded, UTF-16LE representation of `Start "<url>"`, ready to
/// pass as `-EncodedCommand`.
fn encode_powershell_start_command(url: &str) -> String {
    let command = format!("Start \"{url}\"");
    let bytes: Vec<u8> = command.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_start_command_as_utf16le_base64() {
        let encoded = encode_powershell_start_command("https://example.com");
        let decoded: String = STANDARD
            .decode(encoded)
            .unwrap()
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .filter_map(|u| char::from_u32(u as u32))
            .collect();
        assert_eq!(decoded, "Start \"https://example.com\"");
    }

    #[test]
    fn wsl_detection_does_not_panic() {
        let _ = is_wsl();
    }
}
