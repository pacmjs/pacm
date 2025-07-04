# Optional Dependencies Support Implementation Summary

## Overview
Added support for optional dependencies and platform compatibility checks to the PACM package manager, following npm's behavior.

## Key Features Implemented

### 1. Optional Dependencies Support
- **What they are**: Dependencies that should be installed if possible, but installation continues even if they fail
- **Use case**: Platform-specific packages (e.g., `fsevents` for macOS)
- **Behavior**: Attempt installation, log warnings on failure, don't break the process

### 2. Platform Compatibility Checks
- **OS Support**: Checks `os` field in package metadata with npm's syntax:
  - Allow list: `["darwin", "linux"]` - only these platforms
  - Block list: `["!win32"]` - all platforms except these  
  - Mixed: `["darwin", "!win32"]` - allow darwin, block win32 (block takes precedence)
- **CPU Support**: Same syntax for `cpu` field (e.g., `["x64", "!ia32"]`)
- **Current Platform Detection**: Automatically detects current OS and CPU architecture
- **npm Compatibility**: Follows exact npm behavior for platform restrictions

## Implementation Details

### Core Changes

#### 1. ResolvedPackage Structure Extended (`pacm-resolver/src/lib.rs`)
```rust
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub resolved: String,
    pub integrity: String,
    pub dependencies: HashMap<String, String>,
    pub optional_dependencies: HashMap<String, String>, // NEW
    pub os: Option<Vec<String>>,                        // NEW  
    pub cpu: Option<Vec<String>>,                       // NEW
}
```

#### 2. Platform Compatibility Module (`pacm-resolver/src/platform.rs`)
- `is_platform_compatible()`: Checks if package is compatible with current platform
- `get_current_os()`: Returns npm-compatible OS name
- `get_current_cpu()`: Returns npm-compatible CPU architecture

#### 3. Enhanced Dependency Resolution (`pacm-resolver/src/resolver.rs`)

**Sync Resolver (`resolve_full_tree`)**:
- Processes optional dependencies separately
- Platform compatibility checks for optional deps
- Warnings for incompatible packages (continues installation)
- Error handling for failed optional dependency resolution

**Async Resolver (`resolve_full_tree_async`)**:
- Parallel processing of optional dependencies
- Platform compatibility filtering
- Graceful failure handling with warnings

#### 4. Install Process Updates

**Single Installer (`pacm-core/src/install/single.rs`)**:
- Platform compatibility filtering during download phase
- Updated all install methods to handle filtered packages
- Proper error handling for optional dependencies

**Bulk Installer (`pacm-core/src/install/bulk.rs`)**:
- Same platform compatibility filtering
- Maintains original behavior for required dependencies
- Logs warnings for skipped optional dependencies

### Package Metadata Parsing

#### Enhanced JSON Parsing (`pacm-resolver/src/resolver.rs`)
```rust
// Parse optional dependencies
let optional_dependencies = version_data
    .get("optionalDependencies")
    .and_then(|deps| deps.as_object())
    .map(|deps| {
        deps.iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect()
    })
    .unwrap_or_default();

// Parse OS requirements
let os = version_data
    .get("os")
    .and_then(|os| os.as_array())
    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

// Parse CPU requirements  
let cpu = version_data
    .get("cpu")
    .and_then(|cpu| cpu.as_array())
    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());
```

## Installation Flow

### 1. Dependency Resolution
1. Parse package metadata including optional dependencies, OS, and CPU requirements
2. Resolve regular dependencies (fail on error)
3. Attempt to resolve optional dependencies (warn on error, continue)
4. Filter all packages by platform compatibility

### 2. Download Phase
1. Download compatible packages only
2. Skip incompatible packages with warnings
3. Continue installation even if optional dependencies fail

### 3. Linking Phase
1. Link all successfully downloaded packages to project `node_modules`
2. Run postinstall scripts in project context
3. Update lockfile with successful installations

## Example Usage

### Package with Optional Dependencies
```json
{
  "name": "my-package",
  "dependencies": {
    "lodash": "^4.17.21"
  },
  "optionalDependencies": {
    "fsevents": "^2.3.2"
  }
}
```

### Platform-Specific Package Examples

#### Allow List Syntax
```json
{
  "name": "darwin-only-package",
  "os": ["darwin"],
  "cpu": ["x64", "arm64"],
  "dependencies": {
    "some-dep": "^1.0.0"
  }
}
```

#### Block List Syntax (npm style)
```json
{
  "name": "no-windows-package", 
  "os": ["!win32"],
  "cpu": ["!ia32"],
  "dependencies": {
    "some-dep": "^1.0.0"
  }
}
```

