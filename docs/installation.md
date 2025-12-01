# Installation

RustyJSONServer can be installed by building it from source.

---

## 💻 Requirements

- Rust and cargo installed
- Node and npm (for vscode extensions)

---

## 🛠 Build from source

```
git clone <project-github-url>
cd rustyjsonserver
cargo build --release
```

Binary output:

```
target/release/rjserver
```

---

## 🧩 VSCode Extension (optional)

Located at:

```
/rjs-vscode
```

Build extension:

```
npm install
npm run build:servers
npm run compile
npx vsce package
```

Install extension:
code --install-extension ./rjs-vscode-0.0.1.vsix --force

---

## 📨 Verify installation

```
rjserver help
```

---
