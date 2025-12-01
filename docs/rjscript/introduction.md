# Introduction to RJScript

RJScript is a lightweight, strongly-typed scripting language built specifically for RustyJSONServer.
It enables dynamic mock API behavior without writing full backend code.

You can use it to:
- Validate request data
- Generate dynamic responses
- Simulate authentication
- Manage state via a global in-memory cache
- Persist data using the built-in JSON database
- Create branching logic and loops

Scripts can be:
- **Inline** inside JSON configuration files
- **External** in `.rjscript` files

RJScript is safe, sandboxed, and intentionally limited to API‑mocking use cases. It does not have access to the host system (files, network, etc.) beyond the provided built-ins.
