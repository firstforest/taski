# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VS Code extension ("taski") that aggregates tasks from markdown files across the workspace and displays them organized by date with clickable links back to source files. Written in TypeScript, bundled with esbuild, outputs to `dist/extension.js` as CommonJS. Also includes a Rust CLI for appending memos to journal files from the terminal. Cargo workspace with two crates: `parser-wasm` (WASM parser) and `cli` (CLI tool). UI strings and code comments are in Japanese.

## Commands

ビルド・チェック系のコマンドは `mise run` タスクとして定義されている（`mise.toml`）。Rust ツールチェインの環境解決を mise が行うため、`cargo`・`wasm-bindgen` 等を直接呼ぶ必要はない。

### mise tasks（推奨）

- `mise run build-wasm` — Rust を WASM にコンパイルし `src/pkg/` に出力（cargo + wasm-bindgen）
- `mise run build-cli` — CLI バイナリをビルド（`cli/` crate, release mode）
- `mise run test-rust` — Rust テストを実行（`cargo test`）
- `mise run compile` — フルビルド（build-wasm + type-check + lint + esbuild）
- `mise run package` — プロダクションビルド（build-wasm + type-check + lint + minified esbuild）
- `mise run check` — TypeScript type-check + lint のみ

### npm scripts

- `npm run watch` — parallel watch for esbuild and tsc（WASM は再ビルドしない。先に `mise run build-wasm` を実行すること）
- `npm run check-types` — TypeScript type-check only (`tsc --noEmit`)
- `npm run lint` — ESLint on `src/`
- `npm run test` — VS Code 拡張テストを実行（VS Code インスタンスが必要。`@vscode/test-cli` + `@vscode/test-electron` を使用）

Tests require compilation to `out/` first. The `pretest` script handles this: builds WASM, compiles TypeScript to `out/`, copies `src/pkg/` to `out/pkg/` (needed for WASM imports in tests), then runs `compile` and `lint`. The test runner picks up `out/test/**/*.test.js` as configured in `.vscode-test.mjs`.

## Architecture

Extension with eight user-facing commands (`showToday`, `refreshTasks`, `addTodayLog`, `addTomorrowLog`, `toggleTask`, `openTodayJournal`, `syncNow`, `showSchedule`) plus one internal command (`openTaskLocation`). Two TreeViews: `taskiView` (date-grouped tasks) and `taskiTagView` (tag-grouped tasks).

- **`src/extension.ts`** — extension activation, command registration, re-exports parser functions from `parser.ts`. Also provides a `CompletionItemProvider` for Markdown slash commands (`/today`, `/tomorrow`, `/now`) that insert the current date, tomorrow's date, or current time. All dates use local timezone, not UTC.

- **`src/taskTreeProvider.ts`** — Date-grouped TreeView implementation:
  - **`TaskTreeItem`** — TreeItem subclass with node types: `date`, `file`, `task`, `log`. Each type has color-coded icons (today=green, past=orange, completed=green, incomplete=yellow, etc.).
  - **`TaskTreeProvider`** — TreeDataProvider that builds the tree hierarchy (date → file → task → log) using `buildTreeData` from the WASM parser. Display filtering: today's date shows all tasks (completed + incomplete) with progress counter; other dates and "日付なし" only show if they have incomplete tasks.

- **`src/tagTreeProvider.ts`** — Tag-grouped TreeView (`taskiTagView`). Scans all markdown files, extracts `#tag` from task text via `tagUtils.ts`, and builds a tree hierarchy (tag → file → task). Tags are sorted alphabetically with task counts.

- **`src/tagUtils.ts`** — `extractTags()` function that extracts `#hashtag` patterns from task text using regex.

- **`src/fileScanner.ts`** — Shared markdown file discovery logic (`findAllMarkdownUris`). Handles workspace scanning, open documents, `$HOME/taski`, and additional directories with exclude patterns. Used by both tree providers and the schedule panel.

- **`src/schedulePanel.ts`** — WebviewPanel (`showSchedule` command) that displays today's tasks in a time-grid (15-min slots, 6:00–22:00). Uses `buildScheduleData` from the WASM parser. Shows plan vs. actual columns with current time slot highlighted.

- **`src/parser.ts`** — TypeScript wrapper that imports from `./pkg/parser_wasm` and re-exports `parseTasks`, `parseTasksAllDates`, `buildTreeData`, and `buildScheduleData` with proper types.

- **`src/gitSync.ts`** — Git auto-sync manager for `$HOME/taski`:
  - `GitSyncManager` handles automatic git add/commit/pull --rebase/push on a configurable interval (default 60s).
  - Debounced sync on file save (10s delay). Status bar item shows sync state.
  - Conflict handling: aborts rebase and shows warning modal with options to open terminal or retry.

- **`src/test/parseTasks.test.ts`** — Mocha unit tests for `parseTasks` and `parseTasksAllDates` (19 test cases covering date filtering, indentation rules, nested tasks, no-date tasks, edge cases).

- **`src/test/tagUtils.test.ts`** — Unit tests for `extractTags`.

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

- **Rust toolchain** — `rustc` and `cargo` are managed via [mise](https://mise.jdx.dev/)（`mise.toml` で定義）。Rust 関連のビルド・テストは `mise run <task>` で実行すること。`cargo` 等を直接呼ぶ必要がある場合は `mise exec --` 経由で実行する（例: `mise exec -- cargo build`）。直接実行すると `RUSTUP_HOME` が正しく解決されない場合がある。
- **wasm-pack** — WASM パーサーのビルドに必要（`mise exec -- cargo install wasm-pack`）
- **wasm32-unknown-unknown target** — `mise exec -- rustup target add wasm32-unknown-unknown`

## WASM Parser

Four parser functions (`parseTasks`, `parseTasksAllDates`, `buildTreeData`, `buildScheduleData`) are implemented in Rust (`parser-wasm/src/lib.rs`) and compiled to WebAssembly via wasm-pack. The TypeScript wrapper (`src/parser.ts`) re-exports them with typed interfaces, and `src/extension.ts` re-exports from `parser.ts` to maintain the existing public API.

- `mise run build-wasm` — Rust を WASM にコンパイルし `src/pkg/` に出力
- The esbuild plugin (`wasmCopyPlugin` in `esbuild.js`) copies `parser_wasm_bg.wasm` to `dist/` during bundling
- `src/pkg/` is gitignored — it's generated output

## Journal Files

The `openTodayJournal` command creates/opens journal files at `$HOME/taski/journal/<year>/<month>/<year>-<month>-<day>.md`. Directories are created automatically if they don't exist.

## CLI (`cli/`)

Rust CLI (`taski-cli` crate) for appending memos to journal files from the terminal. Part of the Cargo workspace alongside `parser-wasm`.

- **Usage**: `taski memo <text>` appends `- HH:MM: text` to today's journal file. `--no-timestamp` omits the time prefix. Reads from stdin if no text arguments and input is piped.
- **Journal path**: `$HOME/taski/journal/<year>/<month>/<YYYY-MM-DD>.md` (same as `openTodayJournal` command)
- **Build**: `mise run build-cli` → binary at `target/release/taski`
- **Install**: `cargo install --path cli`
- **Dependencies**: `chrono` only

## Release

GitHub Actions workflow (`.github/workflows/release.yml`) triggers on `v*` tags. It builds the VSIX package and creates a GitHub Release with the artifact. To release: bump version in `package.json`, commit, tag with `v<version>`, push tag.
