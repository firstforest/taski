# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## ドキュメント

仕様・構成の詳細は `docs/` にまとめてある。本ファイルは作業時の操作情報（コマンド・前提環境）に絞る。

- **[docs/requirements.md](docs/requirements.md)** — 要求仕様。Markdown フォーマット、ファイル単位の自動タグ、Wiki リンク、各機能・コマンド・設定・CLI サブコマンド・ジャーナルのパス規約など。
- **[docs/architecture.md](docs/architecture.md)** — 全体構成、Cargo ワークスペース構成、パースパイプライン、主要ソースファイル、ビルド構成・配布フロー。

機能・フォーマット・設定・アーキテクチャに関する記述を更新する場合は、上記 docs を更新すること（本ファイルには重複させない）。

## Project Overview

VS Code extension ("taski") that aggregates tasks from markdown files across the workspace and displays them organized by date with clickable links back to source files. Written in TypeScript, bundled with esbuild, outputs to `dist/extension.js` as CommonJS. Also includes a Rust CLI for terminal access to the same functionality. UI strings and code comments are in Japanese.

全体構成（3 クレートの Cargo ワークスペースと WASM/CLI の関係）は [docs/architecture.md](docs/architecture.md) を参照。

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

ロジックを Rust に移す際は、テストも Rust 側で書くこと。

## Prerequisites

- **Rust toolchain** — `rustc` and `cargo` are managed via [mise](https://mise.jdx.dev/)（`mise.toml` で定義）。Rust 関連のビルド・テストは `mise run <task>` で実行すること。`cargo` 等を直接呼ぶ必要がある場合は `mise exec --` 経由で実行する（例: `mise exec -- cargo build`）。直接実行すると `RUSTUP_HOME` が正しく解決されない場合がある。
- **wasm-pack** — WASM パーサーのビルドに必要（`mise exec -- cargo install wasm-pack`）
- **wasm32-unknown-unknown target** — `mise exec -- rustup target add wasm32-unknown-unknown`

## CLI

CLI のビルドと配置:

- **Build**: `mise run build-cli` → binary at `target/release/taski`
- **Install**: `cargo install --path cli`

サブコマンドの一覧・仕様は [docs/requirements.md](docs/requirements.md) の「CLI 要求」を参照。
