const encoder = new TextEncoder();
const decoder = new TextDecoder();

interface FsEntry {
  type: "file" | "dir";
  content?: Uint8Array;
}

export class VirtualFS {
  private entries = new Map<string, FsEntry>();
  private _cwd = "/workspace";

  constructor() {
    this.mkdir("/workspace");
  }

  get cwd(): string {
    return this._cwd;
  }

  resolve(path: string): string {
    if (path.startsWith("/")) return normalize(path);
    return normalize(`${this._cwd}/${path}`);
  }

  cd(path: string): string | null {
    const resolved = this.resolve(path);
    const entry = this.entries.get(resolved);
    if (entry?.type !== "dir") return `cd: ${path}: No such directory`;
    this._cwd = resolved;
    return null;
  }

  mkdir(path: string): string | null {
    const resolved = this.resolve(path);
    if (this.entries.has(resolved)) return `mkdir: ${path}: Already exists`;
    // Ensure parent exists
    const parent = parentPath(resolved);
    if (parent && !this.entries.has(parent)) {
      this.mkdir(parent);
    }
    this.entries.set(resolved, { type: "dir" });
    return null;
  }

  writeFile(path: string, content: string | Uint8Array): void {
    const resolved = this.resolve(path);
    const parent = parentPath(resolved);
    if (parent && !this.entries.has(parent)) this.mkdir(parent);
    const data =
      typeof content === "string" ? encoder.encode(content) : content;
    this.entries.set(resolved, { type: "file", content: data });
  }

  readFile(path: string): string | null {
    const resolved = this.resolve(path);
    const entry = this.entries.get(resolved);
    if (entry?.type !== "file") return null;
    return decoder.decode(entry.content);
  }

  readFileBytes(path: string): Uint8Array | null {
    const resolved = this.resolve(path);
    const entry = this.entries.get(resolved);
    if (entry?.type !== "file" || !entry.content) return null;
    return entry.content;
  }

  exists(path: string): boolean {
    return this.entries.has(this.resolve(path));
  }

  isDir(path: string): boolean {
    const entry = this.entries.get(this.resolve(path));
    return entry?.type === "dir";
  }

  rm(path: string): string | null {
    const resolved = this.resolve(path);
    const entry = this.entries.get(resolved);
    if (!entry) return `rm: ${path}: No such file`;
    if (entry.type === "dir") {
      // Check if dir has children
      for (const key of this.entries.keys()) {
        if (key !== resolved && key.startsWith(`${resolved}/`))
          return `rm: ${path}: Is a directory (use rm -r)`;
      }
    }
    this.entries.delete(resolved);
    return null;
  }

  ls(path?: string): string[] {
    const resolved = path ? this.resolve(path) : this._cwd;
    const prefix = resolved === "/" ? "/" : `${resolved}/`;
    const results: string[] = [];

    for (const [key, entry] of this.entries) {
      if (key === resolved) continue;
      if (!key.startsWith(prefix)) continue;
      const relative = key.slice(prefix.length);
      if (relative.includes("/")) continue; // Skip nested entries
      const suffix = entry.type === "dir" ? "/" : "";
      results.push(relative + suffix);
    }

    return results.sort();
  }

  // Build the directory tree for WASI preopened dirs
  toWasiTree(): Map<string, { type: "file" | "dir"; content?: Uint8Array }> {
    const tree = new Map<
      string,
      { type: "file" | "dir"; content?: Uint8Array }
    >();
    const base = "/workspace";
    for (const [key, entry] of this.entries) {
      if (!key.startsWith(base)) continue;
      const relative = key === base ? "." : key.slice(base.length + 1);
      if (relative === "") continue;
      tree.set(relative, {
        type: entry.type,
        content: entry.content,
      });
    }
    return tree;
  }
}

function normalize(path: string): string {
  const parts = path.split("/").filter(Boolean);
  const resolved: string[] = [];
  for (const part of parts) {
    if (part === ".") continue;
    if (part === "..") {
      resolved.pop();
    } else {
      resolved.push(part);
    }
  }
  return `/${resolved.join("/")}`;
}

function parentPath(path: string): string | null {
  const idx = path.lastIndexOf("/");
  if (idx <= 0) return null;
  return path.slice(0, idx);
}
