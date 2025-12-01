# Global Objects

RJScript exposes a request global object during script execution.

---

# `req` (Request)

The `req` object provides read-only access to the incoming HTTP request data.

## Properties

- **`req.body`**: The parsed JSON body of the request.
- **`req.params`**: An object containing route parameters (e.g., `/users/:id`).
- **`req.query`**: An object containing query string parameters (e.g., `?page=1`).
- **`req.headers`**: An object containing HTTP headers.

## Example

```js
// Accessing a route parameter
let userId: str = req.params.id;

// Checking a header
if (req.headers["x-api-key"] == "secret") {
    // ...
}

// Reading the body
let name: str = req.body.name;
```
