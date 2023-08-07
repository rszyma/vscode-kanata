/* eslint-disable @typescript-eslint/restrict-template-expressions */

import { join } from 'path';

import {
  ExtensionContext,
  RelativePattern,
  Uri,
  window,
  workspace,
  WorkspaceFolder,
  Disposable,
  ConfigurationChangeEvent,
} from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  SettingMonitor,
  TransportKind,
} from 'vscode-languageclient/node';

const extensionName = 'Kanata Configuration Language';
const outputChannel = window.createOutputChannel(extensionName);

// global extension instance
let ext: Extension;

export async function activate(ctx: ExtensionContext): Promise<void> {
  // Update clients when workspace folders change.
  ext = new Extension(ctx);
  await ext.start();
  ctx.subscriptions.push(ext);
}

export async function deactivate(): Promise<void> {
  await ext.stop();
}

class Extension implements Disposable {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  readonly toDisposeOnReload: { dispose(): any }[];
  ctx: ExtensionContext;
  client: LanguageClient | undefined;
  settingMonitor: SettingMonitor | undefined;

  constructor(ctx: ExtensionContext) {
    this.toDisposeOnReload = [];
    this.ctx = ctx;
    // this.settingMonitor = new SettingMonitor(this.client, 'vscode-kanata');
  }

  async start() {
    const openedWorkspaces = workspace.workspaceFolders;
    let root: WorkspaceFolder | undefined;
    if (openedWorkspaces) {
      if (openedWorkspaces.length >= 2) {
        await window.showInformationMessage(
          'Multiple workspaces are currently not supported, only the first workspaces folder will be regarded.'
        );
      }
      root = openedWorkspaces.at(0);
    }

    await this.startClient(root);

    this.ctx.subscriptions.push(
      workspace.onDidChangeConfiguration(this.restart())
    );
  }

  async stop() {
    // Clear any outstanding diagnostics.
    this.client?.diagnostics?.clear();
    // Try flushing latest event in case one's in the chamber.
    await this.client?.stop();
  }

  async startClient(root: WorkspaceFolder | undefined) {
    const server = this.ctx.asAbsolutePath(join('out', 'server.js'));

    if (root === undefined) {
      console.log(
        'single files opened in non-workspace are currently not supported'
      );
      return;
    }

    const deleteWatcher = workspace.createFileSystemWatcher(
      kanataFilesInFolderPattern(root.uri),
      true, // ignoreCreateEvents
      true, // ignoreChangeEvents
      false // ignoreDeleteEvents
    );
    const changeWatcher = workspace.createFileSystemWatcher(
      kanataFilesInFolderPattern(root.uri),
      false, // ignoreCreateEvents
      false, // ignoreChangeEvents
      true // ignoreDeleteEvents
    );

    // Clean up watchers when extension is deactivated.
    this.toDisposeOnReload.push(deleteWatcher);
    this.toDisposeOnReload.push(changeWatcher);

    const serverOpts = { module: server, transport: TransportKind.ipc };
    const clientOpts: LanguageClientOptions = {
      documentSelector: [
        { language: 'kanata', pattern: `${root.uri.fsPath}/**/*.kbd` },
      ],
      synchronize: { fileEvents: deleteWatcher },
      diagnosticCollectionName: extensionName,
      workspaceFolder: root,
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

    this.client = new LanguageClient(extensionName, serverOpts, clientOpts);

    // Start client and mark it for cleanup when the extension is deactivated.
    await this.client.start();

    this.toDisposeOnReload.push(this.client);

    // [didOpen]: https://code.visualstudio.com/api/references/vscode-api#workspace.onDidOpenTextDocument
    // [didChange]: https://code.visualstudio.com/api/references/vscode-api#workspace.onDidChangeTextDocument
    this.toDisposeOnReload.push(changeWatcher.onDidCreate(openDocument));
    this.toDisposeOnReload.push(changeWatcher.onDidChange(openDocument));

    await openKanataFilesInFolder(root.uri);
  }

  restart() {
    return async (e: ConfigurationChangeEvent) => {
      if (e.affectsConfiguration('vscode-kanata')) {
        console.log('vscode-kanata configuration has changed!');
        await this.stop();
        // todo: reload configuration?
        await this.start();
      } else {
        console.log('vscode-kanata configuration has NOT changed');
      }
    };
  }

  dispose() {
    this.toDisposeOnReload.forEach(disposable => {
      disposable.dispose();
    });
  }
}

function kanataFilesInFolderPattern(folder: Uri) {
  return new RelativePattern(folder, '**/*.kbd');
}

async function openKanataFilesInFolder(folder: Uri) {
  const pattern = kanataFilesInFolderPattern(folder);
  const uris = await workspace.findFiles(pattern);
  for (const uri of uris) {
    await openDocument(uri);
  }
}

async function openDocument(uri: Uri) {
  // workspace.openTextDocument has a build-in mechanism that avoids reopening
  // already opened file, so no need to handle that manually.
  await workspace.openTextDocument(uri);
}
