# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VS Code extension ("taski") that aggregates tasks from markdown files across the workspace and displays them organized by date with clickable links back to source files. Written in TypeScript, bundled with esbuild, outputs to `dist/extension.js` as CommonJS. Also includes a Rust CLI for terminal access to the same functionality. UI strings and code comments are in Japanese.

Cargo workspace with three crates:
- **`parser-core`** — shared Rust library with all parsing logic and data types. Used by both `parser-wasm` and `cli`.
- **`parser-wasm`** — thin WASM wrapper (via `wasm-bindgen`) that exposes `parser-core` functions to TypeScript.
- **`cli`** — standalone CLI binary (`taski`) that uses `parser-core` directly.

## Commands

ビルド・チェック系のコマンドは `mise run` タスクとして定義されている（`mise.toml`）。Rust ツールチェインの環境解決を mise が行うため、`cargo`・`wasm-bindgen` 等を直接呼ぶ必要はない。

### mise tasks（推奨）

- `mise run build-wasm` — Rust を WASM にコンパイルし `src/pkg/` に出力（cargo + wasm-bindgen）
- `mise run build-cli` — CLI バイナリをビルド（`cli/` crate, release mode）
- `mise run test-rust` — Rust テストを実行（`cargo test`）
- `mise run compile` — フルビルド（build-wasm + type-check + lint + esbuild）
- `mise run package` — プロダクションビルド（build-wasm + type-check + lint + minified esbuild）
- `mise run check` — TypeScript type-check + lint のみ
- `mise run release` — パッチバージョンを上げて main と tags を push

### npm scripts

- `npm run watch` — parallel watch for esbuild and tsc（WASM は再ビルドしない。先に `mise run build-wasm` を実行すること）
- `npm run check-types` — TypeScript type-check only (`tsc --noEmit`)
- `npm run lint` — ESLint on `src/`
- `npm run test` — VS Code 拡張テストを実行（VS Code インスタンスが必要。`@vscode/test-cli` + `@vscode/test-electron` を使用）

Tests require compilation to `out/` first. The `pretest` script handles this: builds WASM, compiles TypeScript to `out/`, copies `src/pkg/` to `out/pkg/` (needed for WASM imports in tests), then runs `compile` and `lint`. The test runner picks up `out/test/**/*.test.js` as configured in `.vscode-test.mjs`.

## Architecture

Extension with eight user-facing commands (`showToday`, `refreshTasks`, `addTodayLog`, `addTomorrowLog`, `toggleTask`, `openTodayJournal`, `syncNow`, `showSchedule`) plus one internal command (`openTaskLocation`). Two TreeViews: `taskiView` (date-grouped tasks) and `taskiTagView` (tag-grouped tasks).

### Parsing pipeline

All parsing logic lives in Rust (`parser-core/src/lib.rs`). Four public functions: `parseTasks`, `parseTasksAllDates`, `buildTreeData`, `buildScheduleData`. The VS Code extension accesses them via WASM (`parser-wasm` → `src/pkg/` → `src/parser.ts`), while the CLI calls `parser-core` directly. `src/pkg/` is gitignored generated output. The esbuild plugin (`wasmCopyPlugin`) copies `parser_wasm_bg.wasm` to `dist/` during bundling.

### Key source files

- **`src/extension.ts`** — activation, command registration, re-exports parser functions. Provides `CompletionItemProvider` for Markdown slash commands (`/today`, `/tomorrow`, `/now`). All dates use local timezone, not UTC.
- **`src/taskTreeProvider.ts`** — Date-grouped TreeView. Builds hierarchy (date → file → task → log) via WASM parser. Today shows all tasks with progress counter; other dates only show incomplete tasks.
- **`src/tagTreeProvider.ts`** — Tag-grouped TreeView. Extracts `#tag` from task text via `tagUtils.ts`.
- **`src/fileScanner.ts`** — Shared markdown file discovery (`findAllMarkdownUris`). Scans `$HOME/taski`, workspace, open documents, and additional directories.
- **`src/schedulePanel.ts`** — WebviewPanel with time-grid (15-min slots, 6:00–22:00). Plan vs. actual columns.
- **`src/gitSync.ts`** — Git auto-sync for `$HOME/taski` (add/commit/pull --rebase/push on interval). Debounced sync on file save.

The extension has no runtime dependencies beyond the VS Code API. The `vscode` module is marked external in esbuild.

## Task Markdown Format

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
- **`taski.taskAlert`** — タスクの開始時刻にアラート通知を表示する (default: `true`)
- **`taski.taskAlertLeadMinutes`** — タスク開始の何分前にアラートを表示するか (default: `1`, max: `30`)
- **`taski.gitAutoSync`** — enable automatic git sync for `$HOME/taski` (default: `true`)
- **`taski.gitSyncInterval`** — git sync interval in seconds (default: `60`, minimum: `30`)

By default, `$HOME/taski` is always scanned if it exists, regardless of configuration. Currently open markdown documents are also always included.

## Build Configuration

- **esbuild.js** — entry `src/extension.ts` → `dist/extension.js`, platform node, format cjs. Production builds minify; dev builds include sourcemaps.
- **tsconfig.json** — target ES2022, module Node16, strict mode enabled.
- **eslint.config.mjs** — enforces camelCase/PascalCase naming, curly braces, strict equality, no throw literals, semicolons.

## Prerequisites

- **Rust toolchain** — `rustc` and `cargo` are managed via [mise](https://mise.jdx.dev/)（`mise.toml` で定義）。Rust 関連のビルド・テストは `mise run <task>` で実行すること。`cargo` 等を直接呼ぶ必要がある場合は `mise exec --` 経由で実行する（例: `mise exec -- cargo build`）。直接実行すると `RUSTUP_HOME` が正しく解決されない場合がある。
- **wasm-pack** — WASM パーサーのビルドに必要（`mise exec -- cargo install wasm-pack`）
- **wasm32-unknown-unknown target** — `mise exec -- rustup target add wasm32-unknown-unknown`

## Journal Files

The `openTodayJournal` command and CLI `journal` subcommand both use the path `$HOME/taski/journal/<year>/<month>/<YYYY-MM-DD>.md`. Directories are created automatically if they don't exist.

## CLI (`cli/`)

Rust CLI (`taski-cli` crate) for accessing taski functionality from the terminal. Uses `parser-core` directly (no WASM). Built with `clap`.

### Subcommands

- `taski memo <text>` — append `- HH:MM: text` to today's journal. `--no-timestamp` omits the time prefix. Reads from stdin if no text and input is piped.
- `taski list` — display tasks grouped by date. `--format json|yaml` for structured output. `--tag <tag>` to filter by tag.
- `taski journal` — open today's journal in `$EDITOR`. `--print` outputs the path instead.
- `taski toggle <file> <line>` — toggle task completion at a specific file and line number (1-based).
- `taski schedule` — show today's schedule. `--format json|yaml`, `--date YYYY-MM-DD` for a specific date.
- `taski agents-md` — output bundled AGENTS.md content. `--output <path>` to write to file.

**Build**: `mise run build-cli` → binary at `target/release/taski`
**Install**: `cargo install --path cli`

## Release

`mise run release` bumps the patch version, pushes to main, and pushes tags. GitHub Actions workflow (`.github/workflows/release.yml`) triggers on `v*` tags, builds the VSIX package, and creates a GitHub Release with the artifact.
