import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import type { ShellContext } from "./shell";
import { executeCommand } from "./shell";
import { sendStdinEof, sendStdinInput } from "./wasi";

interface QueueEntry {
  line: string;
  needsEcho: boolean;
}

let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let inputBuffer = "";
let shellCtx: ShellContext | null = null;
let isRunning = false;
let waitingForStdin = false;

const commandHistory: string[] = [];
let historyIndex = -1;
let savedInput = "";

const lineQueue: QueueEntry[] = [];
let processingQueue = false;

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

  terminal.open(container);
  fitAddon.fit();

  terminal.write("\x1b[?2004h");
  terminal.writeln(`\x1b[34mPseudoLang v${ctx.version}\x1b[0m - Web IDE`);
  terminal.writeln(
    "Type \x1b[33mhelp\x1b[0m for commands, \x1b[33mrun\x1b[0m to execute your code.",
  );
  terminal.writeln("");
  writePrompt();

  terminal.onData((data) => {
    if (waitingForStdin) {
      handleStdinInput(data);
      return;
    }
    if (isRunning) return;
    handleInput(data);
  });

  return terminal;
}

export function writePrompt(): void {
  terminal?.write(`\x1b[32mpseudolang>\x1b[0m `);
}

export function enableStdinInput(): void {
  waitingForStdin = true;
}

function handleStdinInput(data: string): void {
  if (!terminal) return;

  for (const char of data) {
    if (char === "\r" || char === "\n") {
      terminal.writeln("");
      const line = inputBuffer;
      inputBuffer = "";
      waitingForStdin = false;
      sendStdinInput(line);
      return;
    }
    if (char === "\x7f") {
      if (inputBuffer.length > 0) {
        inputBuffer = inputBuffer.slice(0, -1);
        terminal.write("\b \b");
      }
      continue;
    }
    if (char === "\x03") {
      inputBuffer = "";
      waitingForStdin = false;
      terminal.writeln("^C");
      sendStdinEof();
      return;
    }
    if (char >= " ") {
      inputBuffer += char;
      terminal.write(char);
    }
  }
}

function clearInputLine(): void {
  if (!terminal) return;
  const len = inputBuffer.length;
  if (len > 0) {
    terminal.write(`\x1b[${len}D\x1b[0K`);
  }
}

function navigateHistory(direction: number): void {
  if (!terminal) return;

  if (direction < 0) {
    if (commandHistory.length === 0) return;
    if (historyIndex === -1) {
      savedInput = inputBuffer;
      historyIndex = commandHistory.length - 1;
    } else if (historyIndex > 0) {
      historyIndex--;
    } else {
      return;
    }
  } else {
    if (historyIndex === -1) return;
    if (historyIndex < commandHistory.length - 1) {
      historyIndex++;
    } else {
      historyIndex = -1;
    }
  }

  clearInputLine();

  if (historyIndex === -1) {
    inputBuffer = savedInput;
  } else {
    inputBuffer = commandHistory[historyIndex];
  }
  terminal.write(inputBuffer);
}

function handlePaste(data: string): void {
  if (!terminal) return;
  const cleaned = data.replaceAll("\x1b[200~", "").replaceAll("\x1b[201~", "");
  const lines = cleaned.split(/\r\n|\r|\n/);

  for (const ch of lines[0]) {
    if (ch >= " " && ch !== "\x7f") {
      inputBuffer += ch;
      terminal.write(ch);
    }
  }

  if (lines.length <= 1) return;

  terminal.writeln("");
  const firstLine = inputBuffer;
  inputBuffer = "";
  historyIndex = -1;
  savedInput = "";
  if (firstLine.trim()) commandHistory.push(firstLine);
  lineQueue.push({ line: firstLine, needsEcho: false });

  for (let i = 1; i < lines.length; i++) {
    if (i === lines.length - 1 && lines[i] === "") continue;
    lineQueue.push({ line: lines[i], needsEcho: true });
  }

  drainQueue();
}

function handleInput(data: string): void {
  if (!terminal || !shellCtx) return;

  if (data.includes("\x1b[200~")) {
    handlePaste(data);
    return;
  }

  if (data.length > 1 && /[\r\n]/.test(data)) {
    handlePaste(data);
    return;
  }

  if (data === "\x1b[A") {
    navigateHistory(-1);
    return;
  }
  if (data === "\x1b[B") {
    navigateHistory(1);
    return;
  }

  let i = 0;
  while (i < data.length) {
    const char = data[i];

    if (char === "\r" || char === "\n") {
      if (char === "\r" && i + 1 < data.length && data[i + 1] === "\n") {
        i++;
      }
      terminal.writeln("");
      const line = inputBuffer;
      inputBuffer = "";
      historyIndex = -1;
      savedInput = "";
      if (line.trim()) {
        commandHistory.push(line);
      }
      lineQueue.push({ line, needsEcho: false });
      i++;
      continue;
    }

    if (char === "\x7f") {
      if (inputBuffer.length > 0) {
        inputBuffer = inputBuffer.slice(0, -1);
        terminal.write("\b \b");
      }
      i++;
      continue;
    }

    if (char === "\x03") {
      inputBuffer = "";
      historyIndex = -1;
      savedInput = "";
      lineQueue.length = 0;
      terminal.writeln("^C");
      writePrompt();
      i++;
      continue;
    }

    if (char === "\x1b") {
      i++;
      if (i < data.length && data[i] === "[") {
        i++;
        while (i < data.length && data[i] >= "0" && data[i] <= "?") i++;
        if (i < data.length) i++;
      }
      continue;
    }

    if (char >= " ") {
      inputBuffer += char;
      terminal.write(char);
    }
    i++;
  }

  drainQueue();
}

async function drainQueue(): Promise<void> {
  if (processingQueue || !shellCtx || lineQueue.length === 0) return;
  processingQueue = true;
  while (lineQueue.length > 0) {
    const entry = lineQueue.shift();
    if (!entry) break;

    if (entry.needsEcho && terminal) {
      writePrompt();
      terminal.write(entry.line);
      terminal.writeln("");
      if (entry.line.trim()) commandHistory.push(entry.line);
    }

    isRunning = true;
    try {
      await executeCommand(entry.line, shellCtx);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      terminal?.writeln(`\x1b[31mError: ${msg}\x1b[0m`);
    }
    isRunning = false;
  }
  writePrompt();
  processingQueue = false;
}

export function writeToTerminal(text: string): void {
  terminal?.write(text);
}

export function writelnToTerminal(text: string): void {
  terminal?.writeln(text);
}

export function clearTerminal(): void {
  terminal?.clear();
}

export function resizeTerminal(): void {
  fitAddon?.fit();
}
