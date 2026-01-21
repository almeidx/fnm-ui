# AI Agent Instructions for fnm-ui

## Project Overview

fnm-ui is a native GUI application for [fnm](https://github.com/Schniz/fnm) (Fast Node Manager). It provides a graphical interface to manage Node.js versions on your system.

## Technology Stack

- **Language**: Rust (2021 edition)
- **GUI Framework**: [Iced](https://iced.rs/) 0.13 (Elm architecture)
- **Async Runtime**: Tokio
- **Build System**: Cargo workspace

## Project Structure

```
fnm-ui/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── fnm-ui/                   # Main GUI application
│   │   └── src/
│   │       ├── main.rs           # Entry point
│   │       ├── app.rs            # Iced Application implementation
│   │       ├── message.rs        # Message enum (Elm-style)
│   │       ├── state.rs          # Application state structs
│   │       ├── theme.rs          # Light/dark themes and styles
│   │       ├── settings.rs       # User settings persistence
│   │       ├── views/            # UI views (main_view, onboarding, loading)
│   │       └── widgets/          # Custom widgets (version_list, install_modal, toast)
│   ├── fnm-core/                 # fnm CLI wrapper library
│   │   └── src/
│   │       ├── client.rs         # FnmClient - command execution
│   │       ├── version.rs        # NodeVersion types & parsing
│   │       ├── progress.rs       # Install progress tracking
│   │       ├── detection.rs      # fnm binary detection
│   │       ├── schedule.rs       # Node.js release schedule fetching
│   │       └── error.rs          # Error types
│   ├── fnm-shell/                # Shell detection & configuration
│   │   └── src/
│   │       ├── detect.rs         # Shell detection
│   │       ├── config.rs         # Config file editing
│   │       ├── shells/           # Shell-specific implementations
│   │       └── verify.rs         # Configuration verification
│   └── fnm-platform/             # Platform abstractions
│       └── src/
│           ├── paths.rs          # Platform-native paths
│           └── environment.rs    # Environment abstraction
```

## Architecture

### Elm Architecture (Model-View-Update)

The application follows Iced's Elm-style architecture:

1. **State** (`state.rs`): Immutable application state
2. **Message** (`message.rs`): Events that can modify state
3. **Update** (`app.rs`): Handles messages and returns new state + tasks
4. **View** (`views/`): Pure functions that render state to UI

### Key Patterns

- **Tasks**: Async operations return `Task<Message>` for side effects
- **Subscriptions**: Time-based events (tick for toast timeouts)
- **Theming**: Dynamic light/dark themes based on system preference

## Development Commands

```bash
# Build the project
cargo build

# Run the application
cargo run

# Run with release optimizations
cargo build --release

# Check for errors without building
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

## Code Style Guidelines

- Follow standard Rust conventions (rustfmt)
- Use `thiserror` for error types
- Prefer `async/await` over callbacks
- Keep view functions pure (no side effects)
- Use meaningful message names that describe the event
- Group related functionality into separate crates

## Key Files to Understand

1. `crates/fnm-ui/src/app.rs` - Main application logic and message handling
2. `crates/fnm-ui/src/state.rs` - All state types and their relationships
3. `crates/fnm-ui/src/message.rs` - All possible application events
4. `crates/fnm-core/src/client.rs` - fnm CLI interaction

## Common Tasks

### Adding a New Feature

1. Add new message variant(s) to `message.rs`
2. Add state fields to `state.rs` if needed
3. Handle message in `app.rs` update function
4. Update view in appropriate `views/` file

### Adding a New fnm Command

1. Add method to `FnmClient` in `fnm-core/src/client.rs`
2. Add any new types to `version.rs` if needed
3. Create corresponding message and handler in fnm-ui

### Modifying Styles

- All styles are in `crates/fnm-ui/src/theme.rs`
- Light/dark palettes defined at the top
- Button and container styles as functions

## Testing

- Unit tests should be in the same file as the code
- Integration tests in `tests/` directory
- Test fnm interactions with mock or real fnm installation

## Dependencies

Key external crates:
- `iced` - GUI framework
- `tokio` - Async runtime
- `reqwest` - HTTP client (for release schedule)
- `serde` - Serialization
- `open` - Opening URLs in browser
- `dirs` - Platform directories
- `which` - Finding executables

## Data & Storage

**Settings Location:**
- macOS: `~/Library/Application Support/fnm-ui/`
- Windows: `%APPDATA%/fnm-ui/`
- Linux: `~/.config/fnm-ui/` (XDG-compliant)

**Cached Data:**
- Available Node versions list (fetched from nodejs.org)
- Node.js release schedule (from GitHub)

## fnm Interaction

- All fnm operations execute CLI commands as subprocesses via `FnmClient`
- Parse stdout/stderr for status and results
- Long-running operations (install/download) run in async tasks
- UI remains responsive during operations via Iced's `Task` system

**Key fnm commands used:**
- `fnm list` - Get installed versions
- `fnm list-remote` - Get available versions
- `fnm install <version>` - Install a version
- `fnm uninstall <version>` - Remove a version
- `fnm default <version>` - Set default version
- `fnm current` - Get currently active version

## Platform-Specific Notes

### macOS
- Primary development target
- Native ARM64 and x64 binaries
- Uses `dark-light` crate for system theme detection

### Windows
- Native Windows binary
- Support for PowerShell shell configuration
- WSL integration via `wsl.exe` for multi-environment support

### WSL (Windows Subsystem for Linux)
- Accessed via Windows app's multi-environment support
- Uses `wsl.exe --list` for distro detection
- Each distro treated as separate environment
- Commands executed via `wsl.exe -d <distro> fnm ...`

### Linux
- Native x64 and ARM64 binaries
- XDG-compliant paths
- Support for bash, zsh, fish shells

## Not Yet Implemented

These features are planned but not yet built:
- System tray with quick-switch menu
- fnm update checking and in-app update
- Parallel install operations
- Window size/position persistence
