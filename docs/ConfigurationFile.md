# Jarvy Configuration File

Jarvy uses a configuration file named `jarvy.toml` to set up your local development environment.
This configuration file is written in TOML format and allows you to specify the tools you need,
their versions, and whether to use a package manager for installation.

## Configuration File Structure

The configuration file contains a `[tools]` section where you list the tools you want to install.
Each tool can be specified with either a simple version string or a detailed configuration object.

### Example Configuration File

```toml
[tools]
git = { version = "2.37.0", version_manager = true }
node = { version = "14.15.0" }  # version_manager defaults to true
python3 = { version = "3.9.0", version_manager = false }
docker = "latest"  # version defaults to latest and version_manager defaults to true
```

## Tool Configuration

### Simple Configuration

You can specify a tool with a simple version string. This will set the version to the specified value and use the package manager by default.

```toml
[tools]
tool_name = "version"
```

- **Example**: `git = "latest"`

### Detailed Configuration

You can specify a tool with a detailed configuration object. This allows you to specify the version and whether to use a package manager.

```toml
[tools]
tool_name = { version = "version", version_manager = bool }
```

- **version**: The version of the tool to install. Use `"latest"` to install the latest version.
- **version_manager**: A boolean indicating whether to use a package manager for installation. Defaults to `true`.

- **Example**: `node = { version = "14.15.0", version_manager = true }`

## Default Values

- If the version is not specified, it defaults to `"latest"`.
- If the package manager is not specified, it defaults to `true`.

### Example

```toml
[tools]
git = { version = "2.37.0", version_manager = true }  # Specify version and use package manager
node = { version = "14.15.0" }  # Specify version, use package manager by default
python3 = { version = "3.9.0", version_manager = false }  # Specify version, do not use package manager
docker = "latest"  # Use latest version, use package manager by default
```

## Loading and Using the Configuration

Jarvy will automatically load the configuration file when you run the `setup` command. You can specify a custom configuration file path using the `--file` option.

### Default Usage

By default, Jarvy looks for `jarvy.conf` in the current directory:

```sh
jarvy setup
```

### Custom Configuration File

You can specify a custom path to the configuration file using the `--file` option:

```sh
jarvy setup --file ./path/to/your/config/file.toml
```

## Error Handling

Jarvy will report an error if the configuration file is not found or if there is an issue with the TOML syntax. Make sure your configuration file is correctly formatted and located at the specified path.

## Conclusion

This configuration file format allows you to easily define the tools and versions needed for your project. By leveraging package managers, Jarvy ensures that your local development environment is set up quickly and efficiently.

For further details and updates, please refer to the official Jarvy documentation.