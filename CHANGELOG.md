# Change Log

All notable changes to the "taski" extension will be documented in this file.

Check [Keep a Changelog](http://keepachangelog.com/) for recommendations on how to structure this file.

## [0.0.16] - 2026-02-15

### Added

- Markdown でスラッシュコマンド（`/today`, `/tomorrow`, `/now`）による日付・時刻挿入

## [0.0.15] - 2026-02-15

### Added

- `$HOME/taski` ディレクトリの Git 自動同期機能（`GitSyncManager`）
- ジャーナルファイル作成時に日付を自動挿入

### Changed

- Git sync のデフォルト同期間隔を 30 秒から 60 秒に変更

## [0.0.14] - 2026-02-07

### Added

- 今日のタスクに進捗表示（`x/y`）を追加

## [0.0.13] - 2026-02-07

### Added

- TreeView のアイコンに色を追加（今日=緑、過去=オレンジ、完了=緑、未完了=黄色など）
- ワークスペースのスキャンを無効化する設定（`taski.includeWorkspace`）

## [0.0.12] - 2026-02-07

### Fixed

- ログエントリ挿入時のインデントを 4 スペースから 2 スペースに変更

## [0.0.11] - 2026-02-07

### Changed

- タスク一覧の表示を Webview から TreeView に変更

### Added

- 今日の日付のジャーナルファイルを開くコマンド（`taski.openTodayJournal`）

## [0.0.10] - 2026-02-05

### Fixed

- GitHub Actions に Rust と wasm-pack のセットアップを追加（リリースビルド修正）

## [0.0.9] - 2026-02-05

### Changed

- 拡張機能名を daily-task-logger から Taski に変更

### Added

- デフォルトで `$HOME/taski` ディレクトリをスキャン対象に含める

## [0.0.8] - 2026-02-04

### Changed

- パーサー関数を Rust WASM で再実装
- タスク一覧で未完了タスクを上に表示するように変更

## [0.0.7] - 2026-02-03

### Added

- タスクの完了状態をトグルするコマンド（`taski.toggleTask` / `Cmd+Shift+X`）

## [0.0.6] - 2026-02-03

### Added

- 明日のログエントリ追加コマンド（`taski.addTomorrowLog` / `Cmd+Shift+Y`）

## [0.0.5] - 2026-02-02

### Added

- Markdown ファイル保存時にタスクビューを自動更新
- 今日以外の日付で完了タスクを非表示にするフィルタリング

## [0.0.4] - 2026-02-02

### Added

- 今日のログエントリ追加コマンド（`taski.addTodayLog` / `Cmd+Shift+T`）
- 対象外ディレクトリ指定機能（`taski.excludeDirectories`）

## [0.0.3] - 2026-02-01

### Added

- 追加ディレクトリのスキャン設定（`taski.additionalDirectories`）

## [0.0.2] - 2026-02-01

### Added

- 全日付のタスクと日付なしタスクを表示する機能

### Fixed

- タスクリンクをクリック可能にするため WebviewPanel に移行

## [0.0.1] - 2026-02-01

- Initial release
