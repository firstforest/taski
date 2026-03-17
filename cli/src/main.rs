use chrono::Local;
use clap::{Parser, Subcommand};
use parser_core::{build_tree_data_internal, FileInput};
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process;

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
    List,
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

fn append_memo(text: &str, no_timestamp: bool) {
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

    let line = if no_timestamp {
        format!("- {text}\n")
    } else {
        let time_str = now.format("%H:%M").to_string();
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

fn list_tasks() {
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
        Commands::List => {
            list_tasks();
        }
    }
}
