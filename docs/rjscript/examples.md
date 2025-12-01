# RJScript Examples

## Simple Dynamic Response

Return a static object or one derived from the request.

```js
return { 
    status: "ok", 
    received: req.body 
};
```

---

## Validate Login

Check credentials and return an error or a token.

```js
if (req.body.password != "secret") {
    return 400, { error: "Invalid password" };
}
return { token: "abc123_token" };
```

---

## Stateful Counter (Cache)

Use the global cache to maintain state across requests.

```js
let hits: num = 0;
if(cacheGet("hits") != undefined) {
    hits = cacheGet("hits") + 1;
} 
cacheSet("hits", hits);
return { hits: hits };
```

---

## Generate List

Create a list dynamically using a loop.

```js
let list: vec<obj> = [];
for (let i: num = 0; i < 5; i = i + 1) {
    list.push({ id: i, name: "Item " + toString(i) }); // you can also use templating `Item ${toString(i)}`
}
return list;
```

---

## Database CRUD

A complete handler that interacts with the persistent database.

```js
// Assume this script handles POST /users
let name: str = req.body.name;

if (toType(name) != "string") {
    return { error: "Name is required" };
}

// Create entry
dbCreateEntry("users", { name: name, created_at: "now" });

// Return all users
return dbGetAll("users");
```
