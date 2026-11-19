// Drives a VS Code of its own: a second instance of the one installed here, with its own profile
// and no other extension, started with this extension from its source. What it shows is read and
// worked over the Chrome DevTools Protocol; what only the editor can do is asked of `suite.cjs`.
import { spawn } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, readdirSync, realpathSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
export const EXTENSION = resolve(HERE, "../..");
export const REPO = resolve(EXTENSION, "../..");
const CODE = process.env.VSCODE_UNDER_TEST ?? "/Applications/Visual Studio Code.app/Contents/MacOS/Code";

const pause = (ms) => new Promise((done) => setTimeout(done, ms));

/** Wait until `found` gives something, asking every `every` ms for at most `most` ms. */
export async function until(what, found, most = 30000, every = 200) {
  const end = Date.now() + most;
  for (;;) {
    const held = await found().catch(() => undefined);
    if (held) return held;
    if (Date.now() > end) throw new Error(`timed out waiting for ${what}`);
    await pause(every);
  }
}

/** One page or frame, spoken to over the protocol. */
class Target {
  constructor(socket) {
    this.socket = socket;
    this.next = 1;
    this.waiting = new Map();
    this.contexts = new Map();
    this.console = [];
    socket.addEventListener("message", (event) => {
      const said = JSON.parse(event.data);
      if (said.id !== undefined) {
        const held = this.waiting.get(said.id);
        this.waiting.delete(said.id);
        if (said.error) held?.reject(new Error(said.error.message));
        else held?.resolve(said.result);
      } else if (said.method === "Runtime.executionContextCreated") this.contexts.set(said.params.context.id, said.params.context);
      else if (said.method === "Runtime.executionContextDestroyed") this.contexts.delete(said.params.executionContextId);
      else if (said.method === "Runtime.executionContextsCleared") this.contexts.clear();
      else if (said.method === "Runtime.consoleAPICalled") this.console.push({ kind: said.params.type, text: said.params.args.map((held) => held.value ?? held.description ?? "").join(" ") });
      else if (said.method === "Runtime.exceptionThrown") this.console.push({ kind: "error", text: said.params.exceptionDetails.exception?.description ?? said.params.exceptionDetails.text });
    });
  }

  static async to(url, page = true) {
    const socket = new WebSocket(url);
    await new Promise((opened, failed) => {
      socket.addEventListener("open", opened);
      socket.addEventListener("error", failed);
    });
    const target = new Target(socket);
    if (!page) return target;
    await target.send("Runtime.enable");
    await target.send("Page.enable").catch(() => undefined);
    return target;
  }

