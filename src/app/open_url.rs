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
/// On WSL, mirrors the TypeScript app's `open` + `wsl-utils` behavior: invoke
/// Windows PowerShell via its full path with a Base64-encoded `Start "url"`
/// command. The Rust `open` crate's WSL path relies on `powershell.exe` being on
/// PATH and passes the target via an env var that WSL interop does not forward.
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

/// Encodes `Start "<url>"` as UTF-16LE Base64 for PowerShell's `-EncodedCommand`,
/// matching the `open` npm package on WSL.
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
