// The extension: a session for the design being edited, its chip in a panel beside the editor,
// its findings as diagnostics, and the two linked so that either shows the other.

import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import { existsSync } from "node:fs";
import * as path from "node:path";
import * as readline from "node:readline";
import * as vscode from "vscode";

import { designAt } from "./design";
import type { Diagnostic, FromSession, FromViewer, Scene, ToViewer } from "./scene";
import { madeOn } from "./viewer/geometry";
import { page } from "./viewer/page";

const SEVERITY: Record<Diagnostic["severity"], vscode.DiagnosticSeverity> = {
  error: vscode.DiagnosticSeverity.Error,
  warning: vscode.DiagnosticSeverity.Warning,
  information: vscode.DiagnosticSeverity.Information,
};

/** The Python a design's session runs in: the one the settings name, else the virtual environment
 * of the design's folder, else the one the Python extension is set to, else `python3`. */
function interpreter(design: vscode.Uri, folder: string | undefined): string {
  const named = vscode.workspace.getConfiguration("microfluidics", design).get<string>("python")?.trim();
  if (named) return folder && /[\\/]/.test(named) && !path.isAbsolute(named) ? path.join(folder, named) : named;
  for (const inside of [".venv/bin/python", ".venv/Scripts/python.exe"]) {
    if (folder && existsSync(path.join(folder, inside))) return path.join(folder, inside);
  }
  const chosen = vscode.workspace.getConfiguration("python", design).get<string>("defaultInterpreterPath")?.trim();
  if (chosen && path.isAbsolute(chosen) && existsSync(chosen)) return chosen;
  return "python3";
}

/** One design being previewed: its session, its panel, and what it last showed. */
class Preview {
  private session: ChildProcessWithoutNullStreams | undefined;
  private readonly panel: vscode.WebviewPanel;
  private scene: Scene | undefined;
  private last: ToViewer[] = [];
  private readonly listeners: vscode.Disposable[] = [];
  /** What the session wrote to its error stream, kept to explain a session that ends. */
  private complaint = "";
  private ended = false;
  /** The line last shown for what was selected on the chip, and the editor it is marked in. */
  private marked: vscode.TextEditor | undefined;
  /** How the design stands, as the status bar says it: being built, built with so many problems, or not building. */
  standing: { state: "building" | "built" | "failed"; problems: number } = { state: "building", problems: 0 };

  constructor(
    private readonly design: vscode.Uri,
    extension: vscode.Uri,
    private readonly diagnostics: vscode.DiagnosticCollection,
    private readonly output: vscode.OutputChannel,
    private readonly mark: vscode.TextEditorDecorationType,
    private readonly changed: () => void,
    closed: () => void,
    restored?: vscode.WebviewPanel,
  ) {
    const name = design.path.split("/").pop() ?? "chip";
    const out = vscode.Uri.joinPath(extension, "out");
    const options = { enableScripts: true, localResourceRoots: [out] };
    this.panel = restored ?? vscode.window.createWebviewPanel("microfluidics.preview", `Chip: ${name}`, { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true }, { ...options, retainContextWhenHidden: true });
    this.panel.webview.options = options;
    this.panel.iconPath = new vscode.ThemeIcon("circuit-board");
    const served = (file: string) => this.panel.webview.asWebviewUri(vscode.Uri.joinPath(out, file)).toString();
    this.panel.webview.html = page(served("viewer.js"), served("codicon.css"), this.panel.webview.cspSource, randomBytes(16).toString("base64"), design.fsPath);
    this.listeners.push(
      this.panel.webview.onDidReceiveMessage((message: FromViewer) => this.asked(message)),
      this.panel.onDidDispose(() => {
        this.end();
        closed();
      }),
      // Saving any file beside the design is noticed by the session itself; this makes it prompt.
      vscode.workspace.onDidSaveTextDocument((saved) => {
        if (saved.uri.fsPath === design.fsPath) this.ask({ type: "rebuild" });
      }),
      vscode.window.onDidChangeTextEditorSelection((event) => this.cursor(event.textEditor)),
    );
    this.start();
  }

