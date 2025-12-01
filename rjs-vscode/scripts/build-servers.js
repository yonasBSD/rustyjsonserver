const { execSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..");
const extRoot  = path.resolve(__dirname, "..");
const outDir   = path.join(extRoot, "server");

const BIN_BASE = "rjs-lsp";

const TARGETS = [
  { triple: "x86_64-apple-darwin",      folder: "darwin-x64",  exe: BIN_BASE },
  { triple: "aarch64-apple-darwin",     folder: "darwin-arm64",exe: BIN_BASE },
  { triple: "x86_64-unknown-linux-gnu", folder: "linux-x64",   exe: BIN_BASE },
  { triple: "aarch64-unknown-linux-gnu",folder: "linux-arm64", exe: BIN_BASE },
  { triple: "x86_64-pc-windows-msvc",   folder: "win32-x64",   exe: BIN_BASE + ".exe" }
];

function targetForCurrentHost() {
  const p = process.platform; // 'win32' | 'darwin' | 'linux'
  const a = process.arch;     // 'x64' | 'arm64'
  if (p === "darwin" && a === "x64")   return TARGETS[0];
  if (p === "darwin" && a === "arm64") return TARGETS[1];
  if (p === "linux"  && a === "x64")   return TARGETS[2];
  if (p === "linux"  && a === "arm64") return TARGETS[3];
  if (p === "win32"  && a === "x64")   return TARGETS[4];
  throw new Error(`Unsupported host: ${p}/${a}`);
}

function ensureDir(p) {
  fs.mkdirSync(p, { recursive: true });
}

function rustupAdd(target) {
  try {
    execSync(`rustup target add ${target}`, { stdio: "inherit" });
  } catch (e) {
  }
}

function buildOne(t) {
  console.log(`\n=== Building ${t.triple} ===`);
  rustupAdd(t.triple);
  execSync(`cargo build --release --target ${t.triple}`, {
    cwd: repoRoot,
    stdio: "inherit"
  });

  const src = path.join(repoRoot, "target", t.triple, "release", t.exe);
  if (!fs.existsSync(src)) {
    throw new Error(`Build succeeded but binary not found at ${src}`);
  }

  const destDir = path.join(outDir, t.folder);
  ensureDir(destDir);
  const dest = path.join(destDir, t.exe);

  fs.copyFileSync(src, dest);
  if (!t.exe.endsWith(".exe")) {
    fs.chmodSync(dest, 0o755);
  }
  console.log(` → Copied to ${path.relative(extRoot, dest)}`);
}

function main() {
  console.log("RJS bundler: building LSP and copying into extension/server/...");

  const all = process.env.ALL_PLATFORMS === "1";
  const list = all ? TARGETS : [targetForCurrentHost()];

  list.forEach(buildOne);

  console.log("\nDone. Binaries are in vscode/rjs-vscode/server/<platform-arch>/");
  console.log("Now you can `npm run package` to build the .vsix.");
}

main();
