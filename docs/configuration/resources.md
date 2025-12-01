# Resources

A resource defines an API endpoint.  
Each resource corresponds to a path segment.

Example:

```json
{
  "resources": [
    {
      "path": "example",
      "methods": [
        {
          "method": "GET",
          "response": { "body": { "message": "This is an example!" } }
        }
      ],
      "children": [
        {
          "path": ":id",
          "methods": [
            {
              "method": "GET",
              "response": {
                "status": 400,
                "body": "Bad request"
              }
            }
          ]
        }
      ]
    }
  ]
}
```

This creates:

```
/example
/example/{id_route_parameter}
```

Resources may contain:
- `path`: endpoint path
- `methods`: HTTP method definitions
- `children`: path extensions (can be stored in separate JSON files)
