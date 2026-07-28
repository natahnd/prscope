/**
 * Builds an OSC 52 escape sequence that asks the terminal to set the system
 * clipboard. Works locally and over SSH (unlike shelling out to pbcopy/xclip),
 * as long as the terminal emulator supports/allows it (most modern ones do:
 * iTerm2, Kitty, WezTerm, foot, Windows Terminal, etc).
 */
export function buildOsc52CopySequence(text: string): string {
  const base64 = Buffer.from(text, "utf8").toString("base64");
  return `\x1b]52;c;${base64}\x07`;
}
