use chrono::Local;
use clap::{Parser, Subcommand};
use parser_core::{build_tree_data_internal, FileInput};
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process;
use std::process::Command;

#[derive(Parser)]
#[command(name = "taski", version, about = "ジャーナルにメモを追記するCLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// タイムスタンプ付きでメモを追記
    Memo {
        /// タイムスタンプを付けない
        #[arg(long)]
        no_timestamp: bool,

        /// メモのテキスト（省略時はstdinから読み取り）
        text: Vec<String>,
    },
    /// タスク一覧を表示
    List {
        /// 出力フォーマット（json, yaml）
        #[arg(long, short)]
        format: Option<String>,
    },
    /// $HOME/taskiのGit同期を実行
    Sync,
    /// 今日のジャーナルファイルを開く
    Journal {
        /// パスを表示するだけ（エディタを開かない）
        #[arg(long)]
        print: bool,
    },
    /// タスクの完了状態を切り替え
    Toggle {
        /// 対象ファイルのパス
        file: PathBuf,
        /// 行番号（1始まり）
        line: usize,
    },
}

fn taski_dir() -> PathBuf {
    let home = env("HOME");
    PathBuf::from(home).join("taski")
}

fn journal_dir() -> PathBuf {
    taski_dir().join("journal")
}

fn env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| {
        eprintln!("エラー: {key}環境変数が設定されていません");
        process::exit(1);
    })
}

fn ensure_journal_file() -> PathBuf {
    let now = Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let year = now.format("%Y").to_string();
    let month = now.format("%m").to_string();

    let dir = journal_dir().join(&year).join(&month);
    fs::create_dir_all(&dir).unwrap_or_else(|e| {
        eprintln!("エラー: ディレクトリを作成できません: {e}");
        process::exit(1);
    });

    let file_path = dir.join(format!("{date_str}.md"));

    if !file_path.exists() {
        fs::write(&file_path, format!("# {date_str}\n\n")).unwrap_or_else(|e| {
            eprintln!("エラー: ファイルを作成できません: {e}");
            process::exit(1);
        });
    }

    file_path
}

fn append_memo(text: &str, no_timestamp: bool) {
    let file_path = ensure_journal_file();

    let line = if no_timestamp {
        format!("- {text}\n")
    } else {
        let time_str = Local::now().format("%H:%M").to_string();
        format!("- {time_str}: {text}\n")
    };

    let mut file = OpenOptions::new()
        .append(true)
        .open(&file_path)
        .unwrap_or_else(|e| {
            eprintln!("エラー: ファイルを開けません: {e}");
            process::exit(1);
        });

    file.write_all(line.as_bytes()).unwrap_or_else(|e| {
        eprintln!("エラー: ファイルに書き込めません: {e}");
        process::exit(1);
    });

    println!("追記しました: {}", file_path.display());
}

fn collect_md_files(dir: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_md_files_recursive(dir, &mut files);
    files.sort();
    files
}

fn collect_md_files_recursive(dir: &PathBuf, files: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files_recursive(&path, files);
        } else if path.extension().map_or(false, |ext| ext == "md") {
            files.push(path);
        }
    }
}

fn list_tasks(format: Option<String>) {
    let base_dir = taski_dir();
    if !base_dir.exists() {
        eprintln!("エラー: {} が見つかりません", base_dir.display());
        process::exit(1);
    }

    let md_files = collect_md_files(&base_dir);
    if md_files.is_empty() {
        println!("タスクが見つかりません");
        return;
    }

    let files: Vec<FileInput> = md_files
        .iter()
        .filter_map(|path| {
            let content = fs::read_to_string(path).ok()?;
            let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
            let file_name = path
                .strip_prefix(&base_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            Some(FileInput {
                file_name,
                file_uri: path.to_string_lossy().to_string(),
                lines,
            })
        })
        .collect();

    let today_str = Local::now().format("%Y-%m-%d").to_string();
    let tree = build_tree_data_internal(files, &today_str);

    if tree.is_empty() {
        println!("未完了のタスクはありません");
        return;
    }

    if let Some(fmt) = format {
        match fmt.as_str() {
            "json" => {
                let json = serde_json::to_string_pretty(&tree).unwrap_or_else(|e| {
                    eprintln!("エラー: JSON変換に失敗しました: {e}");
                    process::exit(1);
                });
                println!("{json}");
                return;
            }
            "yaml" => {
                let yaml = serde_yaml::to_string(&tree).unwrap_or_else(|e| {
                    eprintln!("エラー: YAML変換に失敗しました: {e}");
                    process::exit(1);
                });
                print!("{yaml}");
                return;
            }
            _ => {
                eprintln!("エラー: 未対応のフォーマットです: {fmt}");
                process::exit(1);
            }
        }
    }

    for (i, date_group) in tree.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("\x1b[1m{}\x1b[0m", date_group.label);

        for file_group in &date_group.file_groups {
            println!("  \x1b[36m{}\x1b[0m", file_group.file_name);
            for task in &file_group.tasks {
                let checkbox = if task.is_completed {
                    "\x1b[32m[x]\x1b[0m"
                } else {
                    "\x1b[33m[ ]\x1b[0m"
                };
                if task.log.is_empty() {
                    println!("    {} {}", checkbox, task.text);
                } else {
                    println!("    {} {}  \x1b[2m{}\x1b[0m", checkbox, task.text, task.log);
                }
            }
        }
    }
}

