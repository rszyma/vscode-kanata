import {
  createConnection,
  ProposedFeatures,
  PublishDiagnosticsParams,
  InitializeParams,
} from "vscode-languageserver/node";
import { KanataLanguageServer } from "../../out/kls";
import { Console } from "console";

// Redirect all console stdout output to stderr since LSP pipe uses stdout
// and writing to stdout for anything other than LSP protocol will break
// things badly.
global.console = new Console(process.stderr, process.stderr);

// Create LSP connection
const connection = createConnection(ProposedFeatures.all);

connection.onInitialize((params: InitializeParams) => {
  const kls = new KanataLanguageServer(
    params,
    (params: PublishDiagnosticsParams) => connection.sendDiagnostics(params),
  );

  connection.onNotification((...args) => kls.onNotification(...args));

  connection.onDocumentFormatting((...args) =>
    // eslint-disable-next-line @typescript-eslint/no-unsafe-return
    kls.onDocumentFormatting(args[0]),
  );

  // eslint-disable-next-line @typescript-eslint/no-unsafe-return
  connection.onDefinition((...args) => kls.onDefinition(args[0]));

  // connection.languages.semanticTokens.on((...args) =>
  //   // eslint-disable-next-line @typescript-eslint/no-unsafe-return, @typescript-eslint/no-unsafe-call
  //   kls.onSemanticTokens(args[0]),
  // );

  // eslint-disable-next-line @typescript-eslint/no-unsafe-return
  connection.onHover((...args) => kls.onHover(args[0]));

  connection.onPrepareRename((...args) => kls.onPrepareRenameRequest(args[0]));
  connection.onRenameRequest((...args) => kls.onRenameRequest(args[0]));

  // eslint-disable-next-line @typescript-eslint/no-unsafe-return, @typescript-eslint/no-unsafe-call
  return kls.initialize(params);
});

connection.listen();
