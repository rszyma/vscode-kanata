/* eslint-disable @typescript-eslint/restrict-template-expressions */

import { platform } from "os";

import { join } from "path";

import {
  ExtensionContext,
  RelativePattern,
  Uri,
  window,
  workspace,
  WorkspaceFolder,
  Disposable,
  ConfigurationChangeEvent,
  FileSystemWatcher,
  commands,
  ConfigurationTarget,
} from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  SettingMonitor,
  TransportKind,
  MessageActionItem,
  DocumentSelector,
} from "vscode-languageclient/node";

const extensionName = "Kanata Configuration Language";
const outputChannel = window.createOutputChannel(extensionName);

const docSelector: DocumentSelector = [
  {
    scheme: "file",
    language: "kanata",
    pattern: "**/*.kbd",
  },
];

// const defProvider: DefinitionProvider = new Provider(context);

// global extension instance
let ext: Extension;

export async function activate(ctx: ExtensionContext): Promise<void> {
  const cmd1 = commands.registerCommand(
    "vscode-kanata.setSetCurrentFileAsMain",
    async () => {
      const editor = window.activeTextEditor;
      if (editor) {
        const fileName = editor.document.fileName;
        const cfg = workspace.getConfiguration();
        await cfg.update(
          "vscode-kanata.mainConfigFile",
          fileName,
          ConfigurationTarget.Workspace,
        );
      } else {
        await window.showErrorMessage("No active editor");
      }
    },
  );
  ctx.subscriptions.push(cmd1);

  ext = new Extension(ctx);
  await ext.start();
  ctx.subscriptions.push(ext);
}

export async function deactivate(): Promise<void> {
  await ext.stop();
}

class Extension implements Disposable {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  toDisposeOnRestart: { dispose(): any }[];
  ctx: ExtensionContext;
  client: LanguageClient | undefined;
  settingMonitor: SettingMonitor | undefined;

  constructor(ctx: ExtensionContext) {
    this.toDisposeOnRestart = [];
    this.ctx = ctx;

    this.ctx.subscriptions.push(
      workspace.onDidChangeConfiguration(this.restart()),
    );

    // this.ctx.subscriptions.push(
    //   languages.registerDefinitionProvider(docSelector, defProvider),
    // );
  }

  async start() {
    const openedWorkspaces = workspace.workspaceFolders;
    let root: WorkspaceFolder | undefined;
    if (openedWorkspaces) {
      if (openedWorkspaces.length >= 2) {
        await window.showInformationMessage(
          "Multiple workspaces are currently not supported, only the first workspaces folder will be regarded.",
        );
      }
      root = openedWorkspaces.at(0);
    }

    outputChannel.appendLine(
      `starting with ${openedWorkspaces?.length || 0} opened workspaces`,
    );

    await this.startClient(root);
  }

  async stop() {
    // Clear any outstanding diagnostics.
    this.client?.diagnostics?.clear();
    // Try flushing latest event in case one's in the chamber.
    await this.client?.stop();
  }

