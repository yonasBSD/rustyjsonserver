# Configuration Examples

## Simple GET

```json
{
  "resources": [
    {
      "path": "hello",
      "methods": [
        {
          "method": "GET",
          "response": {
            "body": { "msg": "Hello!" }
          }
        }
      ]
    }
  ]
}
```

## Dynamic Login

```json
{
  "resources": [
    {
      "path": "login",
      "methods": [
        {
          "method": "POST",
          "script": {
            "fref": "./scripts/login.rjscript"
          }
        }
      ]
    }
  ]
}
```

`login.rjscript`:

```js
if (req.body.password != "secret") {
  return { error: "Invalid password" };
}
return { token: "abc123" };
```

## Nested Resources

```json
{
  "resources": [
    {
      "path": "api",
      "children": [
        {
          "path": "users",
          "fref": "./configs/users.json"
        }
      ]
    }
  ]
}
```

`configs/users.json`:

```json
{
  "resources": [
    {
      "path": "profile",
      "methods": [
        {
          "method": "GET",
          "response": {
            "body": { "name": "Alice" }
          }
        }
      ]
    }
  ]
}
```

This creates `/users/profile`.
