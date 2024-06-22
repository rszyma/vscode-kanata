/* tslint:disable */
/* eslint-disable */
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
  connection.onDocumentFormatting((...args) => kls.onDocumentFormatting(args[0]));
  connection.onDefinition((...args) => kls.onDefinition(args[0]));
  connection.languages.semanticTokens.on((...args) => kls.onSemanticTokens(args[0]));

  const retVal = kls.initialize(params);

  // This fixes a bug with either lsp-types crate or vscode-languageserver/node.
  // I'm not sure which one, I haven't dug deep into this.
  // The bug is that unlike other compound field that are Object type,
  // `capabilities.semanticTokensProvider` is suprisingly of Map type,
  // and apparently `vscode-languageserver` dislikes that and refuses to read it.
  retVal.capabilities.semanticTokensProvider = Object.fromEntries(retVal.capabilities.semanticTokensProvider);

  console.dir(retVal, { depth: 10 });
  return retVal;
});

connection.listen();
