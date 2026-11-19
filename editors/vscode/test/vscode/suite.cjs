// Runs inside the extension host of the VS Code under test. It does what the driver outside asks,
// over a local port: open a file, run a command, read the Problems panel, end.
const http = require("node:http");
const vscode = require("vscode");

async function answer(asked) {
  if (asked.open) {
    const document = await vscode.workspace.openTextDocument(asked.open);
    await vscode.window.showTextDocument(document, vscode.ViewColumn.One);
    return {};
  }
  if (asked.diagnostics) {
    const found = vscode.languages.getDiagnostics(vscode.Uri.file(asked.diagnostics));
    return { diagnostics: found.map((held) => ({ line: held.range.start.line + 1, severity: held.severity, message: held.message, source: held.source, code: held.code })) };
  }
  if (asked.lenses) {
    return { lenses: await vscode.commands.executeCommand("vscode.executeCodeLensProvider", vscode.Uri.file(asked.lenses)) };
  }
  if (asked.settings) {
    for (const [name, value] of Object.entries(asked.settings)) await vscode.workspace.getConfiguration().update(name, value, vscode.ConfigurationTarget.Global);
    return {};
  }
  if (asked.editor) {
    const editor = vscode.window.activeTextEditor;
    return { file: editor?.document.uri.fsPath, line: editor ? editor.selection.active.line + 1 : undefined, visible: vscode.window.visibleTextEditors.map((held) => held.document.uri.fsPath) };
  }
  return { result: (await vscode.commands.executeCommand(asked.command, ...(asked.args ?? []))) ?? null };
}

exports.run = () =>
  new Promise((resolve) => {
    const server = http.createServer((request, response) => {
      let body = "";
      request.on("data", (piece) => (body += piece));
      request.on("end", async () => {
        try {
          const asked = JSON.parse(body || "{}");
          if (asked.end) {
            response.end("{}");
            server.close();
            resolve();
            return;
          }
          response.end(JSON.stringify(await answer(asked)));
        } catch (error) {
          response.statusCode = 500;
          response.end(JSON.stringify({ error: String(error) }));
        }
      });
    });
    server.listen(Number(process.env.MICROFLUIDICS_TEST_PORT), "127.0.0.1");
  });
