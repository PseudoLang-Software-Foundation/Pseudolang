import type { VirtualFS } from "./filesystem";
import { SAB_BYTES, SIGNAL_BYTES, STDIN_BUFFER_BYTES } from "./wasi-protocol";

const HAS_SAB = typeof SharedArrayBuffer !== "undefined";

let worker: Worker | null = null;
let sab: SharedArrayBuffer | null = null;
let signalArray: Int32Array | null = null;
let dataArray: Uint8Array | null = null;

interface ActiveRun {
  onStdout: (text: string) => void;
  onStderr: (text: string) => void;
  onStdinRequest: () => void;
  settle: (code: number) => void;
}

let activeRun: ActiveRun | null = null;

function ensureWorker(): Worker {
  if (worker) return worker;
  sab = new SharedArrayBuffer(SAB_BYTES);
  signalArray = new Int32Array(sab, 0, 2);
  dataArray = new Uint8Array(sab, SIGNAL_BYTES, STDIN_BUFFER_BYTES);
  const w = new Worker(new URL("./wasi-worker.ts", import.meta.url), {
    type: "module",
  });
  // Installed once for the worker's lifetime and dispatched through
  // `activeRun`. Reassigning `onmessage` per run dropped the previous run's
  // handler on the floor: a superseded run's promise never settled (leaving its
  // awaiting caller and closure pinned forever) and its "exit" message was
  // delivered to whichever run happened to be current.
  w.onmessage = (e: MessageEvent) => {
    const run = activeRun;
    if (!run) return;
    switch (e.data.type) {
      case "stdout":
        run.onStdout(e.data.text);
        break;
      case "stderr":
        run.onStderr(e.data.text);
        break;
      case "stdin-request":
        run.onStdinRequest();
        break;
      case "exit":
        activeRun = null;
        run.settle(e.data.code);
        break;
      case "error":
        activeRun = null;
        run.onStderr(e.data.message);
        run.settle(1);
        break;
    }
  };
  w.postMessage({ type: "init", sab });
  worker = w;
  return w;
}

export function sendStdinInput(input: string): void {
  if (!signalArray || !dataArray) return;
  const encoded = new TextEncoder().encode(`${input}\n`);
  let len = encoded.length;
  if (len > STDIN_BUFFER_BYTES) {
    // The shared window is fixed size, so an over-long line has to be cut.
    // Cut back to a UTF-8 code-point boundary so the worker never decodes a
    // split multi-byte sequence, and reserve the final byte for the newline:
    // without it the guest blocks forever waiting for end-of-line.
    len = STDIN_BUFFER_BYTES - 1;
    while (len > 0 && (encoded[len] & 0b1100_0000) === 0b1000_0000) len--;
    encoded[len] = 0x0a;
    len++;
  }
  dataArray.set(encoded.subarray(0, len));
  Atomics.store(signalArray, 1, len);
  Atomics.store(signalArray, 0, 2);
  Atomics.notify(signalArray, 0);
}

export function sendStdinEof(): void {
  if (!signalArray) return;
  Atomics.store(signalArray, 0, 3);
  Atomics.notify(signalArray, 0);
}

function serializeFs(fs: VirtualFS): Record<string, Uint8Array> {
  const files: Record<string, Uint8Array> = {};
  const tree = fs.toWasiTree();
  for (const [path, entry] of tree) {
    if (entry.type === "file" && entry.content) {
      files[path] = entry.content;
    }
  }
  return files;
}

export async function runWasi(
  args: string[],
  fs: VirtualFS,
  onStdout: (text: string) => void,
  onStderr: (text: string) => void,
  onStdinRequest: () => void,
): Promise<number> {
  const files = serializeFs(fs);

  if (!HAS_SAB) {
    return runWasiFallback(args, files, onStdout, onStderr);
  }

  const w = ensureWorker();

  return new Promise((resolve) => {
    activeRun = { onStdout, onStderr, onStdinRequest, settle: resolve };

    if (signalArray) Atomics.store(signalArray, 0, 0);

    w.postMessage({
      type: "run",
      args,
      files,
    });
  });
}

async function runWasiFallback(
  args: string[],
  files: Record<string, Uint8Array>,
  onStdout: (text: string) => void,
  onStderr: (text: string) => void,
): Promise<number> {
  const {
    ConsoleStdout,
    Fd,
    File,
    PreopenDirectory,
    WASI,
    wasi: wasiDefs,
  } = await import("@bjorn3/browser_wasi_shim");

  class PromptStdin extends Fd {
    private buffer: Uint8Array = new Uint8Array(0);
    private position = 0;

    fd_fdstat_get() {
      return {
        ret: 0,
        fdstat: new wasiDefs.Fdstat(wasiDefs.FILETYPE_CHARACTER_DEVICE, 0),
      };
    }

    fd_read(size: number) {
      if (this.position >= this.buffer.length) {
        const input = window.prompt("Program is requesting input:");
        if (input === null) return { ret: 0, data: new Uint8Array(0) };
        this.buffer = new TextEncoder().encode(`${input}\n`);
        this.position = 0;
      }
      const slice = this.buffer.slice(this.position, this.position + size);
      this.position += slice.length;
      return { ret: 0, data: slice };
    }
  }

  const response = await fetch(`${import.meta.env.BASE_URL}fpli.wasm`);
  const module = await WebAssembly.compileStreaming(response);

  const entries = new Map();
  for (const [path, content] of Object.entries(files)) {
    entries.set(path, new File(content));
  }

  const fds = [
    new PromptStdin(),
    ConsoleStdout.lineBuffered((line: string) => onStdout(`${line}\n`)),
    ConsoleStdout.lineBuffered((line: string) => onStderr(`${line}\n`)),
    new PreopenDirectory(".", entries),
  ];

  const wasi = new WASI(args, [], fds);
  const instance = await WebAssembly.instantiate(module, {
    wasi_snapshot_preview1: wasi.wasiImport,
  });

  try {
    return wasi.start(
      instance as unknown as {
        exports: { memory: WebAssembly.Memory; _start: () => void };
      },
    );
  } catch (err) {
    if (err instanceof Error && err.message.includes("exit")) return 0;
    throw err;
  }
}
