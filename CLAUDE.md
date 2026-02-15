# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VS Code extension ("taski") that aggregates tasks from markdown files across the workspace and displays them organized by date with clickable links back to source files. Written in TypeScript, bundled with esbuild, outputs to `dist/extension.js` as CommonJS. UI strings and code comments are in Japanese.

## Commands

- `npm run compile` — build:wasm + type-check + lint + esbuild (dev mode, with sourcemaps)
- `npm run watch` — parallel watch for esbuild and tsc (does not rebuild WASM; run `build:wasm` first)
- `npm run package` — build:wasm + type-check + lint + minified production build
- `npm run check-types` — TypeScript type-check only (`tsc --noEmit`)
- `npm run lint` — ESLint on `src/`
- `npm run build:wasm` — compiles Rust to WASM (already included in `compile` and `package`)
- `npm run test` — run VS Code extension tests (requires a VS Code instance; uses `@vscode/test-cli` + `@vscode/test-electron`)

Tests require compilation to `out/` first. The `pretest` script handles this: builds WASM, compiles TypeScript to `out/`, copies `src/pkg/` to `out/pkg/` (needed for WASM imports in tests), then runs `compile` and `lint`. The test runner picks up `out/test/**/*.test.js` as configured in `.vscode-test.mjs`.

## Architecture

Extension with eight commands (`showToday`, `refreshTasks`, `addTodayLog`, `addTomorrowLog`, `toggleTask`, `openTodayJournal`, `syncNow`, `openTaskLocation`) across four source files:

- **`src/extension.ts`** — extension activation, command registration, re-exports parser functions from `parser.ts`. Also provides a `CompletionItemProvider` for Markdown slash commands (`/today`, `/tomorrow`, `/now`) that insert the current date, tomorrow's date, or current time. All dates use local timezone, not UTC.

- **`src/taskTreeProvider.ts`** — TreeView implementation:
  - **`TaskTreeItem`** — TreeItem subclass with node types: `date`, `file`, `task`, `log`. Each type has color-coded icons (today=green, past=orange, completed=green, incomplete=yellow, etc.).
  - **`TaskTreeProvider`** — TreeDataProvider that scans markdown files, groups tasks by date, and builds the tree hierarchy (date → file → task → log). Display filtering: today's date shows all tasks (completed + incomplete) with progress counter; other dates and "日付なし" only show if they have incomplete tasks.

- **`src/parser.ts`** — TypeScript wrapper that re-exports WASM parser functions.

- **`src/gitSync.ts`** — Git auto-sync manager for `$HOME/taski`:
  - `GitSyncManager` handles automatic git add/commit/pull --rebase/push on a configurable interval (default 60s).
  - Debounced sync on file save (10s delay). Status bar item shows sync state.
  - Conflict handling: aborts rebase and shows warning modal with options to open terminal or retry.

- **`src/test/parseTasks.test.ts`** — Mocha unit tests for `parseTasks` and `parseTasksAllDates` (19 test cases covering date filtering, indentation rules, nested tasks, no-date tasks, edge cases).

The extension has no runtime dependencies beyond the VS Code API. The `vscode` module is marked external in esbuild since VS Code provides it at runtime.

## Task Markdown Format

The extension parses this structure in `.md` files:

```markdown
- [x] Task name
    - 2026-02-01: Log entry for this date
- [ ] Another task
    - 2026-02-01: Log entry
- [ ] Task with no logs
```

Log lines must be indented deeper than their parent task line. Tasks are displayed grouped by date (today first, then newest to oldest). Tasks without any log entries appear under a "日付なし" section.

## Configuration Settings

- **`taski.includeWorkspace`** — whether to scan the current workspace for markdown files (default: `false`)
- **`taski.excludeDirectories`** — glob patterns for directories to exclude from scanning (e.g., `**/archive/**`)
- **`taski.additionalDirectories`** — absolute paths of additional directories to scan beyond the workspace
- **`taski.gitAutoSync`** — enable automatic git sync for `$HOME/taski` (default: `true`)
- **`taski.gitSyncInterval`** — git sync interval in seconds (default: `60`, minimum: `30`)

By default, `$HOME/taski` is always scanned if it exists, regardless of configuration. Currently open markdown documents are also always included.

## Build Configuration

- **esbuild.js** — entry `src/extension.ts` → `dist/extension.js`, platform node, format cjs. Production builds minify; dev builds include sourcemaps.
- **tsconfig.json** — target ES2022, module Node16, strict mode enabled.
- **eslint.config.mjs** — enforces camelCase/PascalCase naming, curly braces, strict equality, no throw literals, semicolons.

## Prerequisites

- **Rust toolchain** — `rustc` and `cargo` must be installed (via [rustup](https://rustup.rs/))
- **wasm-pack** — required for building the WASM parser (`cargo install wasm-pack`)
- **wasm32-unknown-unknown target** — `rustup target add wasm32-unknown-unknown`

## WASM Parser

The pure parser functions (`parseTasks`, `parseTasksAllDates`) are implemented in Rust (`parser-wasm/src/lib.rs`) and compiled to WebAssembly via wasm-pack. The TypeScript wrapper (`src/parser.ts`) re-exports them, and `src/extension.ts` re-exports from `parser.ts` to maintain the existing public API.

- `npm run build:wasm` — compiles Rust to WASM and outputs glue code to `src/pkg/`
- The esbuild plugin (`wasmCopyPlugin` in `esbuild.js`) copies `parser_wasm_bg.wasm` to `dist/` during bundling
- `src/pkg/` is gitignored — it's generated output

## Journal Files

The `openTodayJournal` command creates/opens journal files at `$HOME/taski/journal/<year>/<month>/<year>-<month>-<day>.md`. Directories are created automatically if they don't exist.

## Release

GitHub Actions workflow (`.github/workflows/release.yml`) triggers on `v*` tags. It builds the VSIX package and creates a GitHub Release with the artifact. To release: bump version in `package.json`, commit, tag with `v<version>`, push tag.
