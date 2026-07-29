//! Terminal clipboard integration using the OSC 52 protocol.

use base64::Engine;

/// Builds an OSC 52 escape sequence that asks the terminal to set the system
/// clipboard. Works locally and over SSH (unlike shelling out to
/// pbcopy/xclip), as long as the terminal emulator supports/allows it (most
/// modern ones do: iTerm2, Kitty, WezTerm, foot, Windows Terminal, etc).
///
/// # Arguments
///
/// * `text` - The plain-text content to place on the system clipboard.
///
/// # Returns
///
/// The complete escape sequence (`ESC ] 5 2 ; c ; <base64> BEL`), ready to be
/// written directly to the terminal.
pub fn build_osc52_copy_sequence(text: &str) -> String {
    let base64 = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    format!("\x1b]52;c;{base64}\x07")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_base64_payload_in_osc52() {
        let seq = build_osc52_copy_sequence("hi");
        assert_eq!(seq, "\x1b]52;c;aGk=\x07");
    }
}
