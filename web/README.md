# PseudoLang Web IDE

Browser-based IDE using Astro, Monaco Editor, and xterm.js. Runs PseudoLang via WASI in a Web Worker.

## Development

```bash
just src dev-web
```

## Build

```bash
just src build-web
```

Requires `fpli.wasm` and `pseudolang.tmLanguage.json` in `public/` (copied automatically by just recipes).