#### Mixed Syntax (block takes precedence)
```json
{
  "name": "complex-platform-package",
  "os": ["darwin", "linux", "!win32"],
  "cpu": ["x64", "arm64", "!mips"],
  "dependencies": {
    "some-dep": "^1.0.0"
  }
}
```

## Expected Behavior Examples

### Example 1: On Windows (win32, x64)
```json
{
  "dependencies": { "lodash": "^4.17.21" },
  "optionalDependencies": { "fsevents": "^2.3.2" },
  "os": ["!win32"],
  "cpu": ["x64", "!ia32"]
}
```
- `lodash`: ✅ Installs successfully (required dependency)
- `fsevents`: ⚠️ Warning logged, installation continues (optional, macOS-only)  
- `os` check: ❌ Package blocked due to `!win32` (would skip entire package)
- `cpu` check: ✅ Allowed (x64 in allow list, not ia32)

### Example 2: On macOS (darwin, arm64)
```json
{
  "dependencies": { "lodash": "^4.17.21" },
  "optionalDependencies": { "fsevents": "^2.3.2" },
  "os": ["darwin", "linux"],
  "cpu": ["!x86"]
}
```
- `lodash`: ✅ Installs successfully
- `fsevents`: ✅ Installs successfully (optional, platform-compatible)
- `os` check: ✅ Allowed (darwin in allow list)
- `cpu` check: ✅ Allowed (arm64 not blocked)
- Result: Full functionality available

### Example 3: On Linux (linux, x64) 
```json
{
  "optionalDependencies": { "fsevents": "^2.3.2" },
  "os": ["!win32", "!darwin"]
}
```
- `fsevents`: ⚠️ Warning logged (optional, not platform-compatible)
- `os` check: ❌ Blocked (linux not explicitly allowed, and neither !win32 nor !darwin apply)
- Result: Package would be skipped entirely

## Error Handling

### Required Dependencies
- **Failure**: Installation stops, error reported
- **Platform incompatible**: Installation stops, error reported

### Optional Dependencies
- **Failure**: Warning logged, installation continues
- **Platform incompatible**: Warning logged, installation continues

## Testing

To test the implementation:

1. Create a test package.json with optional dependencies:
```bash
node test_optional_deps.js
pacm install
```

2. Expected output:
   - `lodash` installs successfully
   - `fsevents` shows platform compatibility warning on non-macOS
   - Installation completes successfully

## npm Compatibility

This implementation follows npm's exact specification for platform compatibility:

### OS Field Behavior
- **Allow List**: `["darwin", "linux"]` - Only install on macOS and Linux
- **Block List**: `["!win32"]` - Install on all platforms except Windows  
- **Mixed**: `["darwin", "!win32"]` - Allow macOS, block Windows (block takes precedence)
- **Platform Detection**: Uses `process.platform` equivalent values

### CPU Field Behavior  
- **Allow List**: `["x64", "arm64"]` - Only install on 64-bit architectures
- **Block List**: `["!ia32", "!arm"]` - Install on all architectures except 32-bit Intel and ARM
- **Mixed**: `["x64", "!ia32"]` - Allow x64, block ia32 (block takes precedence)
- **Architecture Detection**: Uses `process.arch` equivalent values

### Platform Value Mapping
```rust
// OS mapping (env::consts::OS -> npm format)
"windows" => "win32"
"macos" => "darwin"  
"linux" => "linux"
"freebsd" => "freebsd"
// ... etc

// CPU mapping (env::consts::ARCH -> npm format)  
"x86_64" => "x64"
"x86" => "ia32"
"aarch64" => "arm64"
"arm" => "arm"
// ... etc
```

### Error Handling Precedence
1. **Block List Check**: If platform is explicitly blocked (`!platform`), reject immediately
2. **Allow List Check**: If allow list exists and platform not in it, reject
3. **Default**: If no restrictions or only blocks (and not blocked), allow

This matches npm's behavior exactly, ensuring packages that work with npm will work with PACM.

## Benefits

1. **Full npm Compatibility**: Follows npm's exact optional dependency and platform compatibility behavior
2. **Advanced Platform Support**: Supports both allow and block lists with `!` syntax
3. **Platform Awareness**: Automatically handles platform-specific packages  
4. **Graceful Degradation**: Applications work even when optional features unavailable
5. **Better UX**: Clear warnings instead of confusing errors
6. **Performance**: Avoids downloading incompatible packages
7. **Developer Friendly**: Matches npm's behavior exactly for seamless migration

## Future Enhancements

1. **Dependency Conditions**: Support for conditional dependencies based on Node.js version
2. **Custom Platforms**: Support for custom platform specifications
3. **Optional Dependency Groups**: Support for grouping related optional dependencies
4. **Fallback Dependencies**: Automatic fallback to alternative packages when optional deps fail
