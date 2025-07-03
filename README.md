<div align="center">
<img src="./assets/logo.png" width="200" height="200" />
<h1>PACM (Package Manager)</h1>

<div style="display: flex; justify-content: center; gap: 8px; flex-wrap: wrap;">

![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-1.85+-orange)

</div>
</div>

> [!WARNING]
> PACM is currently in its early stages of development and is highly experimental. This project is not yet ready for production use and the API is subject to significant changes.

PACM is a modern package manager written in Rust, designed to provide fast, reliable, and secure package management capabilities.

## 🚧 Project Status

This project is in active development and many features are incomplete or experimental. Use at your own risk.

## ✨ Features

- [X] **Fast Package Resolution**: Efficient dependency resolution algorithm
- [ ] **Secure Downloads**: Cryptographic verification of packages
- [X] **Cross-Platform**: Support for Windows, macOS, and Linux
- [ ] **Workspace Support**: Manage multiple packages in a workspace
- [X] **Lock File**: Deterministic builds with lock file support
- [ ] **Registry Support**: Compatible with multiple package registries
- [ ] **Local Development**: Support for local package development and linking

## 🏗️ Architecture

PACM is built as a modular system with the following crates:

- **`pacm-cli`**: Command-line interface
- **`pacm-constants`**: Shared constants and configuration
- **`pacm-core`**: Core package management functionality
- **`pacm-error`**: Error handling and custom error types
- **`pacm-lock`**: Lock file management
- **`pacm-logger`**: Logging utilities
- **`pacm-project`**: Project and workspace management
- **`pacm-registry`**: Registry client and operations
- **`pacm-resolver`**: Dependency resolution engine
- **`pacm-runtime`**: Runtime environment management
- **`pacm-store`**: Package storage and caching
- **`pacm-utils`**: Shared utilities and helpers

## 🚀 Installation

> **Note**: Installation methods are not yet available as the project is still in development.

```bash
# TODO: Add installation instructions once stable
```

## 📖 Usage

> **Note**: Commands and usage examples are subject to change.

```bash
# TODO: Add usage examples
# pacm install package-name
# pacm update
# pacm remove package-name
```

## 🛠️ Development

### Prerequisites

- Rust 1.85+ (see `rust-toolchain.toml`)
- Git

### Building

```bash
git clone https://github.com/pacmjs/pacm.git
cd pacm
cargo build
```

### Running Tests

```bash
cargo test
```

### Formatting and Linting

```bash
cargo fmt
cargo check
```

## 📋 TODO

### Core Features
- [ ] **Package Resolution**: Implement core dependency resolution algorithm
- [ ] **Download Manager**: Secure package downloading and verification
- [X] **Installation System**: Package installation and linking
- [X] **Lock File Format**: Define and implement lock file specification
- [X] **Registry Protocol**: Design and implement registry communication
- [ ] **Workspace Management**: Multi-package workspace support
- [ ] **Local Development**: Support for local package development and linking
- [ ] **.npmrc Configuration**: Implement configuration file support
- [ ] **Environment Management**: Manage runtime environments and dependencies

### CLI Interface
- [X] **Install Command**: `pacm install [packages] [--dev, --global]`
  - `--dev`: Install as development dependencies
  - `--global`: Install globally
- [X] **Update Command**: `pacm update [packages]`
- [X] **Remove Command**: `pacm remove [packages]`
- [X] **List Command**: `pacm list [--global, --all, --dev]`
  - `--global`: List globally installed packages
  - `--all`: List all packages in the current project
  - `--dev`: List development dependencies
- [X] **Init Command**: `pacm init`
- [ ] **Outdated Command**: `pacm outdated`
- [ ] **Config Command**: `pacm config [get|set] <key> [value]`
- [X] **Help Command**: `pacm help`

### Quality Assurance
- [ ] **Unit Tests**: Comprehensive test coverage for all crates
- [ ] **Integration Tests**: End-to-end testing
- [ ] **Documentation**: API documentation and user guides
- [X] **Benchmarks**: Performance testing and optimization
- [X] **Error Handling**: Robust error messages and recovery

### Infrastructure
- [ ] **CI/CD Pipeline**: Automated testing and releases
- [ ] **Cross-Platform Testing**: Windows, macOS, Linux support
- [ ] **Release Process**: Automated releases and changelogs
- [ ] **Package Distribution**: Distribution through package managers

### Security
- [ ] **Cryptographic Verification**: Package signature verification
- [ ] **Vulnerability Scanning**: Automated security audits
- [ ] **Secure Defaults**: Security-first configuration
- [ ] **Supply Chain Security**: Provenance and attestation

### Performance
- [X] **Parallel Operations**: Concurrent downloads and installations
- [X] **Caching Strategy**: Efficient package caching
- [X] **Memory Optimization**: Minimize memory usage
- [X] **Network Optimization**: Efficient network utilization

## 🤝 Contributing

Contributions are welcome! However, please note that this project is in very early stages and the contribution process is not yet formalized.

## 📄 License

This project is licensed under the BSD 3 License - see the [LICENSE](LICENSE) file for details.

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/pacmjs/pacm/issues)
- **Discussions**: [GitHub Discussions](https://github.com/pacmjs/pacm/discussions)

---

**Disclaimer**: This project is experimental and not recommended for production use. The maintainers are not responsible for any issues that may arise from using this software.
