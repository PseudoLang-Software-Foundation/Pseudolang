import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { Readline } from "xterm-readline";
import type { ShellContext } from "./shell";
import { executeCommand } from "./shell";

let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let rl: Readline | null = null;
let shellCtx: ShellContext | null = null;
let commandResolve: ((line: string) => void) | null = null;
let lastPartialLine = "";

const PROMPT = "\x1b[32mpseudolang>\x1b[0m ";

// Cap on the retained tail of a newline-less output line. The tail is only ever
// replayed as the prompt for the next stdin read, so a couple of terminal lines
// is all that can matter — without a cap, a program that writes megabytes
// before its first newline grows this string without bound.
const MAX_PARTIAL_LINE = 4096;

export function initTerminal(
  container: HTMLElement,
  ctx: ShellContext,
): Terminal {
  shellCtx = ctx;

  terminal = new Terminal({
    theme: {
      background: "#1e1e1e",
      foreground: "#cccccc",
      cursor: "#aeafad",
      selectionBackground: "#264f78",
      black: "#1e1e1e",
      red: "#f44747",
      green: "#4ec9b0",
      yellow: "#dcdcaa",
      blue: "#569cd6",
      magenta: "#c586c0",
      cyan: "#9cdcfe",
      white: "#d4d4d4",
      brightBlack: "#808080",
      brightRed: "#f44747",
      brightGreen: "#4ec9b0",
      brightYellow: "#dcdcaa",
      brightBlue: "#569cd6",
      brightMagenta: "#c586c0",
      brightCyan: "#9cdcfe",
      brightWhite: "#ffffff",
    },
    fontSize: 14,
    fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
    cursorBlink: true,
    convertEol: true,
    allowProposedApi: true,
  });

  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);

  rl = new Readline();
  terminal.loadAddon(rl);

  terminal.open(container);
  fitAddon.fit();

  rl.println(`\x1b[34mPseudoLang v${ctx.version}\x1b[0m - Web IDE`);
  rl.println(
    "Type \x1b[33mhelp\x1b[0m for commands, \x1b[33mrun\x1b[0m to execute your code.",
  );
  rl.println("");

  promptLoop();

  return terminal;
}

async function promptLoop(): Promise<void> {
  if (!rl || !shellCtx) return;
  while (true) {
    const line = await new Promise<string>((resolve) => {
      commandResolve = resolve;
      rl?.read(PROMPT).then((input) => {
        if (commandResolve === resolve) {
          commandResolve = null;
          resolve(input);
        }
      });
    });
    try {
      await executeCommand(line, shellCtx);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      rl?.println(`\x1b[31mError: ${msg}\x1b[0m`);
    }
  }
}

export function injectCommand(cmd: string): void {
  if (!commandResolve) return;
  const resolve = commandResolve;
  commandResolve = null;
  // The pending `read()` has already drawn PROMPT (plus anything typed so far)
  // on the current row. Erase that row before echoing, or the injected command
  // renders as "pseudolang> pseudolang> run".
  rl?.write("\r\x1b[J");
  rl?.println(`${PROMPT}${cmd}`);
  resolve(cmd);
}

export function readStdinLine(): Promise<string> {
  if (!rl) return Promise.resolve("");
  const prompt = lastPartialLine;
  lastPartialLine = "";
  return rl.read(prompt);
}

export function writeToTerminal(text: string): void {
  const lastNewline = text.lastIndexOf("\n");
  if (lastNewline >= 0) {
    lastPartialLine = text.slice(lastNewline + 1);
  } else {
    lastPartialLine += text;
  }
  if (lastPartialLine.length > MAX_PARTIAL_LINE) {
    lastPartialLine = lastPartialLine.slice(-MAX_PARTIAL_LINE);
  }
  rl?.write(text);
}

export function writelnToTerminal(text: string): void {
  lastPartialLine = "";
  rl?.println(text);
}

export function clearTerminal(): void {
  terminal?.write("\x1b[2J\x1b[H");
  terminal?.clear();
}

export function resizeTerminal(): void {
  fitAddon?.fit();
}
