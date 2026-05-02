import * as vscode from 'vscode';
import * as path from 'path';
import { findAllMarkdownUrisCached } from './fileScanner';
import { rankCandidate } from './wikiLinkProviders';

export class WikiLinkCompletionProvider implements vscode.CompletionItemProvider {
	async provideCompletionItems(
		document: vscode.TextDocument,
		position: vscode.Position
	): Promise<vscode.CompletionItem[] | undefined> {
		if (document.languageId !== 'markdown') {
			return;
		}

		const lineText = document.lineAt(position.line).text;
		const head = lineText.slice(0, position.character);
		const openIdx = head.lastIndexOf('[[');
		if (openIdx < 0) {
			return;
		}
		const inside = head.slice(openIdx + 2);
		if (/[\]\s]/.test(inside)) {
			return;
		}

		const replaceRange = new vscode.Range(
			position.line, openIdx + 2,
			position.line, position.character
		);

		const allUris = await findAllMarkdownUrisCached();
		const bestByStem = new Map<string, vscode.Uri>();
		for (const uri of allUris) {
			const stem = path.basename(uri.fsPath, '.md');
			const cur = bestByStem.get(stem);
			if (!cur || rankCandidate(uri) < rankCandidate(cur)) {
				bestByStem.set(stem, uri);
			}
		}

		return Array.from(bestByStem.entries()).map(([stem, uri]) => {
			const item = new vscode.CompletionItem(stem, vscode.CompletionItemKind.File);
			item.insertText = stem;
			item.range = replaceRange;
			item.detail = path.basename(path.dirname(uri.fsPath));
			item.filterText = stem;
			item.sortText = `${rankCandidate(uri)}_${stem}`;
			return item;
		});
	}
}
