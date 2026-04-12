use wasm_bindgen::prelude::*;

pub use parser_core::{
    FileInput, ParsedTask, ParsedTaskWithDate, ScheduleEntry, TreeDateGroup, TreeFileGroup,
    TreeTaskData,
};

// === WASM exports ===

#[wasm_bindgen(js_name = "parseTasks")]
pub fn parse_tasks(lines_js: JsValue, target_date: &str) -> JsValue {
    let lines: Vec<String> = serde_wasm_bindgen::from_value(lines_js).unwrap_or_default();
    let tasks = parser_core::parse_tasks_internal(&lines, target_date);
    serde_wasm_bindgen::to_value(&tasks).unwrap()
}

#[wasm_bindgen(js_name = "parseTasksAllDates")]
pub fn parse_tasks_all_dates(lines_js: JsValue) -> JsValue {
    let lines: Vec<String> = serde_wasm_bindgen::from_value(lines_js).unwrap_or_default();
    let tasks = parser_core::parse_all_dates_internal(&lines);
    serde_wasm_bindgen::to_value(&tasks).unwrap()
}

#[wasm_bindgen(js_name = "buildTreeData")]
pub fn build_tree_data(files_js: JsValue, today_str: &str) -> JsValue {
    let files: Vec<FileInput> = serde_wasm_bindgen::from_value(files_js).unwrap_or_default();
    let result = parser_core::build_tree_data_internal(files, today_str);
    serde_wasm_bindgen::to_value(&result).unwrap()
}

#[wasm_bindgen(js_name = "buildScheduleData")]
pub fn build_schedule_data(files_js: JsValue, target_date: &str) -> JsValue {
    let files: Vec<FileInput> = serde_wasm_bindgen::from_value(files_js).unwrap_or_default();
    let result = parser_core::build_schedule_data_internal(files, target_date);
    serde_wasm_bindgen::to_value(&result).unwrap()
}

#[wasm_bindgen(js_name = "extractTags")]
pub fn extract_tags(text: &str) -> JsValue {
    let tags = parser_core::extract_tags(text);
    serde_wasm_bindgen::to_value(&tags).unwrap()
}

#[wasm_bindgen(js_name = "extractFileTags")]
pub fn extract_file_tags(lines_js: JsValue) -> JsValue {
    let lines: Vec<String> = serde_wasm_bindgen::from_value(lines_js).unwrap_or_default();
    let tags = parser_core::extract_file_tags(&lines);
    serde_wasm_bindgen::to_value(&tags).unwrap()
}
