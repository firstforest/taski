use chrono::Local;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!(
        "taski {VERSION} — ジャーナルにメモを追記するCLI

使い方:
  taski memo <テキスト>              タイムスタンプ付きでメモを追記
  taski memo --no-timestamp <テキスト>  タイムスタンプなしでメモを追記
  echo \"テキスト\" | taski memo      stdinからメモを読み取り

オプション:
  --help, -h       ヘルプを表示
  --version, -V    バージョンを表示"
    );
}

fn journal_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| {
        eprintln!("エラー: HOME環境変数が設定されていません");
        process::exit(1);
    });
    PathBuf::from(home).join("taski").join("journal")
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

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_help();
        process::exit(0);
    }

    match args[0].as_str() {
        "--help" | "-h" => {
            print_help();
        }
        "--version" | "-V" => {
            println!("taski {VERSION}");
        }
        "memo" => {
            let rest = &args[1..];
            let (no_timestamp, text_args) = if rest.first().map(|s| s.as_str()) == Some("--no-timestamp") {
                (true, &rest[1..])
            } else {
                (false, rest)
            };

            let text = if text_args.is_empty() {
                // stdinから読み取り
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
                text_args.join(" ")
            };

            if text.is_empty() {
                eprintln!("エラー: メモのテキストが空です");
                process::exit(1);
            }

            append_memo(&text, no_timestamp);
        }
        other => {
            eprintln!("エラー: 不明なサブコマンド '{other}'");
            eprintln!("ヘルプを表示するには: taski --help");
            process::exit(1);
        }
    }
}
