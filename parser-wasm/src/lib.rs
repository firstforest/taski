use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// === Internal types ===

struct CurrentTask {
    indent: usize,
    completed: bool,
    text: String,
    line: usize,
}

// === Existing WASM output types ===

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ParsedTask {
    is_completed: bool,
    text: String,
    line: usize,
    log: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ParsedTaskWithDate {
    is_completed: bool,
    text: String,
    line: usize,
    log: String,
    date: String,
}

// === New types for build_tree_data ===

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileInput {
    file_name: String,
    file_uri: String,
    lines: Vec<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TreeTaskData {
    is_completed: bool,
    text: String,
    file_uri: String,
    line: usize,
    log: String,
    date: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TreeFileGroup {
    file_name: String,
    file_uri: String,
    tasks: Vec<TreeTaskData>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TreeDateGroup {
    date_key: String,
    label: String,
    is_today: bool,
    file_groups: Vec<TreeFileGroup>,
    completed_count: usize,
    total_count: usize,
}

// === Internal parsing logic ===

fn parse_tasks_internal(lines: &[String], target_date: &str) -> Vec<ParsedTask> {
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

fn parse_all_dates_internal(lines: &[String]) -> Vec<ParsedTaskWithDate> {
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

// === build_tree_data internal logic ===

fn build_tree_data_internal(files: Vec<FileInput>, today_str: &str) -> Vec<TreeDateGroup> {
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

// === WASM exports ===

#[wasm_bindgen(js_name = "parseTasks")]
pub fn parse_tasks(lines_js: JsValue, target_date: &str) -> JsValue {
    let lines: Vec<String> = serde_wasm_bindgen::from_value(lines_js).unwrap_or_default();
    let tasks = parse_tasks_internal(&lines, target_date);
    serde_wasm_bindgen::to_value(&tasks).unwrap()
}

#[wasm_bindgen(js_name = "parseTasksAllDates")]
pub fn parse_tasks_all_dates(lines_js: JsValue) -> JsValue {
    let lines: Vec<String> = serde_wasm_bindgen::from_value(lines_js).unwrap_or_default();
    let tasks = parse_all_dates_internal(&lines);
    serde_wasm_bindgen::to_value(&tasks).unwrap()
}

#[wasm_bindgen(js_name = "buildTreeData")]
pub fn build_tree_data(files_js: JsValue, today_str: &str) -> JsValue {
    let files: Vec<FileInput> = serde_wasm_bindgen::from_value(files_js).unwrap_or_default();
    let result = build_tree_data_internal(files, today_str);
    serde_wasm_bindgen::to_value(&result).unwrap()
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
}
