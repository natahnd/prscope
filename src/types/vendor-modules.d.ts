declare module "marked-terminal" {
  import type { MarkedExtension } from "marked";

  export interface MarkedTerminalOptions {
    width?: number;
    reflowText?: boolean;
    showSectionPrefix?: boolean;
    tab?: number;
    emoji?: boolean;
    unescape?: boolean;
    [key: string]: unknown;
  }

  export function markedTerminal(
    options?: MarkedTerminalOptions,
    highlightOptions?: Record<string, unknown>,
  ): MarkedExtension;

  const Renderer: unknown;
  export default Renderer;
}

declare module "supports-hyperlinks" {
  interface SupportsHyperlinks {
    stdout: boolean;
    stderr: boolean;
  }
  const supportsHyperlinks: SupportsHyperlinks;
  export default supportsHyperlinks;
}