  send(method, params = {}) {
    const id = this.next++;
    return new Promise((resolve, reject) => {
      this.waiting.set(id, { resolve, reject });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }

  /** The value of `expression` in the frame that holds the preview. */
  async evaluate(expression) {
    for (const id of this.contexts.keys()) {
      const here = await this.send("Runtime.evaluate", { expression: "document.querySelector('#chip') !== null", contextId: id, returnByValue: true }).catch(() => undefined);
      if (!here?.result?.value) continue;
      const found = await this.send("Runtime.evaluate", { expression, contextId: id, returnByValue: true, awaitPromise: true });
      if (found.exceptionDetails) throw new Error(found.exceptionDetails.exception?.description ?? found.exceptionDetails.text);
      return found.result.value;
    }
    throw new Error("the preview's frame is not there");
  }

  /** The value of `expression` in the frame that holds the preview's own frame. */
  async outer(expression) {
    for (const id of this.contexts.keys()) {
      const here = await this.send("Runtime.evaluate", { expression: "document.getElementById('active-frame') !== null", contextId: id, returnByValue: true }).catch(() => undefined);
      if (!here?.result?.value) continue;
      const found = await this.send("Runtime.evaluate", { expression, contextId: id, returnByValue: true, awaitPromise: true });
      if (found.exceptionDetails) throw new Error(found.exceptionDetails.exception?.description ?? found.exceptionDetails.text);
      return found.result.value;
    }
    throw new Error("the frame about the preview is not there");
  }

  /** Give the preview a size of its own within the room the editor gives it, or with nothing
   * all of that room again. Gives the size it has. */
  async sized(width, height) {
    const style = width === undefined ? "''" : `'position:absolute;left:0;top:0;width:${width}px;height:${height}px;border:0'`;
    await this.outer(`(() => { const frame = document.getElementById('active-frame'); if (${style} === '') { frame.style.width = '100%'; frame.style.height = '100%'; } else frame.style.cssText = ${style}; })()`);
    await new Promise((done) => setTimeout(done, 350));
    return this.evaluate("[innerWidth, innerHeight]");
  }

  /** The room the editor gives the preview. */
  room() {
    return this.outer("[innerWidth, innerHeight]");
  }

  async screenshot(file) {
    const { data } = await this.send("Page.captureScreenshot", { format: "png" });
    writeFileSync(file, Buffer.from(data, "base64"));
    return file;
  }

  close() {
    this.socket.close();
  }
}

/** A VS Code under test, with a copy of the designs as its workspace. */
export class Code {
  /** Start it. With `hooked`, the editor answers what is asked of it from inside; without, it is
   * an editor as a person starts one, worked by its keys alone, with `open` shown in it. */
  static async start({ settings = {}, width = 1500, height = 950, hooked = true, open } = {}) {
    const root = realpathSync(mkdtempSync(join(tmpdir(), "mf-vscode-")));
    const [user, extensions, workspace] = [join(root, "user"), join(root, "extensions"), join(root, "work")];
    mkdirSync(join(user, "User"), { recursive: true });
    mkdirSync(extensions);
    mkdirSync(workspace);
    for (const file of readdirSync(join(REPO, "designs"))) if (/\.(py|toml)$/.test(file)) cpSync(join(REPO, "designs", file), join(workspace, file));
    // The manifold with nothing tuned beside it, whose lanes are unequal as it is written.
    cpSync(join(REPO, "designs", "mother_machine_manifold.py"), join(workspace, "untuned_manifold.py"));
    writeFileSync(
      join(user, "User", "settings.json"),
      JSON.stringify({
        "microfluidics.python": join(REPO, ".venv/bin/python"),
        "security.workspace.trust.enabled": false,
        "workbench.startupEditor": "none",
        "workbench.colorTheme": "Default Dark Modern",
        "telemetry.telemetryLevel": "off",
        "update.mode": "none",
        "extensions.autoUpdate": false,
        "extensions.ignoreRecommendations": true,
        "window.restoreWindows": "none",
        "window.newWindowDimensions": "default",
        "git.enabled": false,
        "workbench.tips.enabled": false,
        "chat.commandCenter.enabled": false,
        ...settings,
      }),
    );
    const [debug, control] = [9339 + Math.floor(Math.random() * 400), 9800 + Math.floor(Math.random() * 100)];
    const child = spawn(
      CODE,
      [`--user-data-dir=${user}`, `--extensions-dir=${extensions}`, `--extensionDevelopmentPath=${EXTENSION}`, ...(hooked ? [`--extensionTestsPath=${join(HERE, "suite.cjs")}`] : []), `--remote-debugging-port=${debug}`, "--disable-workspace-trust", "--skip-welcome", "--skip-release-notes", "--disable-updates", "--new-window", workspace, ...(open ? [join(workspace, open)] : [])],
      { env: { ...process.env, MICROFLUIDICS_TEST_PORT: String(control), ELECTRON_RUN_AS_NODE: undefined }, stdio: "ignore" },
    );
    const code = new Code(root, workspace, debug, control, child);
    try {
      if (hooked) await until("the editor to answer", () => code.ask({ command: "setContext", args: ["microfluidics.test", true] }), 60000);
      else await until("the editor's window", async () => (await code.targets()).some((held) => held.type === "page" && held.url.includes("workbench")), 60000);
      code.size = await code.window(width, height);
    } catch (error) {
      await code.stop();
      throw error;
    }
    return code;
  }

  constructor(root, workspace, debug, control, child) {
    Object.assign(this, { root, workspace, debug, control, child });
  }

  /** Ask the editor, from inside it. */
  async ask(asked) {
    const response = await fetch(`http://127.0.0.1:${this.control}`, { method: "POST", body: JSON.stringify(asked) });
    const said = await response.json();
    if (said.error) throw new Error(said.error);
    return said;
  }

  async targets() {
    return (await fetch(`http://127.0.0.1:${this.debug}/json/list`)).json();
  }

  /** The editor's own window. */
  async workbench() {
    const found = await until("the editor's window", async () => (await this.targets()).find((held) => held.type === "page" && held.url.includes("workbench")));
    return Target.to(found.webSocketDebuggerUrl);
  }

  /** Run a command by its name, as a person does: from the Command Palette, by the keyboard. */
  async command(name) {
    const bench = await this.workbench();
    const key = (type, held) => bench.send("Input.dispatchKeyEvent", { type, ...held });
    await key("rawKeyDown", { key: "F1", code: "F1", windowsVirtualKeyCode: 112, nativeVirtualKeyCode: 122 });
    await key("keyUp", { key: "F1", code: "F1", windowsVirtualKeyCode: 112, nativeVirtualKeyCode: 122 });
    await pause(700);
    await bench.send("Input.insertText", { text: name });
    await pause(900);
    await key("rawKeyDown", { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 36 });
    await key("char", { key: "Enter", code: "Enter", text: "\r", windowsVirtualKeyCode: 13 });
    await key("keyUp", { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 36 });
    bench.close();
  }

  /** Set the size of the editor's window. */
  async window(width, height) {
    const bench = await this.workbench();
    await bench.send("Runtime.evaluate", { expression: `window.moveTo(40, 40); window.resizeTo(${width}, ${height})` });
    await pause(300);
    const { result } = await bench.send("Runtime.evaluate", { expression: "[window.outerWidth, window.outerHeight]", returnByValue: true });
    bench.close();
    return result.value;
  }

  /** Open a design of the workspace and its preview; gives the preview's frame once it has a scene. */
  async preview(design) {
    await this.ask({ open: join(this.workspace, design) });
    await this.ask({ command: "microfluidics.openPreview" });
    return until("the preview's frame", () => this.frame());
  }

  /** The preview's frame, when there is one and it holds the preview. */
  async frame() {
    const found = (await this.targets()).find((held) => held.url.startsWith("vscode-webview://") && held.type !== "service_worker");
    if (!found) return undefined;
    const frame = await Target.to(found.webSocketDebuggerUrl);
    try {
      await until("the preview to be there", () => frame.evaluate("true"), 5000);
      return frame;
    } catch {
      frame.close();
      return undefined;
    }
  }

  /** A picture of the editor's window, or of the preview alone as the window shows it. */
  async picture(file, { whole = false, size } = {}) {
    const bench = await this.workbench();
    const found = await bench.send("Runtime.evaluate", {
      expression: "(() => { const held = [...document.querySelectorAll('iframe.webview')].map((frame) => frame.getBoundingClientRect()).filter((box) => box.width > 0 && box.height > 0)[0]; return held ? [held.x, held.y, held.width, held.height] : null; })()",
      returnByValue: true,
    });
    const box = found.result.value;
    const zoom = (await bench.send("Runtime.evaluate", { expression: "window.outerWidth / window.innerWidth", returnByValue: true })).result.value;
    // A size given is in the preview's own pixels, which the editor's zoom makes larger or smaller.
    const [wide, high] = size && box ? [Math.min(size[0], box[2]), Math.min(size[1], box[3])] : box ? [box[2], box[3]] : [0, 0];
    const clip = !whole && box ? { clip: { x: box[0] * zoom, y: box[1] * zoom, width: wide * zoom, height: high * zoom, scale: 1 } } : {};
    const { data } = await bench.send("Page.captureScreenshot", { format: "png", ...clip });
    bench.close();
    writeFileSync(file, Buffer.from(data, "base64"));
    return file;
  }

  async stop() {
    await this.ask({ end: true }).catch(() => undefined);
    await pause(500);
    this.child.kill();
  }
}
