# CLI Commands

RustyJSONServer provides a command-line interface with two main subcommands: `serve` and `build`.

## `serve`

Runs the HTTP server using the specified configuration file.

### Usage

```bash
rustyjsonserver serve [OPTIONS] --config <FILE>
```

### Options

- **`-c, --config <FILE>`** (Required)
  The path to the JSON configuration file to load.

- **`--no-watch`**
  Disables the file watcher. By default, the server watches the config file and any referenced files for changes and hot-reloads the configuration.

### Environment Variables

- **`RJS_DB_DIR`**
  Specifies the directory where the persistent JSON database files will be stored. Defaults to `./data` if not set.

- **`RJSERVER_LOG`**
  Specifies the log level. By default it's set to 'info', set to 'debug' for more detailed logs while developing your configuration.

### Example

```bash
# Run with default watching enabled
rustyjsonserver serve --config ./config.json

# Run without watching
rustyjsonserver serve --config ./config.json --no-watch

# Run with custom DB directory
RJS_DB_DIR=./my_db rustyjsonserver serve --config ./config.json
```

---

## `build`

Pre-processes a JSON configuration file by resolving all external references (`$ref`) and inlining them into a single standalone JSON file. This is useful for debugging configuration resolution or preparing a single-file deployment.

### Usage

```bash
rustyjsonserver build [OPTIONS] --config <FILE> --output <FILE>
```

### Options

- **`-c, --config <FILE>`** (Required)
  The input configuration file path.

- **`-o, --output <FILE>`** (Required)
  The output filename for the processed, monolithic JSON file.

### Example

```bash
rustyjsonserver build --config ./src/main_config.json --output ./dist/final_config.json
```
