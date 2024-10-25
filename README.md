# <img src="https://pacmjs.github.io/assets/assets/logo.png" height="35" width="35" style="margin-left: 25px;"></img> Pacm - Package Manager

Pacm is a package manager designed to handle package installations, upgrades, removals and a lot more with ease. This document provides an overview of the usage of Pacm and the performance improvements and new optimizations introduced in the latest version.

## Usage

### Install Packages

To install packages, use the `install` command followed by the package names:

```sh
pacm install <package1> <package2> ...
```

You can also use the `--save-dev` or `-D` flags to save dependencies as dev dependencies:

```sh
pacm install <package1> <package2> --dev
pacm install <package1> <package2> -D
```

To overwrite existing packages, use the `--force` flag:

```sh
pacm install <package1> <package2> --force
```

### Update Packages

To update one or more packages, use the `update` command followed by the package names:

```sh
pacm update <package1> <package2> ...
```

### List Installed Packages

To list all installed packages, use the `list` command:

```sh
pacm list
```

### Remove Packages

To remove one or more packages, use the `remove` command followed by the package names:

```sh
pacm remove <package1> <package2> ...
```

Note: If a package is not installed, the error message will be "Package <package> is not installed."

### Get Package Information

To get information about a package, use the `info` command followed by the package name:

```sh
pacm info <package>
```

### Search for Packages

To search for packages, use the `search` command followed by the search term:

```sh
pacm search <term>
```

### List Outdated Packages

To list packages that have newer versions available, use the `outdated` command:

```sh
pacm outdated
```

### Run Scripts

To run a script from the `scripts` section of the `package.json` file, use the `run` command followed by the script name:

```sh
pacm run <script>
# or
pacm <command>
```

If the command is not found in the `scripts` section, an error message will be displayed.

### Command Aliases

Pacm also supports the following command aliases:

-   `i` as an alias for `install`
-   `rm` as an alias for `remove`

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue on GitHub.

## License

This project is licensed under the BOWL (Buzzr open-source Works) License.

---