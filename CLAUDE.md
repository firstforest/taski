# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VS Code extension ("daily-task-logger") that aggregates tasks from markdown files across the workspace and displays them organized by date with clickable links back to source files. Written in TypeScript, bundled with esbuild, outputs to `dist/extension.js` as CommonJS.

## Commands

- `npm run compile` — type-check + lint + esbuild (dev mode, with sourcemaps)
- `npm run watch` — parallel watch for esbuild and tsc
- `npm run package` — type-check + lint + minified production build
- `npm run check-types` — TypeScript type-check only (`tsc --noEmit`)
- `npm run lint` — ESLint on `src/`
- `npm run test` — run VS Code extension tests (uses `@vscode/test-cli`)

## Architecture

Single-command extension with one main source file:

- **`src/extension.ts`** — registers the `daily-task-logger.showToday` command and a `TextDocumentContentProvider` for the `daily-tasks://` URI scheme. The `TodaysTaskProvider` class scans all `.md` files in the workspace, parses task checkboxes (`- [ ]` / `- [x]`) and their date-prefixed log entries (`- YYYY-MM-DD: text`), filters to the target date, and renders a virtual markdown document with per-file sections and line-linked task entries.
- **`src/test/extension.test.ts`** — Mocha test suite (currently a placeholder).

The extension has no runtime dependencies beyond the VS Code API. The `vscode` module is marked external in esbuild since VS Code provides it at runtime.

## Task Markdown Format

The extension parses this structure in `.md` files:

```markdown
- [x] Task name
    - 2026-02-01: Log entry for this date
- [ ] Another task
    - 2026-02-01: Log entry
```

Log lines must be indented deeper than their parent task line. Only logs matching the target date appear in the output.

## Build Configuration

- **esbuild.js** — entry `src/extension.ts` → `dist/extension.js`, platform node, format cjs. Production builds minify; dev builds include sourcemaps.
- **tsconfig.json** — target ES2022, module Node16, strict mode enabled.
- **eslint.config.mjs** — enforces camelCase/PascalCase naming, curly braces, strict equality, no throw literals, semicolons.
