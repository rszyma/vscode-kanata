/* eslint-disable @typescript-eslint/restrict-template-expressions */

import { join } from 'path';

import {
  ExtensionContext,
  RelativePattern,
  TextDocument,
  Uri,
  window,
  workspace,
  WorkspaceFolder,
  WorkspaceFoldersChangeEvent,
} from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  TransportKind,
} from 'vscode-languageclient/node';

const extensionName = 'Kanata Configuration Language';
const outputChannel = window.createOutputChannel(extensionName);

let client: LanguageClient;

function kanataFilesInFolderPattern(folder: Uri) {
  return new RelativePattern(folder, '**/*.kbd');
}

async function openKanataFilesInFolder(folder: Uri) {
  const pattern = kanataFilesInFolderPattern(folder);
  const uris = await workspace.findFiles(pattern);
  return Promise.all(uris.map(openDocument));
}

async function openDocument(uri: Uri) {
  const uriMatch = (d: TextDocument) => d.uri.toString() === uri.toString();
  const doc = workspace.textDocuments.find(uriMatch);
  if (doc === undefined) await workspace.openTextDocument(uri);
  return uri;
}

// Sets global client.
async function setClient(folder: WorkspaceFolder, ctx: ExtensionContext) {
  const server = ctx.asAbsolutePath(join('out', 'server.js'));

  const kanataFilesIncluded: Set<string> = new Set();

  const root: Uri = folder.uri;
  const deleteWatcher = workspace.createFileSystemWatcher(
    kanataFilesInFolderPattern(root),
    true, // ignoreCreateEvents
    true, // ignoreChangeEvents
    false // ignoreDeleteEvents
  );
  const createChangeWatcher = workspace.createFileSystemWatcher(
    kanataFilesInFolderPattern(root),
    false, // ignoreCreateEvents
    false, // ignoreChangeEvents
    true // ignoreDeleteEvents
  );

  // Clean up watchers when extension is deactivated.
  ctx.subscriptions.push(deleteWatcher);
  ctx.subscriptions.push(createChangeWatcher);

  const serverOpts = { module: server, transport: TransportKind.ipc };
  const clientOpts: LanguageClientOptions = {
    documentSelector: [
      { language: 'kanata', pattern: `${root.fsPath}/**/*.kbd` },
    ],
    synchronize: { fileEvents: deleteWatcher },
    diagnosticCollectionName: extensionName,
    workspaceFolder: folder,
    outputChannel,
    initializationOptions: {
      mainConfigFile: workspace
        .getConfiguration()
        .get<string>('vscode-kanata.mainConfigFile', ''),
      includesAndWorkspaces: workspace
        .getConfiguration()
        .get<string>('vscode-kanata.includesAndWorkspaces', ''),
    },
  };

  // initializationOptions: {
  //   positionEncoding: PositionEncodingKind.UTF8,
  //   // You can include any other initialization options here if needed.
  // },

  // set global client
  client = new LanguageClient(extensionName, serverOpts, clientOpts);

  // Start client and mark it for cleanup when the extension is deactivated.
  ctx.subscriptions.push(client.start());

  // [didOpen]: https://code.visualstudio.com/api/references/vscode-api#workspace.onDidOpenTextDocument
  // [didChange]: https://code.visualstudio.com/api/references/vscode-api#workspace.onDidChangeTextDocument
  ctx.subscriptions.push(createChangeWatcher.onDidCreate(openDocument));
  ctx.subscriptions.push(createChangeWatcher.onDidChange(openDocument));

  const openedFiles = await openKanataFilesInFolder(root);
  openedFiles.forEach(f => kanataFilesIncluded.add(f.toString()));

  // Ensure that all Kanata files across the workspace were included.
  (await openKanataFilesInFolder(folder.uri)).forEach(file => {
    if (!kanataFilesIncluded.has(file.toString()))
      outputChannel.appendLine(`[kls] Kanata file not included: ${file}`);
  });
}

async function stopClient() {
  // Clear any outstanding diagnostics.
  client.diagnostics?.clear();
  // Try flushing latest event in case one's in the chamber.
  return await client.stop();
}

// Support only 1 opened workspace at a time.
function updateClients(context: ExtensionContext) {
  return async function ({ added, removed }: WorkspaceFoldersChangeEvent) {
    // Clean up clients for removed folders.
    if (removed.length > 0) await stopClient();

    // Create clients for added folders.
    for (const folder of added) await setClient(folder, context);
  };
}

export async function activate(context: ExtensionContext): Promise<void> {
  const folders = workspace.workspaceFolders || [];

  // Start clients for every folder in the workspace.
  for (const folder of folders) await setClient(folder, context);

  // Update clients when workspace folders change.
  workspace.onDidChangeWorkspaceFolders(updateClients(context));
}

export async function deactivate(): Promise<void> {
  await stopClient();
}