  /** Start the session, in the Python that has the package. */
  private start(): void {
    this.said(JSON.stringify({ type: "status", state: "building" }));
    const folder = vscode.workspace.getWorkspaceFolder(this.design)?.uri.fsPath;
    const command = interpreter(this.design, folder);
    const session = spawn(command, ["-m", "microfluidics", "session", this.design.fsPath], { cwd: folder });
    this.session = session;
    const current = () => !this.ended && this.session === session;
    readline.createInterface({ input: session.stdout }).on("line", (line) => { if (current()) this.said(line); });
    session.stderr.on("data", (text: Buffer) => {
      if (!current()) return;
      this.complaint = (this.complaint + text.toString()).slice(-8000);
      this.output.append(text.toString());
    });
    session.stdin.on("error", () => undefined);
    session.on("error", (error) => {
      if (current()) this.stopped(`\`${command}\` could not be started: ${error.message}. Set "microfluidics.python" to a Python that has the microfluidics package.`);
    });
    session.on("exit", (code, signal) => {
      if (!current()) return;
      const said = this.complaint.trim();
      this.stopped(`The session ended${code === null ? ` on ${signal}` : ` with ${code}`}.${said ? `\n${said}` : ""}`);
    });
  }

  reveal(): void {
    this.panel.reveal(vscode.ViewColumn.Beside, true);
  }

  /** The session is gone: say why where the chip would be, and in the output. */
  private stopped(why: string): void {
    if (this.ended) return;
    this.output.appendLine(why);
    this.said(JSON.stringify({ type: "status", state: "failed", message: why }));
  }

  ask(message: object): void {
    if (this.ended || !this.session?.stdin.writable) return;
    this.session.stdin.write(`${JSON.stringify(message)}\n`);
  }

  private post(message: ToViewer): void {
    void this.panel.webview.postMessage(message);
  }

  /** What the session said: the scene and its status go to the viewer, findings to the editor. */
  private said(line: string): void {
    let message: FromSession;
    try {
      message = JSON.parse(line) as FromSession;
    } catch {
      this.output.appendLine(line);
      return;
    }
    if (message.type === "diagnostics") {
      this.diagnostics.clear();
      for (const [file, found] of Object.entries(message.files)) {
        this.diagnostics.set(vscode.Uri.file(file), found.map((held) => this.diagnostic(file, held)));
      }
      return;
    }
    if (message.type === "scene") {
      this.scene = message.scene;
      this.last = [];
      this.standing = { state: "built", problems: message.scene.problems };
      this.changed();
    }
    if (message.type === "status" && message.state !== "ready") {
      this.standing = { state: message.state, problems: this.standing.problems };
      this.changed();
    }
    if (message.type === "result") {
      this.output.appendLine(message.text);
      if (!message.ok) this.output.show(true);
    }
    this.last.push(message);
    this.post(message);
  }

  private diagnostic(file: string, held: Diagnostic): vscode.Diagnostic {
    const line = Math.max(held.line - 1, 0);
    const text = vscode.workspace.textDocuments.find((open) => open.uri.fsPath === file)?.lineAt(Math.min(line, 1e9))?.text ?? "";
    const range = new vscode.Range(line, text.length - text.trimStart().length, line, Math.max(text.length, 1));
    const made = new vscode.Diagnostic(range, held.message, SEVERITY[held.severity]);
    made.source = `microfluidics: ${held.group}`;
    made.code = held.rule;
    return made;
  }

