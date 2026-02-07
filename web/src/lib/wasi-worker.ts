import {
  ConsoleStdout,
  type Directory,
  Fd,
  File,
  PreopenDirectory,
  WASI,
  wasi as wasiDefs,
} from "@bjorn3/browser_wasi_shim";

let signalArray: Int32Array;
let dataArray: Uint8Array;
let wasmModule: WebAssembly.Module | null = null;

class BlockingStdin extends Fd {
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
      Atomics.store(signalArray, 0, 1);
      self.postMessage({ type: "stdin-request" });
      Atomics.wait(signalArray, 0, 1);

      const signal = Atomics.load(signalArray, 0);
      if (signal === 3) {
        return { ret: 0, data: new Uint8Array(0) };
      }

      const len = Atomics.load(signalArray, 1);
      this.buffer = new Uint8Array(len);
      this.buffer.set(
        new Uint8Array(dataArray.buffer, dataArray.byteOffset, len),
      );
      this.position = 0;
      Atomics.store(signalArray, 0, 0);
    }

    const slice = this.buffer.slice(this.position, this.position + size);
    this.position += slice.length;
    return { ret: 0, data: slice };
  }
}

function unbufferedStdout(msgType: string): ConsoleStdout {
  const decoder = new TextDecoder("utf-8", { fatal: false });
  return new ConsoleStdout((data: Uint8Array) => {
    const text = decoder.decode(data, { stream: true });
    if (text) {
      self.postMessage({ type: msgType, text });
    }
  });
}

self.onmessage = async (e: MessageEvent) => {
  const msg = e.data;

  if (msg.type === "init") {
    signalArray = new Int32Array(msg.sab, 0, 2);
    dataArray = new Uint8Array(msg.sab, 8, 4096);
    self.postMessage({ type: "ready" });
    return;
  }

  if (msg.type === "run") {
    try {
      if (!wasmModule) {
        const response = await fetch(`${import.meta.env.BASE_URL}fpli.wasm`);
        wasmModule = await WebAssembly.compileStreaming(response);
      }

      const entries = new Map<string, File | Directory>();
      for (const [path, content] of Object.entries(
        msg.files as Record<string, Uint8Array>,
      )) {
        entries.set(path, new File(content));
      }

      const fds = [
        new BlockingStdin(),
        unbufferedStdout("stdout"),
        unbufferedStdout("stderr"),
        new PreopenDirectory(".", entries),
      ];

      const wasi = new WASI(msg.args as string[], [], fds);
      const wasiImport = {
        ...wasi.wasiImport,
        poll_oneoff(
          inPtr: number,
          outPtr: number,
          nsubscriptions: number,
          neventsPtr: number,
        ): number {
          const wasmMemory = (
            wasi.inst as unknown as {
              exports: { memory: WebAssembly.Memory };
            }
          ).exports.memory.buffer;
          const view = new DataView(wasmMemory);
          const view8 = new Uint8Array(wasmMemory);
          let maxNs = BigInt(0);
          for (let i = 0; i < nsubscriptions; i++) {
            const subPtr = inPtr + i * 48;
            const tag = view.getUint8(subPtr + 8);
            if (tag === 0) {
              const timeout = view.getBigUint64(subPtr + 24, true);
              if (timeout > maxNs) maxNs = timeout;
            }
            const evtPtr = outPtr + i * 32;
            view8.copyWithin(evtPtr, subPtr, subPtr + 8);
            view.setUint16(evtPtr + 8, 0, true);
            view.setUint8(evtPtr + 10, tag);
          }
          if (maxNs > 0) {
            const ms = Number(maxNs / BigInt(1_000_000));
            const sleepBuf = new SharedArrayBuffer(4);
            const sleepArr = new Int32Array(sleepBuf);
            Atomics.wait(sleepArr, 0, 0, ms);
          }
          view.setUint32(neventsPtr, nsubscriptions, true);
          return 0;
        },
      };
      const instance = await WebAssembly.instantiate(wasmModule, {
        wasi_snapshot_preview1: wasiImport,
      });

      const exitCode = wasi.start(
        instance as unknown as {
          exports: { memory: WebAssembly.Memory; _start: () => void };
        },
      );
      self.postMessage({ type: "exit", code: exitCode });
    } catch (err) {
      if (err instanceof Error && err.message.includes("exit")) {
        self.postMessage({ type: "exit", code: 0 });
      } else {
        self.postMessage({
          type: "error",
          message: err instanceof Error ? err.message : String(err),
        });
      }
    }
  }
};
