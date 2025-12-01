# Built-in Functions

RJScript provides a set of global functions and methods for working with data, caching, and persistence.

## Core Functions

### `print(value, ...)`
Logs one or more values to the server console.
```js
print("Hello", 123);
```

### `sleep(ms)`
Pauses execution for the specified number of milliseconds.
```js
sleep(1000); // wait 1 second
```

### `toString(value)`
Converts any value to its string representation.
```js
let s = toString(123); // "123"
```

### `toType(value)`
Returns the type of the value as a string type literal (e.g., `number`, `string`, `obj`, `vec<number>`).
```js
let t = toType(10); // number
```

---

## String Methods

Methods available on string values.

- **`length()`**: Returns the number of characters.
- **`contains(substring)`**: Returns `true` if the string contains the substring.
- **`split(delimiter)`**: Splits the string into an array of strings.
- **`substring(start, end)`**: Returns the substring between `start` (inclusive) and `end` (exclusive).
- **`replace(from, to)`**: Replaces the first occurrence of `from` with `to`.
- **`to_chars()`**: Returns an array of single-character strings.

```js
let s = "hello";
print(s.length()); // 5
print(s.contains("ell")); // true
```

## Array Methods

Methods available on vector/array values.

- **`length()`**: Returns the number of elements.
- **`push(value)`**: Adds an element to the end. Returns the new length.
- **`remove(value)`**: Removes the first occurrence of the value. Returns `true` if found.
- **`removeAt(index)`**: Removes the element at the given index. Returns the removed element.

```js
let list = [1, 2];
list.push(3);
list.removeAt(0); // returns 1
```

---

## Cache Functions

An in-memory key-value cache shared across all scripts.

### `cacheSet(key, value)`
Stores a value in the cache.
- `key`: string
- `value`: any value
```js
cacheSet("user_1", { name: "Alice" });
```

### `cacheGet(key)`
Retrieves a value from the cache. Returns `undefined` if not found.
- `key`: string
```js
let val = cacheGet("user_1");
```

### `cacheDel(key)`
Removes a value from the cache. Returns `true` if removed, `false` otherwise.
- `key`: string

### `cacheClear()`
Clears all entries from the cache.

---

## Database Functions

Simple persistent table storage.

### Table Management

- **`dbCreateTable(name)`**: Creates a new table.
- **`dbGetAllTables()`**: Returns an array of table names.
- **`dbDropTable(name)`**: Deletes a table and all its data.
- **`dbDrop()`**: Deletes the entire database.

### CRUD Operations

- **`dbCreateEntry(table, value)`**: Inserts a new entry. Returns id of entry.
- **`dbGetAll(table)`**: Returns all entries as an array of objects. Each object has an `id` field.
- **`dbGetById(table, id)`**: Returns a single entry object (with `id`) or `undefined`.
- **`dbGetByFields(table, filter)`**: Returns an array of entries matching the filter object.
- **`dbUpdateById(table, id, patch)`**: Updates an entry by ID. Returns `true` if updated.
- **`dbUpdateByFields(table, filter, patch)`**: Updates multiple entries. Returns the count of updated entries.
- **`dbDeleteById(table, id)`**: Deletes an entry by ID. Returns `true` if deleted.
- **`dbDeleteByFields(table, filter)`**: Deletes multiple entries. Returns the count of deleted entries.