  /** What the viewer asked: to open the line that made something, or to pass something on. */
  private asked(message: FromViewer): void {
    if (message.type === "ready") {
      // A panel that was hidden and shown again is told what it last showed.
      for (const held of this.last) this.post(held);
    } else if (message.type === "reveal") {
      this.show(message.path);
    } else if (message.type === "reload") {
      void vscode.commands.executeCommand("workbench.action.reloadWindow");
    } else if (message.type === "log") {
      this.output.show(true);
    } else if (message.type === "open") {
      const origin = message.path !== undefined ? (this.scene?.origins[message.path] ?? this.nearest(message.path)) : message.file !== undefined ? { file: message.file, line: message.line ?? 1 } : undefined;
      if (origin) void this.open(origin.file, origin.line);
    } else {
      this.ask(message);
    }
  }

  /** Where the nearest thing that holds `path` was made: an element of a row, then the instance
   * it is in, and so on outward. */
  private nearest(path: string): { file: string; line: number } | undefined {
    for (let held = path; held; held = held.slice(0, Math.max(held.lastIndexOf("."), held.lastIndexOf("[")))) {
      const origin = this.scene?.origins[held];
      if (origin) return origin;
      if (!/[.[]/.test(held)) return undefined;
    }
    return undefined;
  }

  private located(file: string): vscode.Uri {
    const folder = vscode.workspace.getWorkspaceFolder(this.design)?.uri;
    return file.startsWith("/") || !folder ? vscode.Uri.file(file) : vscode.Uri.joinPath(folder, file);
  }

  /** Bring the line that made `path` into view and mark it, leaving the keyboard where it is.
   * The line shown is the nearest one, going outward from `path`, in an editor that is open
   * beside the chip: what a standard region made inside itself shows the line that placed it. */
  private show(path: string): void {
    this.marked?.setDecorations(this.mark, []);
    this.marked = undefined;
    for (let held = path; held; held = held.slice(0, Math.max(held.lastIndexOf("."), 0))) {
      const origin = this.scene?.origins[held];
      const editor = origin && vscode.window.visibleTextEditors.find((open) => open.document.uri.fsPath === this.located(origin.file).fsPath);
      if (origin && editor) {
        const line = Math.min(Math.max(origin.line - 1, 0), editor.document.lineCount - 1);
        const range = editor.document.lineAt(line).range;
        editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
        editor.setDecorations(this.mark, [range]);
        this.marked = editor;
        return;
      }
      if (!held.includes(".")) return;
    }
  }

  private async open(file: string, line: number): Promise<void> {
    const uri = this.located(file);
    try {
      const document = await vscode.workspace.openTextDocument(uri);
      const at = new vscode.Position(Math.max(line - 1, 0), 0);
      await vscode.window.showTextDocument(document, { viewColumn: vscode.ViewColumn.One, selection: new vscode.Range(at, at), preserveFocus: false });
    } catch {
      void vscode.window.showWarningMessage(`${file}:${line} could not be opened from here.`);
    }
  }

  /** Show on the chip what the line under the cursor made. */
  private cursor(editor: vscode.TextEditor): void {
    if (!this.scene) return;
    const paths = madeOn(this.scene, editor.document.uri.fsPath, editor.selection.active.line + 1);
    this.post({ type: "highlight", paths });
  }

  end(): void {
    this.ask({ type: "shutdown" });
    this.ended = true;
    this.marked?.setDecorations(this.mark, []);
    this.session?.stdin.end();
    for (const listener of this.listeners) listener.dispose();
    this.diagnostics.clear();
  }
}

export function activate(context: vscode.ExtensionContext): void {
  const previews = new Map<string, Preview>();
  const diagnostics = vscode.languages.createDiagnosticCollection("microfluidics");
  const output = vscode.window.createOutputChannel("Microfluidics");
  const mark = vscode.window.createTextEditorDecorationType({ isWholeLine: true, backgroundColor: new vscode.ThemeColor("editor.rangeHighlightBackground"), overviewRulerColor: new vscode.ThemeColor("editorOverviewRuler.rangeHighlightForeground"), overviewRulerLane: vscode.OverviewRulerLane.Center });
  const status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 10);
  status.command = "microfluidics.openPreview";
  context.subscriptions.push(diagnostics, output, mark, status, { dispose: () => previews.forEach((preview) => preview.end()) });

