import {
	parseTasks as wasmParseTasks,
	parseTasksAllDates as wasmParseTasksAllDates,
	buildTreeData as wasmBuildTreeData,
	buildScheduleData as wasmBuildScheduleData,
	extractTags as wasmExtractTags,
} from './pkg/parser_wasm';
import type { ParsedTask, ParsedTaskWithDate } from './extension';

export interface FileInput {
	fileName: string;
	fileUri: string;
	lines: string[];
}

export interface TreeTaskData {
	isCompleted: boolean;
	text: string;
	fileUri: string;
	line: number;
	log: string;
	date: string;
	context: string[];
}

export interface TreeFileGroup {
	fileName: string;
	fileUri: string;
	tasks: TreeTaskData[];
}

export interface TreeDateGroup {
	dateKey: string;
	label: string;
	isToday: boolean;
	fileGroups: TreeFileGroup[];
	completedCount: number;
	totalCount: number;
}

export function parseTasks(lines: string[], targetDate: string): ParsedTask[] {
	return wasmParseTasks(lines, targetDate) as ParsedTask[];
}

export function parseTasksAllDates(lines: string[]): ParsedTaskWithDate[] {
	return wasmParseTasksAllDates(lines) as ParsedTaskWithDate[];
}

export function buildTreeData(files: FileInput[], todayStr: string): TreeDateGroup[] {
	return wasmBuildTreeData(files, todayStr) as TreeDateGroup[];
}

export interface ScheduleEntry {
	taskText: string;
	taskLine: number;
	isCompleted: boolean;
	logText: string;
	logLine: number;
	time: string;
	endTime: string;
	fileUri: string;
}

export function buildScheduleData(files: FileInput[], targetDate: string): ScheduleEntry[] {
	return wasmBuildScheduleData(files, targetDate) as ScheduleEntry[];
}

export function extractTags(text: string): string[] {
	return wasmExtractTags(text) as string[];
}
