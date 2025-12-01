# Methods

Each resource can define any HTTP method:

- GET
- POST
- PUT
- PATCH
- DELETE
- HEAD
- OPTIONS

A method may return a **static response** or a **dynamic response** powered by `rjscript`.

Example:

```json
{
  "methods": [
    {
      "method": "POST",
      "response": {
        "body": { "message": "PostReply!" }
      }
    },
    {
      "method": "GET",
      "script": "return cacheGet(\"example\");"
    },
    {
      "method": "DELETE",
      "script": {
        "fref": "DeleteEndpoint.rjscript"
      }
    }
  ]
}
```

Method fields:
- `method` → name of the method that the endpoint accepts
- `response`/`script`/`script.fref` → what is returned when the method is called

# Responses

A method can return:

## 1. Static Response

A static response must be an object with a required `body` field and an optional `status` field:

- `response.body` → JSON value returned as the HTTP response body
- `response.status` → HTTP status code (defaults to `200` if omitted)

Valid examples:

```json
{
  "method": "GET",
  "response": {
    "body": "Mock GET response for client details"
  }
}
{
  "method": "DELETE",
  "response": {
    "status": 204,
    "body": true
  }
}
{
  "method": "PUT",
  "response": {
    "body": ["user1", "user2", "user3"]
  }
}
{
  "method": "POST",
  "response": {
    "status": 201,
    "body": { "name": "user", "role": "admin" }
  }
}
```

## 2. Dynamic Response (inline script)

```json
{
  "method": "GET",
  "script": "return dbGetAll();"
}
```

## 3. Dynamic Response (external script)

```json
{
  "method": "GET",
  "script": {
    "fref": "./scripts/GetUser.rjscript"
  }
}
```

Dynamic scripts can:
- read request data
- access global cache
- access persistent data
- simulate validation / failures