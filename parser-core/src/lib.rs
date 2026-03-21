use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

// === Internal types ===

struct CurrentTask {
    indent: usize,
    completed: bool,
    text: String,
    line: usize,
}

// === Output types ===

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedTask {
    pub is_completed: bool,
    pub text: String,
    pub line: usize,
    pub log: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedTaskWithDate {
    pub is_completed: bool,
    pub text: String,
    pub line: usize,
    pub log: String,
    pub date: String,
}

// === Tree types ===

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileInput {
    pub file_name: String,
    pub file_uri: String,
    pub lines: Vec<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TreeTaskData {
    pub is_completed: bool,
    pub text: String,
    pub file_uri: String,
    pub line: usize,
    pub log: String,
    pub date: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TreeFileGroup {
    pub file_name: String,
    pub file_uri: String,
    pub tasks: Vec<TreeTaskData>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TreeDateGroup {
    pub date_key: String,
    pub label: String,
    pub is_today: bool,
    pub file_groups: Vec<TreeFileGroup>,
    pub completed_count: usize,
    pub total_count: usize,
}

// === Schedule types ===

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleEntry {
    pub task_text: String,
    pub task_line: usize,
    pub is_completed: bool,
    pub log_text: String,
    pub log_line: usize,
    pub time: String,
    pub file_uri: String,
}

// === Parsing logic ===

pub fn parse_tasks_internal(lines: &[String], target_date: &str) -> Vec<ParsedTask> {
    let task_re = Regex::new(r"^(\s*)-\s*\[([ x])\]\s*(.*)").unwrap();
    let date_re = Regex::new(r"^(\s*)-\s*(\d{4}-\d{2}-\d{2}):\s*(.*)").unwrap();

    let mut tasks: Vec<ParsedTask> = Vec::new();
    let mut current_task: Option<CurrentTask> = None;

    for (i, text) in lines.iter().enumerate() {
        if let Some(caps) = task_re.captures(text) {
            current_task = Some(CurrentTask {
                indent: caps[1].len(),
                completed: &caps[2] == "x",
                text: caps[3].to_string(),
                line: i,
            });
            continue;
        }

        if let Some(caps) = date_re.captures(text) {
            if let Some(ref ct) = current_task {
                let date_indent = caps[1].len();
                let date_str = &caps[2];
                let log_content = &caps[3];

                if date_str == target_date && date_indent > ct.indent {
                    tasks.push(ParsedTask {
                        is_completed: ct.completed,
                        text: ct.text.clone(),
                        line: ct.line,
                        log: log_content.to_string(),
                    });
                }
            }
        }
    }

    tasks
}

pub fn parse_all_dates_internal(lines: &[String]) -> Vec<ParsedTaskWithDate> {
    let task_re = Regex::new(r"^(\s*)-\s*\[([ x])\]\s*(.*)").unwrap();
    let date_re = Regex::new(r"^(\s*)-\s*(\d{4}-\d{2}-\d{2}):\s*(.*)").unwrap();

    let mut tasks: Vec<ParsedTaskWithDate> = Vec::new();
    let mut current_task: Option<CurrentTask> = None;
    let mut current_task_has_log = false;

    for (i, text) in lines.iter().enumerate() {
        if let Some(caps) = task_re.captures(text) {
            if let Some(ref ct) = current_task {
                if !current_task_has_log {
                    tasks.push(ParsedTaskWithDate {
                        is_completed: ct.completed,
                        text: ct.text.clone(),
                        line: ct.line,
                        log: String::new(),
                        date: String::new(),
                    });
                }
            }
            current_task = Some(CurrentTask {
                indent: caps[1].len(),
                completed: &caps[2] == "x",
                text: caps[3].to_string(),
                line: i,
            });
            current_task_has_log = false;
            continue;
        }

        if let Some(caps) = date_re.captures(text) {
            if let Some(ref ct) = current_task {
                let date_indent = caps[1].len();
                let date_str = &caps[2];
                let log_content = &caps[3];

                if date_indent > ct.indent {
                    tasks.push(ParsedTaskWithDate {
                        is_completed: ct.completed,
                        text: ct.text.clone(),
                        line: ct.line,
                        log: log_content.to_string(),
                        date: date_str.to_string(),
                    });
                    current_task_has_log = true;
                }
            }
        }
    }

    if let Some(ref ct) = current_task {
        if !current_task_has_log {
            tasks.push(ParsedTaskWithDate {
                is_completed: ct.completed,
                text: ct.text.clone(),
                line: ct.line,
                log: String::new(),
                date: String::new(),
            });
        }
    }

    tasks
}

pub fn build_tree_data_internal(files: Vec<FileInput>, today_str: &str) -> Vec<TreeDateGroup> {
    // date -> Vec<(file_name, file_uri, tasks)>
    let mut date_map: HashMap<String, Vec<(String, String, Vec<TreeTaskData>)>> = HashMap::new();

    for file in &files {
        let parsed = parse_all_dates_internal(&file.lines);
        if parsed.is_empty() {
            continue;
        }

        // Group by date within this file
        let mut by_date: HashMap<String, Vec<TreeTaskData>> = HashMap::new();
        for t in parsed {
            by_date
                .entry(t.date.clone())
                .or_default()
                .push(TreeTaskData {
                    is_completed: t.is_completed,
                    text: t.text,
                    file_uri: file.file_uri.clone(),
                    line: t.line,
                    log: t.log,
                    date: t.date,
                });
        }

        for (date, tasks) in by_date {
            date_map.entry(date).or_default().push((
                file.file_name.clone(),
                file.file_uri.clone(),
                tasks,
            ));
        }
    }

    let mut result: Vec<TreeDateGroup> = Vec::new();

    // 今日のタスク（全タスク表示 + 進捗計算）
    if let Some(groups) = date_map.remove(today_str) {
        let mut file_groups: Vec<TreeFileGroup> = Vec::new();
        let mut completed_count = 0;
        let mut total_count = 0;

        for (file_name, file_uri, mut tasks) in groups {
            total_count += tasks.len();
            completed_count += tasks.iter().filter(|t| t.is_completed).count();
            // 未完了を先にソート
            tasks.sort_by_key(|t| t.is_completed);
            file_groups.push(TreeFileGroup {
                file_name,
                file_uri,
                tasks,
            });
        }

        if !file_groups.is_empty() {
            result.push(TreeDateGroup {
                label: format!(
                    "今日 ({}) ({}/{})",
                    today_str, completed_count, total_count
                ),
                date_key: today_str.to_string(),
                is_today: true,
                file_groups,
                completed_count,
                total_count,
            });
        }
    }

    // その他の日付（降順、未完了のみ、全完了なら除外）
    let mut other_dates: Vec<String> = date_map.keys().filter(|d| !d.is_empty()).cloned().collect();
    other_dates.sort();
    other_dates.reverse();

    for date in other_dates {
        let groups = date_map.remove(&date).unwrap();
        let mut file_groups: Vec<TreeFileGroup> = Vec::new();
        let mut has_incomplete = false;

        for (file_name, file_uri, tasks) in groups {
            let incomplete_tasks: Vec<TreeTaskData> =
                tasks.into_iter().filter(|t| !t.is_completed).collect();
            if !incomplete_tasks.is_empty() {
                has_incomplete = true;
                file_groups.push(TreeFileGroup {
                    file_name,
                    file_uri,
                    tasks: incomplete_tasks,
                });
            }
        }

        if has_incomplete {
            result.push(TreeDateGroup {
                label: date.clone(),
                date_key: date,
                is_today: false,
                file_groups,
                completed_count: 0,
                total_count: 0,
            });
        }
    }

    // 日付なし（未完了のみ）
    if let Some(groups) = date_map.remove("") {
        let mut file_groups: Vec<TreeFileGroup> = Vec::new();
        let mut has_incomplete = false;

        for (file_name, file_uri, tasks) in groups {
            let incomplete_tasks: Vec<TreeTaskData> =
                tasks.into_iter().filter(|t| !t.is_completed).collect();
            if !incomplete_tasks.is_empty() {
                has_incomplete = true;
                file_groups.push(TreeFileGroup {
                    file_name,
                    file_uri,
                    tasks: incomplete_tasks,
                });
            }
        }

        if has_incomplete {
            result.push(TreeDateGroup {
                label: "日付なし".to_string(),
                date_key: String::new(),
                is_today: false,
                file_groups,
                completed_count: 0,
                total_count: 0,
            });
        }
    }

    result
}

// === Schedule parsing ===

pub fn parse_schedule_internal(lines: &[String], target_date: &str) -> Vec<ScheduleEntry> {
    let task_re = Regex::new(r"^(\s*)-\s*\[([ x])\]\s*(.*)").unwrap();
    let date_re =
        Regex::new(r"^(\s*)-\s*(\d{4}-\d{2}-\d{2})(?:\s+(\d{2}:\d{2}))?:\s*(.*)").unwrap();
    let time_memo_re = Regex::new(r"^- (\d{1,2}:\d{2}): (.+)").unwrap();
    let heading_date_re = Regex::new(r"^#\s+(\d{4}-\d{2}-\d{2})").unwrap();

    // ジャーナルファイルの日付見出しが target_date と一致するか判定
    let is_target_date_file = lines.iter().any(|line| {
        heading_date_re
            .captures(line)
            .map_or(false, |caps| &caps[1] == target_date)
    });

    let mut entries: Vec<ScheduleEntry> = Vec::new();
    let mut current_task: Option<CurrentTask> = None;

    for (i, text) in lines.iter().enumerate() {
        if let Some(caps) = task_re.captures(text) {
            current_task = Some(CurrentTask {
                indent: caps[1].len(),
                completed: &caps[2] == "x",
                text: caps[3].to_string(),
                line: i,
            });
            continue;
        }

        if let Some(caps) = date_re.captures(text) {
            if let Some(ref ct) = current_task {
                let date_indent = caps[1].len();
                let date_str = &caps[2];
                let time_str = caps.get(3).map_or("", |m| m.as_str());
                let log_content = &caps[4];

                if date_str == target_date && date_indent > ct.indent {
                    entries.push(ScheduleEntry {
                        task_text: ct.text.clone(),
                        task_line: ct.line,
                        is_completed: ct.completed,
                        log_text: log_content.to_string(),
                        log_line: i,
                        time: time_str.to_string(),
                        file_uri: String::new(),
                    });
                }
            }
            continue;
        }

        // 時刻メモ: ジャーナルファイル内のトップレベル「- HH:MM: テキスト」行
        if is_target_date_file {
            if let Some(caps) = time_memo_re.captures(text) {
                let time_str = &caps[1];
                let memo_text = &caps[2];
                // 時刻を2桁にパディング（例: "9:30" → "09:30"）
                let time_padded = if time_str.len() == 4 {
                    format!("0{}", time_str)
                } else {
                    time_str.to_string()
                };
                entries.push(ScheduleEntry {
                    task_text: String::new(),
                    task_line: i,
                    is_completed: false,
                    log_text: memo_text.to_string(),
                    log_line: i,
                    time: time_padded,
                    file_uri: String::new(),
                });
            }
        }
    }

    entries
}

pub fn build_schedule_data_internal(
    files: Vec<FileInput>,
    target_date: &str,
) -> Vec<ScheduleEntry> {
    let mut all_entries: Vec<ScheduleEntry> = Vec::new();

    for file in &files {
        let mut entries = parse_schedule_internal(&file.lines, target_date);
        for entry in &mut entries {
            entry.file_uri = file.file_uri.clone();
        }
        all_entries.extend(entries);
    }

    // 時刻順にソート（空文字は末尾）
    all_entries.sort_by(|a, b| {
        match (a.time.is_empty(), b.time.is_empty()) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.time.cmp(&b.time),
        }
    });

    all_entries
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn s(str: &str) -> String {
        str.to_string()
    }

    fn lines(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| s.to_string()).collect()
    }

    // --- parse_tasks_internal tests ---

    #[test]
    fn test_parse_tasks_basic_incomplete() {
        let l = lines(&["- [ ] タスクA", "    - 2026-02-01: ログA"]);
        let result = parse_tasks_internal(&l, "2026-02-01");
        assert_eq!(result.len(), 1);
        assert!(!result[0].is_completed);
        assert_eq!(result[0].text, "タスクA");
        assert_eq!(result[0].log, "ログA");
        assert_eq!(result[0].line, 0);
    }

    #[test]
    fn test_parse_tasks_basic_completed() {
        let l = lines(&["- [x] 完了タスク", "    - 2026-02-01: 完了ログ"]);
        let result = parse_tasks_internal(&l, "2026-02-01");
        assert_eq!(result.len(), 1);
        assert!(result[0].is_completed);
        assert_eq!(result[0].text, "完了タスク");
        assert_eq!(result[0].log, "完了ログ");
    }

    #[test]
    fn test_parse_tasks_only_target_date() {
        let l = lines(&[
            "- [ ] タスク",
            "    - 2026-01-31: 昨日のログ",
            "    - 2026-02-01: 今日のログ",
            "    - 2026-02-02: 明日のログ",
        ]);
        let result = parse_tasks_internal(&l, "2026-02-01");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].log, "今日のログ");
    }

    #[test]
    fn test_parse_tasks_multiple() {
        let l = lines(&[
            "- [ ] タスク1",
            "    - 2026-02-01: ログ1",
            "- [x] タスク2",
            "    - 2026-02-01: ログ2",
        ]);
        let result = parse_tasks_internal(&l, "2026-02-01");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "タスク1");
        assert_eq!(result[1].text, "タスク2");
        assert!(result[1].is_completed);
    }

    #[test]
    fn test_parse_tasks_same_date_multiple_logs() {
        let l = lines(&[
            "- [ ] タスク",
            "    - 2026-02-01: ログA",
            "    - 2026-02-01: ログB",
        ]);
        let result = parse_tasks_internal(&l, "2026-02-01");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].log, "ログA");
        assert_eq!(result[1].log, "ログB");
    }

    #[test]
    fn test_parse_tasks_shallow_indent_ignored() {
        let l = lines(&[
            "    - [ ] タスク（インデント4）",
            "    - 2026-02-01: 同レベルのログ",
            "  - 2026-02-01: 浅いインデントのログ",
        ]);
        let result = parse_tasks_internal(&l, "2026-02-01");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_tasks_no_task_with_log() {
        let l = lines(&["    - 2026-02-01: 孤立したログ"]);
        let result = parse_tasks_internal(&l, "2026-02-01");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_tasks_empty() {
        let result = parse_tasks_internal(&[], "2026-02-01");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_tasks_no_log() {
        let l = lines(&["- [ ] ログなしタスク"]);
        let result = parse_tasks_internal(&l, "2026-02-01");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_tasks_line_numbers() {
        let l = lines(&[
            "# ヘッダー",
            "",
            "- [ ] 3行目のタスク",
            "    - 2026-02-01: ログ",
        ]);
        let result = parse_tasks_internal(&l, "2026-02-01");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].line, 2);
    }

    #[test]
    fn test_parse_tasks_with_unrelated_lines() {
        let l = lines(&[
            "- [ ] タスク1",
            "    - 2026-02-01: ログ1",
            "",
            "これは普通のテキスト",
            "",
            "- [x] タスク2",
            "    - 2026-02-01: ログ2",
        ]);
        let result = parse_tasks_internal(&l, "2026-02-01");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "タスク1");
        assert_eq!(result[1].text, "タスク2");
    }

    #[test]
    fn test_parse_tasks_nested() {
        let l = lines(&[
            "- [ ] 親タスク",
            "    - [ ] 子タスク",
            "        - 2026-02-01: 子のログ",
        ]);
        let result = parse_tasks_internal(&l, "2026-02-01");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "子タスク");
        assert_eq!(result[0].log, "子のログ");
    }

    // --- parse_all_dates_internal tests ---

    #[test]
    fn test_parse_all_dates_basic() {
        let l = lines(&[
            "- [ ] タスクA",
            "    - 2026-01-31: 昨日のログ",
            "    - 2026-02-01: 今日のログ",
            "    - 2026-02-02: 明日のログ",
        ]);
        let result = parse_all_dates_internal(&l);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].date, "2026-01-31");
        assert_eq!(result[0].log, "昨日のログ");
        assert_eq!(result[1].date, "2026-02-01");
        assert_eq!(result[1].log, "今日のログ");
        assert_eq!(result[2].date, "2026-02-02");
        assert_eq!(result[2].log, "明日のログ");
    }

    #[test]
    fn test_parse_all_dates_multiple_tasks() {
        let l = lines(&[
            "- [ ] タスク1",
            "    - 2026-02-01: ログ1",
            "- [x] タスク2",
            "    - 2026-01-30: ログ2",
        ]);
        let result = parse_all_dates_internal(&l);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "タスク1");
        assert_eq!(result[0].date, "2026-02-01");
        assert_eq!(result[1].text, "タスク2");
        assert_eq!(result[1].date, "2026-01-30");
    }

    #[test]
    fn test_parse_all_dates_shallow_indent_ignored() {
        let l = lines(&[
            "    - [ ] タスク（インデント4）",
            "    - 2026-02-01: 同レベルのログ",
        ]);
        let result = parse_all_dates_internal(&l);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "タスク（インデント4）");
        assert_eq!(result[0].date, "");
        assert_eq!(result[0].log, "");
    }

    #[test]
    fn test_parse_all_dates_empty_input() {
        let result = parse_all_dates_internal(&[]);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_all_dates_no_log_task() {
        let l = lines(&["- [ ] ログなしタスク"]);
        let result = parse_all_dates_internal(&l);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "ログなしタスク");
        assert_eq!(result[0].date, "");
        assert_eq!(result[0].log, "");
    }

    #[test]
    fn test_parse_all_dates_mixed() {
        let l = lines(&[
            "- [ ] タスク1",
            "    - 2026-02-01: ログ1",
            "- [ ] タスク2",
            "- [x] タスク3",
            "    - 2026-01-30: ログ3",
        ]);
        let result = parse_all_dates_internal(&l);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].text, "タスク1");
        assert_eq!(result[0].date, "2026-02-01");
        assert_eq!(result[1].text, "タスク2");
        assert_eq!(result[1].date, "");
        assert_eq!(result[2].text, "タスク3");
        assert_eq!(result[2].date, "2026-01-30");
    }

    // --- build_tree_data_internal tests ---

    #[test]
    fn test_build_tree_data_empty() {
        let result = build_tree_data_internal(vec![], "2026-02-01");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_build_tree_data_today_all_tasks_shown() {
        let files = vec![FileInput {
            file_name: s("test.md"),
            file_uri: s("file:///test.md"),
            lines: lines(&[
                "- [ ] 未完了タスク",
                "    - 2026-02-01: ログ1",
                "- [x] 完了タスク",
                "    - 2026-02-01: ログ2",
            ]),
        }];
        let result = build_tree_data_internal(files, "2026-02-01");
        assert_eq!(result.len(), 1);
        assert!(result[0].is_today);
        assert_eq!(result[0].completed_count, 1);
        assert_eq!(result[0].total_count, 2);
        assert_eq!(result[0].label, "今日 (2026-02-01) (1/2)");
        let tasks = &result[0].file_groups[0].tasks;
        assert_eq!(tasks.len(), 2);
        // 未完了が先
        assert!(!tasks[0].is_completed);
        assert!(tasks[1].is_completed);
    }

    #[test]
    fn test_build_tree_data_other_date_incomplete_only() {
        let files = vec![FileInput {
            file_name: s("test.md"),
            file_uri: s("file:///test.md"),
            lines: lines(&[
                "- [ ] 未完了タスク",
                "    - 2026-01-30: ログ1",
                "- [x] 完了タスク",
                "    - 2026-01-30: ログ2",
            ]),
        }];
        let result = build_tree_data_internal(files, "2026-02-01");
        assert_eq!(result.len(), 1);
        assert!(!result[0].is_today);
        assert_eq!(result[0].date_key, "2026-01-30");
        let tasks = &result[0].file_groups[0].tasks;
        assert_eq!(tasks.len(), 1);
        assert!(!tasks[0].is_completed);
    }

    #[test]
    fn test_build_tree_data_other_date_all_complete_excluded() {
        let files = vec![FileInput {
            file_name: s("test.md"),
            file_uri: s("file:///test.md"),
            lines: lines(&["- [x] 完了タスク", "    - 2026-01-30: ログ"]),
        }];
        let result = build_tree_data_internal(files, "2026-02-01");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_build_tree_data_no_date() {
        let files = vec![FileInput {
            file_name: s("test.md"),
            file_uri: s("file:///test.md"),
            lines: lines(&["- [ ] 日付なしタスク"]),
        }];
        let result = build_tree_data_internal(files, "2026-02-01");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].date_key, "");
        assert_eq!(result[0].label, "日付なし");
    }

    #[test]
    fn test_build_tree_data_no_date_all_complete_excluded() {
        let files = vec![FileInput {
            file_name: s("test.md"),
            file_uri: s("file:///test.md"),
            lines: lines(&["- [x] 完了日付なしタスク"]),
        }];
        let result = build_tree_data_internal(files, "2026-02-01");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_build_tree_data_sort_order() {
        let files = vec![FileInput {
            file_name: s("test.md"),
            file_uri: s("file:///test.md"),
            lines: lines(&[
                "- [ ] 今日のタスク",
                "    - 2026-02-01: ログ",
                "- [ ] 古いタスク",
                "    - 2026-01-15: ログ",
                "- [ ] 昨日のタスク",
                "    - 2026-01-31: ログ",
                "- [ ] 日付なしタスク",
            ]),
        }];
        let result = build_tree_data_internal(files, "2026-02-01");
        assert_eq!(result.len(), 4);
        // 今日が最初
        assert!(result[0].is_today);
        assert_eq!(result[0].date_key, "2026-02-01");
        // 降順
        assert_eq!(result[1].date_key, "2026-01-31");
        assert_eq!(result[2].date_key, "2026-01-15");
        // 日付なしが最後
        assert_eq!(result[3].date_key, "");
    }

    #[test]
    fn test_build_tree_data_multiple_files() {
        let files = vec![
            FileInput {
                file_name: s("file1.md"),
                file_uri: s("file:///file1.md"),
                lines: lines(&["- [ ] タスク1", "    - 2026-02-01: ログ1"]),
            },
            FileInput {
                file_name: s("file2.md"),
                file_uri: s("file:///file2.md"),
                lines: lines(&["- [ ] タスク2", "    - 2026-02-01: ログ2"]),
            },
        ];
        let result = build_tree_data_internal(files, "2026-02-01");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file_groups.len(), 2);
    }

    #[test]
    fn test_build_tree_data_task_sort_incomplete_first() {
        let files = vec![FileInput {
            file_name: s("test.md"),
            file_uri: s("file:///test.md"),
            lines: lines(&[
                "- [x] 完了タスク1",
                "    - 2026-02-01: ログ1",
                "- [ ] 未完了タスク",
                "    - 2026-02-01: ログ2",
                "- [x] 完了タスク2",
                "    - 2026-02-01: ログ3",
            ]),
        }];
        let result = build_tree_data_internal(files, "2026-02-01");
        let tasks = &result[0].file_groups[0].tasks;
        assert!(!tasks[0].is_completed);
        assert!(tasks[1].is_completed);
        assert!(tasks[2].is_completed);
    }

    // --- parse_schedule_internal tests ---

    #[test]
    fn test_parse_schedule_with_time() {
        let l = lines(&["- [ ] タスクA", "    - 2026-03-21 09:00: ミーティング"]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].task_text, "タスクA");
        assert_eq!(result[0].time, "09:00");
        assert_eq!(result[0].log_text, "ミーティング");
        assert!(!result[0].is_completed);
    }

    #[test]
    fn test_parse_schedule_no_time() {
        let l = lines(&["- [ ] タスクA", "    - 2026-03-21: ログ"]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].time, "");
        assert_eq!(result[0].log_text, "ログ");
    }

    #[test]
    fn test_parse_schedule_log_line() {
        let l = lines(&[
            "# ヘッダー",
            "",
            "- [ ] タスク",
            "    - 2026-03-21 10:00: ログ",
        ]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].task_line, 2);
        assert_eq!(result[0].log_line, 3);
    }

    #[test]
    fn test_parse_schedule_indent_rules() {
        let l = lines(&[
            "    - [ ] タスク（インデント4）",
            "    - 2026-03-21 09:00: 同レベルのログ",
        ]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_schedule_multiple_times() {
        let l = lines(&[
            "- [ ] タスクA",
            "    - 2026-03-21 09:00: 朝",
            "    - 2026-03-21 14:00: 午後",
        ]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].time, "09:00");
        assert_eq!(result[0].log_text, "朝");
        assert_eq!(result[1].time, "14:00");
        assert_eq!(result[1].log_text, "午後");
    }

    #[test]
    fn test_parse_schedule_wrong_date_ignored() {
        let l = lines(&[
            "- [ ] タスク",
            "    - 2026-03-20 09:00: 昨日のログ",
            "    - 2026-03-21 09:00: 今日のログ",
        ]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].log_text, "今日のログ");
    }

    #[test]
    fn test_parse_schedule_completed_task() {
        let l = lines(&["- [x] 完了タスク", "    - 2026-03-21 10:00: 完了"]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 1);
        assert!(result[0].is_completed);
    }

    // --- parse_schedule_internal time memo tests ---

    #[test]
    fn test_parse_schedule_time_memo_in_journal() {
        let l = lines(&[
            "# 2026-03-21",
            "",
            "- 09:30: 散歩した",
            "- 14:00: コーヒー飲んだ",
        ]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].time, "09:30");
        assert_eq!(result[0].log_text, "散歩した");
        assert_eq!(result[0].task_text, "");
        assert!(!result[0].is_completed);
        assert_eq!(result[1].time, "14:00");
        assert_eq!(result[1].log_text, "コーヒー飲んだ");
    }

    #[test]
    fn test_parse_schedule_time_memo_single_digit_hour() {
        let l = lines(&["# 2026-03-21", "", "- 9:30: 朝の散歩"]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].time, "09:30");
        assert_eq!(result[0].log_text, "朝の散歩");
    }

    #[test]
    fn test_parse_schedule_time_memo_wrong_date_ignored() {
        let l = lines(&["# 2026-03-20", "", "- 09:30: 昨日のメモ"]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_schedule_time_memo_no_heading_ignored() {
        // 日付見出しがないファイルでは時刻メモを拾わない
        let l = lines(&["- 09:30: メモ"]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_schedule_time_memo_mixed_with_tasks() {
        let l = lines(&[
            "# 2026-03-21",
            "",
            "- 09:00: 朝ごはん",
            "- [ ] タスクA",
            "    - 2026-03-21 10:00: 作業開始",
            "- 12:00: 昼休み",
        ]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 3);
        // 時刻メモ
        assert_eq!(result[0].time, "09:00");
        assert_eq!(result[0].log_text, "朝ごはん");
        assert_eq!(result[0].task_text, "");
        // タスクログ
        assert_eq!(result[1].time, "10:00");
        assert_eq!(result[1].log_text, "作業開始");
        assert_eq!(result[1].task_text, "タスクA");
        // 時刻メモ
        assert_eq!(result[2].time, "12:00");
        assert_eq!(result[2].log_text, "昼休み");
        assert_eq!(result[2].task_text, "");
    }

    #[test]
    fn test_parse_schedule_indented_time_not_memo() {
        // インデントされた行は時刻メモとして拾わない
        let l = lines(&["# 2026-03-21", "", "  - 09:30: インデントあり"]);
        let result = parse_schedule_internal(&l, "2026-03-21");
        assert_eq!(result.len(), 0);
    }

    // --- build_schedule_data_internal tests ---

    #[test]
    fn test_build_schedule_data_sorted_by_time() {
        let files = vec![FileInput {
            file_name: s("test.md"),
            file_uri: s("file:///test.md"),
            lines: lines(&[
                "- [ ] タスクB",
                "    - 2026-03-21 14:00: 午後",
                "- [ ] タスクA",
                "    - 2026-03-21 09:00: 朝",
                "- [ ] タスクC",
                "    - 2026-03-21: 時刻なし",
            ]),
        }];
        let result = build_schedule_data_internal(files, "2026-03-21");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].time, "09:00");
        assert_eq!(result[0].task_text, "タスクA");
        assert_eq!(result[1].time, "14:00");
        assert_eq!(result[1].task_text, "タスクB");
        assert_eq!(result[2].time, "");
        assert_eq!(result[2].task_text, "タスクC");
    }

    #[test]
    fn test_build_schedule_data_multiple_files() {
        let files = vec![
            FileInput {
                file_name: s("file1.md"),
                file_uri: s("file:///file1.md"),
                lines: lines(&["- [ ] タスク1", "    - 2026-03-21 09:00: ログ1"]),
            },
            FileInput {
                file_name: s("file2.md"),
                file_uri: s("file:///file2.md"),
                lines: lines(&["- [ ] タスク2", "    - 2026-03-21 10:00: ログ2"]),
            },
        ];
        let result = build_schedule_data_internal(files, "2026-03-21");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].file_uri, "file:///file1.md");
        assert_eq!(result[1].file_uri, "file:///file2.md");
    }

    #[test]
    fn test_parse_schedule_backward_compat_old_parser() {
        // 既存パーサーは時刻をログテキストの一部として扱う
        let l = lines(&["- [ ] タスク", "    - 2026-03-21 09:00: ログ"]);
        let result = parse_tasks_internal(&l, "2026-03-21");
        // 既存パーサーはこの形式にマッチしない（時刻がある場合）
        // date_reが `YYYY-MM-DD:` のみマッチするため
        assert_eq!(result.len(), 0);
    }
}