  async startClient(root: WorkspaceFolder | undefined) {
    const serverModulePath = this.ctx.asAbsolutePath(join("out", "server.js"));

    let deleteWatcher: FileSystemWatcher | undefined = undefined;
    let changeWatcher: FileSystemWatcher | undefined = undefined;

    if (root !== undefined) {
      deleteWatcher = workspace.createFileSystemWatcher(
        kanataFilesInFolderPattern(root.uri),
        true, // ignoreCreateEvents
        true, // ignoreChangeEvents
        false, // ignoreDeleteEvents
      );
      changeWatcher = workspace.createFileSystemWatcher(
        kanataFilesInFolderPattern(root.uri),
        false, // ignoreCreateEvents
        false, // ignoreChangeEvents
        true, // ignoreDeleteEvents
      );

      // Clean up watchers when extension is deactivated.
      this.toDisposeOnRestart.push(deleteWatcher);
      this.toDisposeOnRestart.push(changeWatcher);
      // [didOpen]: https://code.visualstudio.com/api/references/vscode-api#workspace.onDidOpenTextDocument
      this.toDisposeOnRestart.push(changeWatcher.onDidCreate(openDocument));
      // [didChange]: https://code.visualstudio.com/api/references/vscode-api#workspace.onDidChangeTextDocument
      this.toDisposeOnRestart.push(changeWatcher.onDidChange(openDocument));
    }

    const serverOpts: ServerOptions = {
      module: serverModulePath,
      transport: TransportKind.ipc,
    };

    const clientOpts: LanguageClientOptions = {
      documentSelector: docSelector,
      synchronize: { fileEvents: deleteWatcher },
      diagnosticCollectionName: extensionName,
      workspaceFolder: root,
      outputChannel,
      initializationOptions: {
        mainConfigFile: workspace
          .getConfiguration()
          .get<string>("vscode-kanata.mainConfigFile", ""),
        includesAndWorkspaces: workspace
          .getConfiguration()
          .get<string>("vscode-kanata.includesAndWorkspaces", ""),
        localKeysVariant: getLocalKeysVariant() as string,
        format: getFormatterSettings(),
        envVariables: workspace.getConfiguration().get<{
          [id: string]: string;
        }>("vscode-kanata.environmentVariables", {}),
        dimInactiveConfigItems: workspace
          .getConfiguration()
          .get<boolean>("vscode-kanata.dimInactiveConfigItems", true),
      },
    };

    this.client = new LanguageClient(extensionName, serverOpts, clientOpts);

    await this.client.start();

    this.toDisposeOnRestart.push(this.client);

    if (root !== undefined) {
      await openKanataFilesInFolder(root.uri);
    } else {
      // When file is opened in non-workspace mode, vscode will automatically
      // call textDocument/didOpen, so no need to do anything here.
    }
  }

  restart() {
    return async (e: ConfigurationChangeEvent) => {
      if (e.affectsConfiguration("vscode-kanata")) {
        outputChannel.appendLine("vscode-kanata configuration has changed!");
        await this.stop();
        this.dispose();
        outputChannel.clear();
        await this.start();
      } else {
        outputChannel.appendLine(
          "Settings changed but vscode-kanata configuration hasn't changed",
        );
      }
    };
  }

  dispose() {
    this.toDisposeOnRestart.forEach((disposable) => {
      disposable.dispose();
    });
  }
}

function kanataFilesInFolderPattern(folder: Uri) {
  return new RelativePattern(folder, "**/*.kbd");
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

async function showLocalkeysManualInterventionNeeded() {
  const message =
    "Cannot select `deflocalkeys` variant automatically. Please go to extension settings and select it manually.";
  const openSettingsAction: MessageActionItem = { title: "Open Settings" };

  await window
    .showInformationMessage(message, openSettingsAction)
    .then(async (selectedAction) => {
      if (selectedAction === openSettingsAction) {
        await commands.executeCommand(
          "workbench.action.openSettings",
          "vscode-kanata.localKeysVariant",
        );
      }
    });
}

type LocalKeysVariant =
  | "deflocalkeys-win"
  | "deflocalkeys-wintercept"
  | "deflocalkeys-linux"
  | "deflocalkeys-macos"
  | "deflocalkeys-winiov2";

// Gets localkeys variant from config and when set to auto, detects it based on current OS.
function getLocalKeysVariant(): LocalKeysVariant {
  const localKeysVariant = workspace
    .getConfiguration()
    .get<string>("vscode-kanata.localKeysVariant", "");

  if (localKeysVariant == "auto") {
    switch (platform()) {
      case "linux":
        return "deflocalkeys-linux";
      case "darwin":
        return "deflocalkeys-macos";
      default: // Catches both unsupported systems as well as windows, since there are 3 possible variants for windows.
        showLocalkeysManualInterventionNeeded()
          .then(null)
          .catch((e) => {
            outputChannel.appendLine(`error: ${e}`);
          });
        // Use 'deflocalkeys-win' as a fallback, since that's the most common variant, I guess.
        return "deflocalkeys-win";
    }
  }

  return localKeysVariant as LocalKeysVariant;
}

interface FormatterSettings {
  enable: boolean;
  useDefsrcLayoutOnDeflayers: boolean;
}

function getFormatterSettings(): FormatterSettings {
  const formatSettings = workspace
    .getConfiguration()
    .get<FormatterSettings>("vscode-kanata.format");

  if (formatSettings === undefined) {
    throw new Error("should be defined");
  }

  console.log("formatSettings:", formatSettings);

  return formatSettings;
}
