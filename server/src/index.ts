import {
  createConnection,
  ProposedFeatures,
  PublishDiagnosticsParams,
  TextDocumentSyncKind,
  InitializeParams,
  PositionEncodingKind,
} from "vscode-languageserver/node";
import { KanataLanguageServer } from "../../out/kls";

// Create LSP connection
const connection = createConnection(ProposedFeatures.all);

const sendDiagnosticsCallback = (params: PublishDiagnosticsParams) =>
  connection.sendDiagnostics(params);

connection.onInitialize((params: InitializeParams) => {
  const kls = new KanataLanguageServer(params, sendDiagnosticsCallback);

  connection.onNotification((...args) => kls.onNotification(...args));
  connection.onDocumentFormatting((...args) =>
    kls.onDocumentFormatting(...args),
  );

  return {
    capabilities: {
      textDocumentSync: {
        openClose: true,
        save: { includeText: false },
        change: TextDocumentSyncKind.Full,
      },
      // UTF-8 is not supported in vscode-languageserver/node. See:
      // https://github.com/microsoft/vscode-languageserver-node/issues/1224
      positionEncoding: PositionEncodingKind.UTF16,
      documentFormattingProvider: true,
      workspace: {
        workspaceFolders: { supported: false },
        fileOperations: {
          didDelete: {
            filters: [{ pattern: { /* matches: 'folder', */ glob: "**" } }],
          },
        },
      },
    },
  };
});

connection.listen();
