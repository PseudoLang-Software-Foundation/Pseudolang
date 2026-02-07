import { shikiToMonaco } from "@shikijs/monaco";
import * as monaco from "monaco-editor/esm/vs/editor/editor.api";
import { createHighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import darkPlus from "shiki/themes/dark-plus.mjs";

let editorInstance: monaco.editor.IStandaloneCodeEditor | null = null;
let saveTimeout: ReturnType<typeof setTimeout> | null = null;

const STORAGE_KEY = "pseudolang-editor-content";

const DEFAULT_CODE = `// Welcome to PseudoLang!
// Press the Run button or type 'run' in the terminal.

PROCEDURE greet(name)
{
  DISPLAY("Hello, " + name + "!")
}

names <- ["Alice", "Bob", "Charlie"]

FOR EACH name IN names
{
  greet(name)
}

x <- RANDOM(1, 100)
DISPLAY("Random number: " + TOSTRING(x))
`;

export async function initEditor(container: HTMLElement): Promise<void> {
  // Load the tmLanguage grammar
  const grammarResponse = await fetch(
    `${import.meta.env.BASE_URL}pseudolang.tmLanguage.json`,
  );
  const grammar = await grammarResponse.json();

  // Create Shiki highlighter with ONLY PseudoLang (no bundled languages)
  const highlighter = await createHighlighterCore({
    themes: [darkPlus],
    langs: [
      {
        ...grammar,
        name: "pseudolang",
        scopeName: "source.pseudolang",
      },
    ],
    engine: createJavaScriptRegexEngine(),
  });

  // Register language with Monaco
  monaco.languages.register({
    id: "pseudolang",
    extensions: [".psl"],
    aliases: ["PseudoLang", "pseudolang"],
  });

  // Wire Shiki highlighting into Monaco
  shikiToMonaco(highlighter, monaco);

  editorInstance = monaco.editor.create(container, {
    value: localStorage.getItem(STORAGE_KEY) ?? DEFAULT_CODE,
    language: "pseudolang",
    theme: "dark-plus",
    fontSize: 14,
    fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
    minimap: { enabled: false },
    lineNumbers: "on",
    tabSize: 2,
    insertSpaces: true,
    wordWrap: "on",
    automaticLayout: false,
    scrollBeyondLastLine: false,
    padding: { top: 8 },
    renderLineHighlight: "line",
    cursorBlinking: "smooth",
    smoothScrolling: true,
    bracketPairColorization: { enabled: true },
  });

  editorInstance.onDidChangeModelContent(() => {
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      localStorage.setItem(STORAGE_KEY, editorInstance?.getValue() ?? "");
    }, 500);
  });
}

export function getEditorContent(): string {
  return editorInstance?.getValue() ?? "";
}

export function resizeEditor(): void {
  editorInstance?.layout();
}
