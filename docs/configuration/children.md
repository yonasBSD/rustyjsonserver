# Children

Children allow you to split large APIs into multiple JSON files and compose paths step by step.

A resource can have:

- a `path`
- optional `methods`
- optional `children` (inline child resources)
- optional `fref` pointing to another JSON file that defines more `methods` and `children` for that path

---

## Example: Main entry file

**Main.json**

```json
{
  "port": 8080,
  "resources": [
    {
      "path": "api/v1",
      "children": [
        {
          "path": "users",
          "fref": "Users/Users.json"
        },
        {
          "path": "projects",
          "fref": "Projects/Projects.json"
        },
        {
          "path": "clients",
          "methods": [
            {
              "method": "GET",
              "response": {
                "body": { "users": ["user1", "user2", "user3"] }
              }
            }
          ],
          "children": [
            {
              "path": "details",
              "methods": [
                {
                  "method": "GET",
                  "response": { "body": "Mock GET response for client details" }
                }
              ]
            }
          ]
        }
      ]
    }
  ]
}
```

This defines:

- base path: `/api/v1`
- `/api/v1/users` → configuration loaded from `Users/Users.json`
- `/api/v1/projects` → configuration loaded from `Projects/Projects.json`
- `/api/v1/clients` and `/api/v1/clients/details` → defined inline

---

## Example: Users children file

**Users/Users.json**

```json
{
  "methods": [
    {
      "method": "GET",
      "response": {
        "body": { "users": ["user1", "user2", "user3"] }
      }
    },
    {
      "method": "POST",
      "script": {
        "fref": "PostUsers.rjscript"
      }
    }
  ],
  "children": [
    {
      "path": "details",
      "methods": [
        {
          "method": "GET",
          "response": { "body": "Mock GET response for user details" }
        },
        {
          "method": "POST",
          "response": { "body": "Mock POST response for user details" }
        }
      ]
    },
    {
      "path": ":userId",
      "fref": "User/User.json"
    }
  ]
}
```

This extends `/api/v1/users` with:

- `/api/v1/users` → GET, POST (from `methods`)
- `/api/v1/users/details` → extra nested routes
- `/api/v1/users/:userId` → more config loaded from `User/User.json`

---

## Example: Projects children file

**Projects/Projects.json**

```json
{
  "methods": [
    {
      "method": "GET",
      "script": "let x = 42; return x;"
    },
    {
      "method": "POST",
      "response": {
        "body": {
          "name": "new_project",
          "status": "active"
        }
      }
    },
  ]
}
```

This defines methods for:

- `/api/v1/projects`

---

## Resulting paths

Using the examples above, the following routes are created:

- `GET  /api/v1/users`
- `POST /api/v1/users`
- `GET  /api/v1/users/details`
- `POST /api/v1/users/details`
- `/api/v1/users/:userId` (methods defined in `User/User.json`)
- `GET  /api/v1/projects`
- `POST /api/v1/projects`
- `GET  /api/v1/clients`
- `GET  /api/v1/clients/details`

Children can be nested as deeply as you need by combining `path`, `children`, and `fref`.