fn exec_git(args: &[&str]) -> Result<String, String> {
    let taski = taski_dir();
    let output = Command::new("git")
        .args(args)
        .current_dir(&taski)
        .output()
        .unwrap_or_else(|e| {
            eprintln!("エラー: gitコマンドの実行に失敗しました: {e}");
            process::exit(1);
        });

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn sync() {
    let taski = taski_dir();
    if !taski.exists() {
        eprintln!("エラー: {} が見つかりません", taski.display());
        process::exit(1);
    }

    if !taski.join(".git").exists() {
        eprintln!("エラー: {} はGitリポジトリではありません", taski.display());
        process::exit(1);
    }

    // 変更チェック
    let status = exec_git(&["status", "--porcelain"]).unwrap_or_else(|e| {
        eprintln!("エラー: git status に失敗しました: {e}");
        process::exit(1);
    });

    if !status.trim().is_empty() {
        println!("変更を検出しました。コミットします...");

        exec_git(&["add", "-A"]).unwrap_or_else(|e| {
            eprintln!("エラー: git add に失敗しました: {e}");
            process::exit(1);
        });

        let now = Local::now();
        let commit_msg = format!("taski: 自動同期 {}", now.format("%Y-%m-%d %H:%M"));

        exec_git(&["commit", "-m", &commit_msg]).unwrap_or_else(|e| {
            eprintln!("エラー: git commit に失敗しました: {e}");
            process::exit(1);
        });

        println!("コミット完了: {commit_msg}");
    } else {
        println!("変更はありません");
    }

    // Pull rebase
    println!("リモートから変更を取得します...");
    if let Err(e) = exec_git(&["pull", "--rebase"]) {
        if e.contains("CONFLICT") || e.contains("could not apply") || e.contains("Failed to merge")
        {
            eprintln!("コンフリクトを検出しました。Rebaseを中断します...");
            let _ = exec_git(&["rebase", "--abort"]);
            eprintln!("手動でコンフリクトを解決してください:");
            eprintln!("  cd {} && git pull --rebase", taski.display());
            process::exit(1);
        } else if e.contains("Could not resolve host") || e.contains("unable to access") {
            eprintln!("ネットワークエラー: {e}");
            process::exit(1);
        } else {
            eprintln!("エラー: git pull に失敗しました: {e}");
            process::exit(1);
        }
    }
    println!("Pull完了");

    // Push
    println!("リモートにプッシュします...");
    if let Err(e) = exec_git(&["push"]) {
        eprintln!("エラー: git push に失敗しました: {e}");
        process::exit(1);
    }
    println!("Push完了。同期が完了しました。");
}

fn open_journal(print_only: bool) {
    let file_path = ensure_journal_file();

    if print_only {
        println!("{}", file_path.display());
        return;
    }

    match std::env::var("EDITOR") {
        Ok(editor) => {
            let status = Command::new(&editor)
                .arg(&file_path)
                .status()
                .unwrap_or_else(|e| {
                    eprintln!("エラー: {editor} の起動に失敗しました: {e}");
                    process::exit(1);
                });

            if !status.success() {
                eprintln!("エラー: エディタが異常終了しました");
                process::exit(1);
            }
        }
        Err(_) => {
            println!("{}", file_path.display());
        }
    }
}

fn toggle_line(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    // "- [ ]" or "- [x]" パターンを探す（先頭の空白は許容）
    let dash_pos = bytes.iter().position(|&b| b != b' ' && b != b'\t')?;
    if bytes.get(dash_pos) != Some(&b'-') {
        return None;
    }

    // "- [" を探す
    let after_dash = &line[dash_pos + 1..];
    let bracket_offset = after_dash.find("[")?;
    let bracket_pos = dash_pos + 1 + bracket_offset;

    // "[" の前が空白のみであることを確認
    let between = &line[dash_pos + 1..bracket_pos];
    if !between.chars().all(|c| c == ' ') {
        return None;
    }

    // "[ ]" or "[x]" をチェック
    let check_char = bytes.get(bracket_pos + 1)?;
    if bytes.get(bracket_pos + 2) != Some(&b']') {
        return None;
    }

    match check_char {
        b' ' => {
            let mut result = line.to_string();
            result.replace_range(bracket_pos..bracket_pos + 3, "[x]");
            Some(result)
        }
        b'x' => {
            let mut result = line.to_string();
            result.replace_range(bracket_pos..bracket_pos + 3, "[ ]");
            Some(result)
        }
        _ => None,
    }
}

fn toggle_task(file: &PathBuf, line_num: usize) {
    if line_num == 0 {
        eprintln!("エラー: 行番号は1以上を指定してください");
        process::exit(1);
    }

    let content = fs::read_to_string(file).unwrap_or_else(|e| {
        eprintln!("エラー: ファイルを読み込めません: {e}");
        process::exit(1);
    });

    let mut lines: Vec<&str> = content.lines().collect();
    let trailing_newline = content.ends_with('\n');

    if line_num > lines.len() {
        eprintln!(
            "エラー: 行番号が範囲外です（ファイルは{}行です）",
            lines.len()
        );
        process::exit(1);
    }

    let idx = line_num - 1;
    let original = lines[idx];

    let toggled = toggle_line(original).unwrap_or_else(|| {
        eprintln!("エラー: 指定された行にタスクのチェックボックスがありません");
        process::exit(1);
    });

    lines[idx] = &toggled;

    let mut output = lines.join("\n");
    if trailing_newline {
        output.push('\n');
    }

    fs::write(file, output).unwrap_or_else(|e| {
        eprintln!("エラー: ファイルに書き込めません: {e}");
        process::exit(1);
    });

    println!("切り替えました: {}", toggled.trim());
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Memo { no_timestamp, text } => {
            let memo_text = if text.is_empty() {
                if io::stdin().is_terminal() {
                    eprintln!("エラー: メモのテキストを指定してください");
                    process::exit(1);
                }
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
                    eprintln!("エラー: stdinの読み取りに失敗しました: {e}");
                    process::exit(1);
                });
                buf.trim().to_string()
            } else {
                text.join(" ")
            };

            if memo_text.is_empty() {
                eprintln!("エラー: メモのテキストが空です");
                process::exit(1);
            }

            append_memo(&memo_text, no_timestamp);
        }
        Commands::List { format } => {
            list_tasks(format);
        }
        Commands::Sync => {
            sync();
        }
        Commands::Journal { print } => {
            open_journal(print);
        }
        Commands::Toggle { file, line } => {
            toggle_task(&file, line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_line_incomplete_to_complete() {
        assert_eq!(
            toggle_line("- [ ] タスク名"),
            Some("- [x] タスク名".to_string())
        );
    }

    #[test]
    fn test_toggle_line_complete_to_incomplete() {
        assert_eq!(
            toggle_line("- [x] タスク名"),
            Some("- [ ] タスク名".to_string())
        );
    }

    #[test]
    fn test_toggle_line_with_indent() {
        assert_eq!(
            toggle_line("    - [ ] インデントされたタスク"),
            Some("    - [x] インデントされたタスク".to_string())
        );
    }

    #[test]
    fn test_toggle_line_not_a_task() {
        assert_eq!(toggle_line("普通のテキスト"), None);
    }

    #[test]
    fn test_toggle_line_log_entry() {
        assert_eq!(toggle_line("    - 2026-03-17: ログ"), None);
    }

    #[test]
    fn test_toggle_line_empty() {
        assert_eq!(toggle_line(""), None);
    }
}
