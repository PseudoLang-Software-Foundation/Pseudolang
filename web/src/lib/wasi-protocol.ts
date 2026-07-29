// Layout of the SharedArrayBuffer shared between the main thread (`wasi.ts`)
// and the WASI worker (`wasi-worker.ts`). Both sides must agree exactly, so the
// numbers live here rather than being written out at each use site.

/** Two Int32 slots: [0] = state signal, [1] = stdin payload length. */
export const SIGNAL_BYTES = 8;

/** Size of the stdin payload window that follows the signal slots. */
export const STDIN_BUFFER_BYTES = 4096;

/** Total SharedArrayBuffer size. */
export const SAB_BYTES = SIGNAL_BYTES + STDIN_BUFFER_BYTES;
