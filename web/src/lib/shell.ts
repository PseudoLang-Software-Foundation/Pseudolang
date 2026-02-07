import type { VirtualFS } from "./filesystem";

export interface ShellContext {
  fs: VirtualFS;
  write: (text: string) => void;
  writeln: (text: string) => void;
  clear: () => void;
  runPseudolang: (filename: string) => Promise<void>;
  getEditorContent: () => string;
  version: string;
}

type CommandFn = (args: string[], ctx: ShellContext) => void | Promise<void>;

async function fpliRun(
  args: string[],
  ctx: ShellContext,
  requireFile: boolean,
): Promise<void> {
  const file = args[0] || (requireFile ? "" : "main.psl");
  if (!file) {
    ctx.writeln("fpli run: missing file operand");
    ctx.writeln("Usage: fpli run <file.psl>");
    return;
  }
  ctx.fs.writeFile("main.psl", ctx.getEditorContent());
  if (!ctx.fs.exists(file)) {
    ctx.writeln(`fpli: ${file}: No such file`);
    return;
  }
  await ctx.runPseudolang(file);
}

const FPLI_HELP = `PseudoLang Usage:
    fpli [OPTIONS] COMMAND [ARGS]

COMMANDS:
    run <file.psl> [PROGRAM_ARGS...]    Execute a PseudoLang program

OPTIONS:
    -h, --help       Display this help message
    -V, --version    Display version information
    -d, --debug      Enable debug output during execution

Examples:
    fpli run program.psl
    fpli run main.psl`;

const commands: Record<string, CommandFn> = {
  fpli: async (args, ctx) => {
    if (args.length === 0 || args[0] === "-h" || args[0] === "--help") {
      ctx.writeln(FPLI_HELP);
      return;
    }
    if (args[0] === "-V" || args[0] === "--version") {
      ctx.writeln(`PseudoLang version ${ctx.version}`);
      return;
    }
    const subcommand = args[0];
    if (subcommand === "run") {
      await fpliRun(args.slice(1), ctx, true);
      return;
    }
    ctx.writeln(`fpli: unknown command '${subcommand}'`);
    ctx.writeln("Run 'fpli --help' for usage.");
  },

  run: async (args, ctx) => {
    await fpliRun(args, ctx, false);
  },

  ls: (args, ctx) => {
    const path = args[0];
    const entries = ctx.fs.ls(path);
    if (entries.length === 0) {
      ctx.writeln("(empty)");
      return;
    }
    for (const entry of entries) {
      const isDir = entry.endsWith("/");
      ctx.writeln(isDir ? `\x1b[34m${entry}\x1b[0m` : entry);
    }
  },

  cat: (args, ctx) => {
    if (args.length === 0) {
      ctx.writeln("cat: missing file operand");
      return;
    }
    const content = ctx.fs.readFile(args[0]);
    if (content === null) {
      ctx.writeln(`cat: ${args[0]}: No such file`);
      return;
    }
    ctx.write(content);
    if (!content.endsWith("\n")) ctx.writeln("");
  },

  echo: (args, ctx) => {
    ctx.writeln(args.join(" "));
  },

  pwd: (_args, ctx) => {
    ctx.writeln(ctx.fs.cwd);
  },

  cd: (args, ctx) => {
    if (args.length === 0) {
      ctx.fs.cd("/workspace");
      return;
    }
    const err = ctx.fs.cd(args[0]);
    if (err) ctx.writeln(err);
  },

  mkdir: (args, ctx) => {
    if (args.length === 0) {
      ctx.writeln("mkdir: missing operand");
      return;
    }
    const err = ctx.fs.mkdir(args[0]);
    if (err) ctx.writeln(err);
  },

  rm: (args, ctx) => {
    if (args.length === 0) {
      ctx.writeln("rm: missing operand");
      return;
    }
    const err = ctx.fs.rm(args[0]);
    if (err) ctx.writeln(err);
  },

  touch: (args, ctx) => {
    if (args.length === 0) {
      ctx.writeln("touch: missing operand");
      return;
    }
    if (!ctx.fs.exists(args[0])) {
      ctx.fs.writeFile(args[0], "");
    }
  },

  clear: (_args, ctx) => {
    ctx.clear();
  },

  version: (_args, ctx) => {
    ctx.writeln(`PseudoLang v${ctx.version}`);
  },

  help: (_args, ctx) => {
    ctx.writeln("Available commands:");
    ctx.writeln(
      "  fpli <cmd>    PseudoLang CLI (fpli run <file>, fpli --help)",
    );
    ctx.writeln("  run [file]    Shortcut for 'fpli run' (default: main.psl)");
    ctx.writeln("  ls [path]     List directory contents");
    ctx.writeln("  cat <file>    Print file contents");
    ctx.writeln("  echo <text>   Print text");
    ctx.writeln("  pwd           Print working directory");
    ctx.writeln("  cd <dir>      Change directory");
    ctx.writeln("  mkdir <dir>   Create directory");
    ctx.writeln("  rm <file>     Remove file");
    ctx.writeln("  touch <file>  Create empty file");
    ctx.writeln("  clear         Clear terminal");
    ctx.writeln("  version       Show PseudoLang version");
    ctx.writeln("  help          Show this help");
  },
};

export function parseCommand(
  line: string,
): { name: string; args: string[] } | null {
  const trimmed = line.trim();
  if (!trimmed) return null;
  const parts = trimmed.split(/\s+/);
  return { name: parts[0], args: parts.slice(1) };
}

export async function executeCommand(
  line: string,
  ctx: ShellContext,
): Promise<void> {
  const parsed = parseCommand(line);
  if (!parsed) return;

  const handler = commands[parsed.name];
  if (!handler) {
    ctx.writeln(`${parsed.name}: command not found. Type 'help' for commands.`);
    return;
  }

  await handler(parsed.args, ctx);
}
