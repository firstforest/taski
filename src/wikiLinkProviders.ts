import * as vscode from 'vscode';
import * as path from 'path';
import * as os from 'os';
import {
	parseWikiLinks,
	normalizeWikiName,
	resolveWikiLink,
	wikiLinkCreatePath,
	wikiLinkInitialContent,
} from './parser';
import { findAllMarkdownUris } from './fileScanner';

interface OpenWikiLinkArgs {
	name: string;
	fromUri: string;
}

export class WikiLinkDocumentLinkProvider implements vscode.DocumentLinkProvider {
	provideDocumentLinks(document: vscode.TextDocument): vscode.DocumentLink[] {
		const text = document.getText();
		const matches = parseWikiLinks(text);
		const links: vscode.DocumentLink[] = [];
		for (const m of matches) {
			const startPos = document.positionAt(m.start);
			const endPos = document.positionAt(m.end);
			const range = new vscode.Range(startPos, endPos);
			const args: OpenWikiLinkArgs = { name: m.name, fromUri: document.uri.toString() };
			const target = vscode.Uri.parse(
				`command:taski.openWikiLink?${encodeURIComponent(JSON.stringify(args))}`
			);
			const link = new vscode.DocumentLink(range, target);
			link.tooltip = `Open [[${m.name}]]`;
			links.push(link);
		}
		return links;
	}
}

function rankCandidate(uri: vscode.Uri): number {
	const taskiHome = path.join(os.homedir(), 'taski');
	const fs = uri.fsPath;
	if (fs.startsWith(taskiHome + path.sep) || fs === taskiHome) {
		return 0;
	}
	const workspaceFolders = vscode.workspace.workspaceFolders ?? [];
	for (const folder of workspaceFolders) {
		const fp = folder.uri.fsPath;
		if (fs.startsWith(fp + path.sep) || fs === fp) {
			return 1;
		}
	}
	const config = vscode.workspace.getConfiguration('taski');
	const additionalDirs: string[] = config.get<string[]>('additionalDirectories', []);
	for (const dir of additionalDirs) {
		if (fs.startsWith(dir + path.sep) || fs === dir) {
			return 2;
		}
	}
	return 3;
}

export async function openWikiLink(args: OpenWikiLinkArgs): Promise<void> {
	const normalized = normalizeWikiName(args.name);
	const allUris = await findAllMarkdownUris();
	const sorted = [...allUris].sort((a, b) => rankCandidate(a) - rankCandidate(b));
	const candidatePaths = sorted.map((u) => u.fsPath);
	const matched = resolveWikiLink(normalized.name, candidatePaths);

	let targetPath: string;
	if (matched) {
		targetPath = matched;
	} else {
		const taskiHome = path.join(os.homedir(), 'taski');
		targetPath = wikiLinkCreatePath(normalized.name, normalized.isJournal, taskiHome);
		const targetUri = vscode.Uri.file(targetPath);
		try {
			await vscode.workspace.fs.stat(targetUri);
		} catch {
			const dir = path.dirname(targetPath);
			await vscode.workspace.fs.createDirectory(vscode.Uri.file(dir));
			const content = wikiLinkInitialContent(normalized.name);
			await vscode.workspace.fs.writeFile(targetUri, new TextEncoder().encode(content));
		}
	}

	const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(targetPath));
	await vscode.window.showTextDocument(doc);
}