  /** What the editor in front is: whether it holds a design, and how its chip stands. */
  const looked = (): void => {
    const document = vscode.window.activeTextEditor?.document;
    const design = document !== undefined && document.languageId === "python" && designAt(document.getText()) !== undefined;
    void vscode.commands.executeCommand("setContext", "microfluidics.isDesign", design);
    if (!design || !document) {
      status.hide();
      return;
    }
    const held = previews.get(document.uri.fsPath);
    const name = document.uri.path.split("/").pop()?.replace(/\.py$/, "") ?? "chip";
    const { state, problems } = held?.standing ?? { state: "closed", problems: 0 };
    const said = state === "closed" ? "open preview" : state === "building" ? "$(sync~spin)" : state === "failed" ? "$(error) does not build" : problems ? `$(error) ${problems}` : "$(pass)";
    status.text = `$(circuit-board) ${name} ${said}`;
    status.backgroundColor = state === "failed" || (state === "built" && problems > 0) ? new vscode.ThemeColor("statusBarItem.errorBackground") : undefined;
    const told = state === "closed" ? "Open the chip preview of this design" : state === "building" ? "The design is being built" : state === "failed" ? "The design does not build" : problems === 0 ? "No problems: every check passes" : `${problems} ${problems === 1 ? "problem stands" : "problems stand"} in the way of the design passing`;
    status.tooltip = `${told}. Click to show the chip.`;
    status.show();
  };
  looked();
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(looked),
    vscode.workspace.onDidSaveTextDocument(looked),
    vscode.languages.registerCodeLensProvider({ language: "python" }, {
      provideCodeLenses(document) {
        const found = designAt(document.getText());
        if (found === undefined) return [];
        const at = document.positionAt(found);
        return [new vscode.CodeLens(new vscode.Range(at, at), { title: "$(open-preview) Open chip preview", command: "microfluidics.openPreview" })];
      },
    }),
  );

  /** A preview of a design, in a new panel or in one the editor kept from before it was reloaded. */
  const made = (design: vscode.Uri, restored?: vscode.WebviewPanel): Preview => {
    const key = design.fsPath;
    const closed = (): void => {
      previews.delete(key);
      looked();
    };
    const held = new Preview(design, context.extensionUri, diagnostics, output, mark, looked, closed, restored);
    previews.set(key, held);
    looked();
    return held;
  };

  /** The preview of the design in the active editor, opened when it is not yet. */
  const preview = (): Preview | undefined => {
    const document = vscode.window.activeTextEditor?.document;
    if (!document || document.languageId !== "python") {
      void vscode.window.showInformationMessage("Open a design's Python file to preview its chip.");
      return undefined;
    }
    const held = previews.get(document.uri.fsPath) ?? made(document.uri);
    held.reveal();
    return held;
  };

  // A preview that was open when the window was reloaded is opened again on the design it showed.
  context.subscriptions.push(
    vscode.window.registerWebviewPanelSerializer("microfluidics.preview", {
      deserializeWebviewPanel(panel: vscode.WebviewPanel, state: { design?: string } | undefined): Thenable<void> {
        const design = state?.design;
        if (!design || !existsSync(design) || previews.has(design)) panel.dispose();
        else made(vscode.Uri.file(design), panel);
        return Promise.resolve();
      },
    }),
  );

  context.subscriptions.push(vscode.commands.registerCommand("microfluidics.openPreview", preview));
  for (const command of ["check", "solve", "close", "corners"]) {
    context.subscriptions.push(vscode.commands.registerCommand(`microfluidics.${command}`, () => preview()?.ask({ type: "run", command })));
  }
}

export function deactivate(): void {}
