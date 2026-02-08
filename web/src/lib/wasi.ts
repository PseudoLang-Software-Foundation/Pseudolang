import type { VirtualFS } from "./filesystem";

const HAS_SAB = typeof SharedArrayBuffer !== "undefined";
const SAB_SIZE = 8 + 4096;

let worker: Worker | null = null;
let sab: SharedArrayBuffer | null = null;
let signalArray: Int32Array | null = null;
let dataArray: Uint8Array | null = null;

function ensureWorker(): Worker {
  if (worker) return worker;
  sab = new SharedArrayBuffer(SAB_SIZE);
  signalArray = new Int32Array(sab, 0, 2);
  dataArray = new Uint8Array(sab, 8, 4096);
  worker = new Worker(new URL("./wasi-worker.ts", import.meta.url), {
    type: "module",
  });
  worker.postMessage({ type: "init", sab });
  return worker;
}

export function sendStdinInput(input: string): void {
  if (!signalArray || !dataArray) return;
  const encoded = new TextEncoder().encode(`${input}\n`);
  const len = Math.min(encoded.length, 4096);
  dataArray.set(encoded.slice(0, len));
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
    w.onmessage = (e: MessageEvent) => {
      switch (e.data.type) {
        case "stdout":
          onStdout(e.data.text);
          break;
        case "stderr":
          onStderr(e.data.text);
          break;
        case "stdin-request":
          onStdinRequest();
          break;
        case "exit":
          resolve(e.data.code);
          break;
        case "error":
          onStderr(e.data.message);
          resolve(1);
          break;
      }
    };

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
