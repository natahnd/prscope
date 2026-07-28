import { useStdin, useStdout } from "ink";
import { useEffect, useRef } from "react";

export type MouseButton = "left" | "middle" | "right" | "wheel-up" | "wheel-down" | "unknown";
export type MouseAction = "down" | "up" | "move";

export interface MouseInputEvent {
  /** 0-indexed terminal column. */
  x: number;
  /** 0-indexed terminal row. */
  y: number;
  button: MouseButton;
  action: MouseAction;
  shift: boolean;
  alt: boolean;
  ctrl: boolean;
}

// SGR extended mouse reporting: `\x1b[<{code};{col};{row}{M|m}`, terminated
// by 'M' on press/move and 'm' on release. Matches globally since a single
// stdin chunk can contain several events (e.g. while dragging).
const SGR_MOUSE_SEQUENCE = /\x1b\[<(\d+);(\d+);(\d+)([Mm])/g;

function decodeMouseCode(code: number): { button: MouseButton; isDrag: boolean } {
  // Wheel events use the 64 bit instead of the low button bits, and never
  // carry the drag/motion flag.
  if ((code & 64) !== 0) {
    return { button: code % 2 === 0 ? "wheel-up" : "wheel-down", isDrag: false };
  }
  const isDrag = (code & 32) !== 0;
  const buttonBits = code & 3;
  const button: MouseButton =
    buttonBits === 0 ? "left" : buttonBits === 1 ? "middle" : buttonBits === 2 ? "right" : "unknown";
  return { button, isDrag };
}

/**
 * Enables SGR mouse reporting (click/release + drag-while-pressed) for the
 * lifetime of the calling component and forwards decoded events to
 * `onEvent`. Ink has no built-in mouse support, so this talks to the
 * terminal directly via raw ANSI escapes and parses the raw stdin bytes.
 */
export function useMouse(onEvent: (event: MouseInputEvent) => void, enabled = true): void {
  const { stdin, isRawModeSupported } = useStdin();
  const { stdout } = useStdout();
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  useEffect(() => {
    if (!enabled || !isRawModeSupported || !stdin.isTTY) return;

    // 1000 = report button press/release, 1002 = also report motion while a
    // button is held (needed for divider dragging, without the flood of
    // events that "any motion" mode (1003) would produce), 1006 = SGR
    // extended coordinates (works past column/row 223).
    stdout.write("\x1b[?1000h\x1b[?1002h\x1b[?1006h");

    const handleData = (chunk: Buffer | string) => {
      const text = typeof chunk === "string" ? chunk : chunk.toString("utf8");
      if (!text.includes("\x1b[<")) return;
      for (const match of text.matchAll(SGR_MOUSE_SEQUENCE)) {
        const code = Number(match[1]);
        const x = Number(match[2]) - 1;
        const y = Number(match[3]) - 1;
        const isRelease = match[4] === "m";
        const { button, isDrag } = decodeMouseCode(code);
        onEventRef.current({
          x,
          y,
          button,
          action: isDrag ? "move" : isRelease ? "up" : "down",
          shift: (code & 4) !== 0,
          alt: (code & 8) !== 0,
          ctrl: (code & 16) !== 0,
        });
      }
    };

    stdin.on("data", handleData);
    return () => {
      stdin.off("data", handleData);
      stdout.write("\x1b[?1000l\x1b[?1002l\x1b[?1006l");
    };
  }, [enabled, isRawModeSupported, stdin, stdout]);
}
