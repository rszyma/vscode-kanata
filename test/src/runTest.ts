import { resolve } from "path";

import { runTests } from "@vscode/test-electron";
import { minVersion } from "semver";

import { engines } from "../../package.json";

void (async function () {
  try {
    // Fetch the semver constraint from the 'engines' field in the extension's
    // package.json and test against the minimum satisfiable version.
    const minSupportedVSCodeVersion =
      minVersion(engines.vscode)?.toString() || engines.vscode;

    const extensionDevelopmentPath = resolve(__dirname, "../../..");
    const extensionTestsPath = resolve(__dirname, "./suite");
    const workspace = resolve(
      __dirname,
      "../../../test-fixtures/workspace/test.code-workspace",
    );

    await runTests({
      version: minSupportedVSCodeVersion,
      extensionDevelopmentPath,
      extensionTestsPath,
      launchArgs: [workspace, "--disable-extensions", "--disable-telemetry"],
    });
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
  } catch (e: any) {
    console.error(`====== ERROR ======\n${e}\n==================`);
    process.exitCode = 1;
  }
})();
