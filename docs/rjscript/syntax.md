# RJScript Syntax

RJScript is a small, strongly‑typed scripting language, designed for HTTP request handlers and simple data transformations.

This page summarizes the core syntax:

## Variables and Types

RJScript uses `let` for variable declarations. Types are required for declarations.

```js
let id: str = "u1";
let count: num = 10;
let active: bool = true;
let user: obj = { id: "u1", name: "Alice" };
let tags: vec<str> = ["a", "b"]; // vectors (arrays)
```

Built‑in types:

- `num`, `bool`, `str`
- `obj` – generic object
- `vec<T>` – vector/array of `T` (e.g. `vec<number>`)
- `any` – any value (allowed only for vectors, ex: vec<any>)
- `Undefined` - only used for type checking (ex: if (toType(req.headers["User-Agent"]) != Undefined))

## Objects & Arrays

Object and array literals are similar to JavaScript:

```js
let user: obj = { id: "1", name: "Bob" };
let list: vec<num>  = [1, 2, 3];

// Nested structures
let project: obj = {
    id: "p1",
    members: [
        { id: "u1", name: "Alice" },
        { id: "u2", name: "Bob" },
    ],
};
```

Property and index access:

```js
let list: vec<num> = [1, 2 ,3];

let name: str = user.name;
let id: str = user.["id"];
let first: num = list[0];
```

## Expressions

Supported expressions include:

- Literals: numbers, booleans, strings, `undefined`
- Binary operators: `+ - * / % < <= > >= == != && ||`
- Assignment: `=`
- Unary minus: `-x`
- Function calls: `print(x)`, `foo(a, b)`
- Member/index access: `obj.prop`, `obj["key"]`, `arr[0]`

## Conditionals

```js
if (req.body.age < 18) {
    return { status: 400, body: { error: "Too young" } };
} else if (req.body.age < 21) {
    return { status: 403, body: { error: "Not allowed" } };
} else {
    return { status: 200, body: { ok: true } };
}
```

RJScript also supports `switch`:

```js
switch (req.params.role) {
    case "admin":
        return { status: 200, body: { admin: true } };
    case "user":
        return { status: 200, body: { admin: false } };
    default:
        return { status: 404, body: { error: "Unknown role" } };
}
```

## Loops

```js
for (let i: num = 0; i < 3; i = i + 1) {
    print(i);
}

// You can omit parts:
for (;; ) {
    // infinite loop (use carefully)
    break;
}
```

`break` and `continue` work inside loops.

## Functions

You can define reusable functions with typed parameters and return types:

```js
func greet(name: str): obj {
    return { message: `Hello, ${name}` };
}

func sum(values: vec<num>): num {
    let total: num = 0;
    for (let i: num = 0; i < values.length; i = i + 1) {
        total = total + values[i];
    }
    return total;
}
```

## Request Object (`req`)

Handlers run in the context of an HTTP request. The `req` object exposes request data:

- `req.body` – parsed request body (JSON/object)
- `req.params` – path parameters
- `req.query` – query parameters
- `req.headers` – HTTP headers

Examples:

```js
let userId: str = req.params.id;
let page: num = req.query.page;
let auth: str = req.headers["authorization"];
let payload: obj = req.body;
```

`req.*` values are read‑only; you cannot assign to them.

## Return Values

Scripts should return a value. A recommended pattern is to return a status code and response body:

```js
return 200,{ ok: true };
```

You can also return plain values, the default status code will be 200:

```js
return 42;
return { id: "u1", name: "Alice" };
```

## Comments

Same as JavaScript:

```js
// single-line comment
/* multi-line
   comment */
```

