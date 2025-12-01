import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, Executable } from "vscode-languageclient/node";
import * as path from "node:path";
import * as fs from "node:fs";

let client: LanguageClient | undefined;

function platformFolder(): string {
  const p = process.platform;   // 'win32' | 'darwin' | 'linux'
  const a = process.arch;       // 'x64' | 'arm64' | ...
  if (p === "darwin" && a === "x64") return "darwin-x64";
  if (p === "darwin" && a === "arm64") return "darwin-arm64";
  if (p === "linux" && a === "x64") return "linux-x64";
  if (p === "linux" && a === "arm64") return "linux-arm64";
  if (p === "win32" && a === "x64") return "win32-x64";
  vscode.window.showErrorMessage(`RJS: unsupported platform: ${p}/${a}`);
  return "";
}

function bundledServerPath(context: vscode.ExtensionContext): string | undefined {
  const bin = process.platform === "win32" ? "rjs-lsp.exe" : "rjs-lsp";
  const rel = path.join("server", platformFolder(), bin);
  if (!rel) return undefined;
  const abs = context.asAbsolutePath(rel);
  if (!fs.existsSync(abs)) {
    vscode.window.showErrorMessage(
      `RJS: bundled server not found at ${rel}. Did you run the bundling script?`
    );
    return undefined;
  }
  if (process.platform !== "win32") {
    try { fs.chmodSync(abs, 0o755); } catch { }
  }
  return abs;
}

function createClient(context: vscode.ExtensionContext): LanguageClient | undefined {
  const command = bundledServerPath(context);
  if (!command) return undefined;

  const serverOptions: Executable = { command, args: [], options: { env: process.env } };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ language: "rjscript", scheme: "file" }, { language: "rjscript", scheme: "untitled" }],
    synchronize: { configurationSection: "rjs" },
  };

  return new LanguageClient("rjs-lsp", "RJS Language Server", serverOptions, clientOptions);
}

export async function activate(context: vscode.ExtensionContext) {
  client = createClient(context);
  if (!client) return;
  await client.start();

  context.subscriptions.push(
    vscode.commands.registerCommand("rjs.restartServer", async () => {
      if (client) { await client.stop(); }
      let new_client = createClient(context);
      if (!new_client) return;
      client = new_client;
      await client.start();
      vscode.window.showInformationMessage("RJS Language Server restarted.");
    })
  );

  context.subscriptions.push({ dispose: () => { if (client) { void client.stop(); } } });
}

export async function deactivate(): Promise<void> {
  if (client) { await client.stop(); }
}
