import * as vscode from 'vscode';
import * as path from 'path';
import { parseTasksAllDates, ParsedTaskWithDate } from './extension';
import { findAllMarkdownUris } from './fileScanner';
import { extractTags } from './tagUtils';

type TagNodeType = 'tag' | 'file' | 'task';

interface TagTaskData {
	isCompleted: boolean;
	text: string;
	fileUri: string;
	line: number;
}

interface TagFileGroup {
	fileName: string;
	fileUri: string;
	tasks: TagTaskData[];
}

export class TagTreeItem extends vscode.TreeItem {
	constructor(
		public readonly nodeType: TagNodeType,
		public readonly label: string,
		public readonly collapsibleState: vscode.TreeItemCollapsibleState,
		public readonly tagName?: string,
		public readonly fileGroup?: TagFileGroup,
		public readonly task?: TagTaskData
	) {
		super(label, collapsibleState);
		this.contextValue = nodeType;

		if (nodeType === 'tag') {
			this.iconPath = new vscode.ThemeIcon('tag', new vscode.ThemeColor('charts.blue'));
		} else if (nodeType === 'file') {
			this.iconPath = new vscode.ThemeIcon('file-text', new vscode.ThemeColor('charts.blue'));
		} else if (nodeType === 'task' && task) {
			if (task.isCompleted) {
				this.iconPath = new vscode.ThemeIcon('pass', new vscode.ThemeColor('charts.green'));
			} else {
				this.iconPath = new vscode.ThemeIcon('circle-outline', new vscode.ThemeColor('charts.yellow'));
			}
			this.command = {
				command: 'taski.openTaskLocation',
				title: 'タスクの場所を開く',
				arguments: [task.fileUri, task.line]
			};
		}
	}
}

export class TagTreeProvider implements vscode.TreeDataProvider<TagTreeItem> {
	private _onDidChangeTreeData = new vscode.EventEmitter<TagTreeItem | undefined | null | void>();
	readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

	// タグ名 → ファイルグループの配列
	private tagMap: Map<string, TagFileGroup[]> = new Map();

	refresh(): void {
		this._onDidChangeTreeData.fire();
	}

	async getChildren(element?: TagTreeItem): Promise<TagTreeItem[]> {
		if (!element) {
			await this.collectAllTaggedTasks();
			return this.getTagNodes();
		}

		if (element.nodeType === 'tag' && element.tagName !== undefined) {
			return this.getFileNodes(element.tagName);
		}

		if (element.nodeType === 'file' && element.fileGroup) {
			return this.getTaskNodes(element.fileGroup);
		}

		return [];
	}

	getTreeItem(element: TagTreeItem): vscode.TreeItem {
		return element;
	}

	private getTagNodes(): TagTreeItem[] {
		const sortedTags = [...this.tagMap.keys()].sort();
		return sortedTags.map(tag => {
			let taskCount = 0;
			for (const group of this.tagMap.get(tag)!) {
				taskCount += group.tasks.length;
			}
			return new TagTreeItem(
				'tag',
				`#${tag} (${taskCount})`,
				vscode.TreeItemCollapsibleState.Collapsed,
				tag
			);
		});
	}

	private getFileNodes(tagName: string): TagTreeItem[] {
		const groups = this.tagMap.get(tagName);
		if (!groups) {
			return [];
		}

		return groups.map(group => new TagTreeItem(
			'file',
			group.fileName,
			vscode.TreeItemCollapsibleState.Expanded,
			tagName,
			group
		));
	}

	private getTaskNodes(fileGroup: TagFileGroup): TagTreeItem[] {
		// 未完了優先でソート
		const sorted = [...fileGroup.tasks].sort((a, b) => Number(a.isCompleted) - Number(b.isCompleted));

		return sorted.map(task => new TagTreeItem(
			'task',
			task.text,
			vscode.TreeItemCollapsibleState.None,
			undefined,
			undefined,
			task
		));
	}

	private async collectAllTaggedTasks(): Promise<void> {
		const allFileUris = await findAllMarkdownUris();
		this.tagMap = new Map();

		for (const fileUri of allFileUris) {
			const doc = await vscode.workspace.openTextDocument(fileUri);
			const lines: string[] = [];
			for (let i = 0; i < doc.lineCount; i++) {
				lines.push(doc.lineAt(i).text);
			}
			const tasksInFile = parseTasksAllDates(lines);

			const relativePath = vscode.workspace.asRelativePath(fileUri);
			const fileName = path.basename(relativePath);

			// タスクテキストの重複を除外するためにユニークなタスクを特定
			// parseTasksAllDates はログごとにエントリを返すため、同じタスクが複数回出現する
			const seenTasks = new Map<number, ParsedTaskWithDate>();
			for (const t of tasksInFile) {
				if (!seenTasks.has(t.line)) {
					seenTasks.set(t.line, t);
				}
			}

			for (const task of seenTasks.values()) {
				const tags = extractTags(task.text);
				if (tags.length === 0) {
					continue;
				}

				const taskData: TagTaskData = {
					isCompleted: task.isCompleted,
					text: task.text,
					fileUri: fileUri.toString(),
					line: task.line
				};

				for (const tag of tags) {
					let groups = this.tagMap.get(tag);
					if (!groups) {
						groups = [];
						this.tagMap.set(tag, groups);
					}

					// 同じファイルのグループを探す
					let fileGroup = groups.find(g => g.fileUri === fileUri.toString());
					if (!fileGroup) {
						fileGroup = {
							fileName,
							fileUri: fileUri.toString(),
							tasks: []
						};
						groups.push(fileGroup);
					}

					fileGroup.tasks.push(taskData);
				}
			}
		}
	}
}
