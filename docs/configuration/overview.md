# Configuration Overview

RustyJSONServer is fully driven by JSON configuration files. A config describes:
- resources (endpoints)
- methods (GET, POST, PUT, etc.)
- static or dynamic responses
- optional scripts (inline or in external `.rjscript` files)
- nested configs using `children`

Every configuration starts with a **root object** containing at least a `resources` field.

```json
{
  "port": 8080,
  "resources": [
    {
      "path": "example",
      "methods": [
        {
          "method": "GET",
          "response": {
            "body": { "message": "This is an example!" }
          }
        }
      ]
    }
  ]
}
```