# RustyJSONServer

RustyJSONServer is a fast, flexible mock API server powered by JSON configuration and a lightweight scripting language (`rjscript`).  
It lets you build **static or dynamic APIs** without writing backend code — perfect for prototyping, testing, and front-end development workflows.

---

## 🚀 What it does

- Create endpoints using a simple **JSON config**
- Return **static JSON** or **dynamic data** via inline or external `.rjscript` files
- Split large mock APIs into multiple files using **nested configs**
- Reload automatically when config or script files change
- Maintain state using a **global in-memory cache**
- Maintain persistent data using the **integrated database**

Designed to simulate realistic API behavior with minimal setup.

---

## 📦 Quick Example

```json
{
  "port": 8080,
  "resources": [
    {
      "path": "hello",
      "methods": [
        {
          "method": "GET",
          "response": {
            "status": 200,
            "body": {
              "message": "Hello, World!"
            }
          }
        }
      ]
    }
  ]
}
```

---

## 🔨 CLI

Start a server:

```
rjserver serve --config config.json
```

## 💻 VSCode Extension Included

The repository includes a VSCode extension providing syntax highlighting and error messages for .rjscript files

---

## 📚 Documentation

Full documentation and examples can be found in the  
📁 **`/docs`** folder.
📁 **`/examples`** folder.

---

## 🤝 Contributing

Contributions and pull requests are welcome!

---

## 📄 License

MIT