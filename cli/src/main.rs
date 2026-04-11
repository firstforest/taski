use chrono::Local;
use clap::{Parser, Subcommand};
use parser_core::{build_tree_data_internal, extract_tags, FileInput, TreeDateGroup};
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

        /// 指定したタグを含むタスクのみ表示（例: --tag work）
        #[arg(long, short)]
        tag: Option<String>,
    },
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
    /// AGENTS.mdを生成して出力
    AgentsMd {
        /// 出力ファイルパス（省略時はstdoutに出力）
        #[arg(long, short)]
        output: Option<PathBuf>,
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

fn filter_tree_by_tag(tree: Vec<TreeDateGroup>, tag: &str) -> Vec<TreeDateGroup> {
    tree.into_iter()
        .filter_map(|mut date_group| {
            date_group.file_groups = date_group
                .file_groups
                .into_iter()
                .filter_map(|mut file_group| {
                    file_group.tasks.retain(|task| {
                        extract_tags(&task.text)
                            .iter()
                            .any(|t| t == tag)
                    });
                    if file_group.tasks.is_empty() {
                        None
                    } else {
                        Some(file_group)
                    }
                })
                .collect();
            if date_group.file_groups.is_empty() {
                None
            } else {
                Some(date_group)
            }
        })
        .collect()
}

fn list_tasks(format: Option<String>, tag: Option<String>) {
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

    let tree = if let Some(ref tag) = tag {
        filter_tree_by_tag(tree, tag)
    } else {
        tree
    };

    if tree.is_empty() {
        if tag.is_some() {
            println!("該当するタグのタスクが見つかりません");
        } else {
            println!("未完了のタスクはありません");
        }
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

fn agents_md_content() -> &'static str {
    include_str!("../AGENTS.md")
}

fn generate_agents_md(output: Option<PathBuf>) {
    let content = agents_md_content();

    match output {
        Some(path) => {
            fs::write(&path, content).unwrap_or_else(|e| {
                eprintln!("エラー: ファイルに書き込めません: {e}");
                process::exit(1);
            });
            println!("生成しました: {}", path.display());
        }
        None => {
            print!("{content}");
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
        Commands::List { format, tag } => {
            list_tasks(format, tag);
        }
        Commands::Journal { print } => {
            open_journal(print);
        }
        Commands::Toggle { file, line } => {
            toggle_task(&file, line);
        }
        Commands::AgentsMd { output } => {
            generate_agents_md(output);
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

    #[test]
    fn test_extract_tags_single() {
        assert_eq!(extract_tags("タスク #work"), vec!["work"]);
    }

    #[test]
    fn test_extract_tags_multiple() {
        assert_eq!(extract_tags("#work #urgent タスク"), vec!["work", "urgent"]);
    }

    #[test]
    fn test_extract_tags_none() {
        let result: Vec<String> = vec![];
        assert_eq!(extract_tags("タグなしタスク"), result);
    }

    #[test]
    fn test_extract_tags_japanese() {
        assert_eq!(extract_tags("タスク #仕事"), vec!["仕事"]);
    }

    #[test]
    fn test_filter_tree_by_tag() {
        use parser_core::{TreeDateGroup, TreeFileGroup, TreeTaskData};

        let tree = vec![TreeDateGroup {
            date_key: "2026-04-09".to_string(),
            label: "今日".to_string(),
            is_today: true,
            completed_count: 0,
            total_count: 2,
            file_groups: vec![TreeFileGroup {
                file_name: "test.md".to_string(),
                file_uri: "/test.md".to_string(),
                tasks: vec![
                    TreeTaskData {
                        is_completed: false,
                        text: "タスクA #work".to_string(),
                        file_uri: "/test.md".to_string(),
                        line: 1,
                        log: String::new(),
                        date: "2026-04-09".to_string(),
                    },
                    TreeTaskData {
                        is_completed: false,
                        text: "タスクB #personal".to_string(),
                        file_uri: "/test.md".to_string(),
                        line: 2,
                        log: String::new(),
                        date: "2026-04-09".to_string(),
                    },
                ],
            }],
        }];

        let filtered = filter_tree_by_tag(tree, "work");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].file_groups[0].tasks.len(), 1);
        assert_eq!(filtered[0].file_groups[0].tasks[0].text, "タスクA #work");
    }

    #[test]
    fn test_filter_tree_by_tag_no_match() {
        use parser_core::{TreeDateGroup, TreeFileGroup, TreeTaskData};

        let tree = vec![TreeDateGroup {
            date_key: "2026-04-09".to_string(),
            label: "今日".to_string(),
            is_today: true,
            completed_count: 0,
            total_count: 1,
            file_groups: vec![TreeFileGroup {
                file_name: "test.md".to_string(),
                file_uri: "/test.md".to_string(),
                tasks: vec![TreeTaskData {
                    is_completed: false,
                    text: "タスクA #work".to_string(),
                    file_uri: "/test.md".to_string(),
                    line: 1,
                    log: String::new(),
                    date: "2026-04-09".to_string(),
                }],
            }],
        }];

        let filtered = filter_tree_by_tag(tree, "nonexistent");
        assert!(filtered.is_empty());
    }
}
