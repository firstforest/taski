import * as vscode from 'vscode';
import * as path from 'path';
import { findAllMarkdownUris } from './fileScanner';
import { buildTreeData } from './extension';
import type { TreeTaskData, TreeFileGroup, TreeDateGroup, FileInput } from './extension';

// TreeViewのノード種別
type TreeNodeType = 'date' | 'file' | 'task' | 'log';

// TreeItemとして表示するノード
export class TaskTreeItem extends vscode.TreeItem {
	constructor(
		public readonly nodeType: TreeNodeType,
		public readonly label: string,
		public readonly collapsibleState: vscode.TreeItemCollapsibleState,
		public readonly dateKey?: string,
		public readonly fileGroup?: TreeFileGroup,
		public readonly task?: TreeTaskData,
		public readonly isToday?: boolean
	) {
		super(label, collapsibleState);
		this.contextValue = nodeType;

		if (nodeType === 'date') {
			// 今日: 緑、過去: オレンジ、日付なし: グレー
			if (isToday) {
				this.iconPath = new vscode.ThemeIcon('calendar', new vscode.ThemeColor('charts.green'));
			} else if (dateKey === '') {
				this.iconPath = new vscode.ThemeIcon('calendar', new vscode.ThemeColor('disabledForeground'));
			} else {
				this.iconPath = new vscode.ThemeIcon('calendar', new vscode.ThemeColor('charts.orange'));
			}
		} else if (nodeType === 'file') {
			this.iconPath = new vscode.ThemeIcon('file-text', new vscode.ThemeColor('charts.blue'));
		} else if (nodeType === 'task' && task) {
			// 完了: 緑、未完了: 黄色
			if (task.isCompleted) {
				this.iconPath = new vscode.ThemeIcon('pass', new vscode.ThemeColor('charts.green'));
			} else {
				this.iconPath = new vscode.ThemeIcon('circle-outline', new vscode.ThemeColor('charts.yellow'));
			}
			// タスクをクリックしたらファイルを開く
			this.command = {
				command: 'taski.openTaskLocation',
				title: 'タスクの場所を開く',
				arguments: [task.fileUri, task.line]
			};
		} else if (nodeType === 'log') {
			this.iconPath = new vscode.ThemeIcon('note', new vscode.ThemeColor('charts.purple'));
		}
	}
}

export class TaskTreeProvider implements vscode.TreeDataProvider<TaskTreeItem> {
	private _onDidChangeTreeData = new vscode.EventEmitter<TaskTreeItem | undefined | null | void>();
	readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

	private treeData: TreeDateGroup[] = [];
	private todayStr: string = '';

	constructor() {
		this.todayStr = this.getLocalDateString();
	}

	refresh(): void {
		this.todayStr = this.getLocalDateString();
		this._onDidChangeTreeData.fire();
	}

	private getLocalDateString(): string {
		const d = new Date();
		const year = d.getFullYear();
		const month = String(d.getMonth() + 1).padStart(2, '0');
		const day = String(d.getDate()).padStart(2, '0');
		return `${year}-${month}-${day}`;
	}

	async getChildren(element?: TaskTreeItem): Promise<TaskTreeItem[]> {
		if (!element) {
			// ルートレベル: 日付ノードを返す
			await this.collectAllTasks();
			return this.getDateNodes();
		}

		if (element.nodeType === 'date' && element.dateKey !== undefined) {
			// 日付ノードの子: ファイルノードを返す
			return this.getFileNodes(element.dateKey);
		}

		if (element.nodeType === 'file' && element.fileGroup) {
			// ファイルノードの子: タスクノードを返す
			return this.getTaskNodes(element.fileGroup);
		}

		if (element.nodeType === 'task' && element.task?.log) {
			// タスクノードの子: ログノードを返す
			return [new TaskTreeItem(
				'log',
				element.task.log,
				vscode.TreeItemCollapsibleState.None
			)];
		}

		return [];
	}

	getTreeItem(element: TaskTreeItem): vscode.TreeItem {
		return element;
	}

	private getDateNodes(): TaskTreeItem[] {
		return this.treeData.map(group => new TaskTreeItem(
			'date',
			group.label,
			group.isToday ? vscode.TreeItemCollapsibleState.Expanded : vscode.TreeItemCollapsibleState.Collapsed,
			group.dateKey,
			undefined,
			undefined,
			group.isToday
		));
	}

	private getFileNodes(dateKey: string): TaskTreeItem[] {
		const dateGroup = this.treeData.find(g => g.dateKey === dateKey);
		if (!dateGroup) {
			return [];
		}

		return dateGroup.fileGroups.map(group => new TaskTreeItem(
			'file',
			group.fileName,
			vscode.TreeItemCollapsibleState.Expanded,
			dateKey,
			group
		));
	}

	private getTaskNodes(fileGroup: TreeFileGroup): TaskTreeItem[] {
		return fileGroup.tasks.map(task => new TaskTreeItem(
			'task',
			task.text,
			task.log ? vscode.TreeItemCollapsibleState.Expanded : vscode.TreeItemCollapsibleState.None,
			undefined,
			undefined,
			task
		));
	}

	private async collectAllTasks(): Promise<void> {
		const allFileUris = await findAllMarkdownUris();
		const files: FileInput[] = [];

		for (const fileUri of allFileUris) {
			const doc = await vscode.workspace.openTextDocument(fileUri);
			const lines: string[] = [];
			for (let i = 0; i < doc.lineCount; i++) {
				lines.push(doc.lineAt(i).text);
			}
			const relativePath = vscode.workspace.asRelativePath(fileUri);
			const fileName = path.basename(relativePath);
			files.push({
				fileName,
				fileUri: fileUri.toString(),
				lines,
			});
		}

		this.treeData = buildTreeData(files, this.todayStr);
	}

}
