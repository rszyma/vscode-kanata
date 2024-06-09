import {
  createConnection,
  ProposedFeatures,
  PublishDiagnosticsParams,
  InitializeParams,
} from "vscode-languageserver/node";
import { KanataLanguageServer } from "../../out/kls";

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
  connection.languages.semanticTokens.on((...args) =>
    // eslint-disable-next-line @typescript-eslint/no-unsafe-return, @typescript-eslint/no-unsafe-call
    kls.onSemanticTokens(args[0]),
  );
  // eslint-disable-next-line @typescript-eslint/no-unsafe-return, @typescript-eslint/no-unsafe-call
  return kls.initialize(params);
});

connection.listen();
