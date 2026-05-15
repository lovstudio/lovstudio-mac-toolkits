<p align="center">
  <img src="docs/images/cover.png" alt="Lovstudio.ai Mac Menu Manager Cover" width="100%">
</p>

<h1 align="center">
  <img src="assets/logo.svg" width="32" height="32" alt="Logo" align="top">
  Lovstudio.ai Mac Menu Manager
</h1>

<p align="center">
  <strong>A pluggable macOS menu bar manager from Lovstudio.ai, 手工川工作室.</strong><br>
  <sub>macOS / Tauri / React / TypeScript / Rust</sub>
</p>

<p align="center">
  <a href="#features">Features</a> ·
  <a href="#install">Install</a> ·
  <a href="#usage">Usage</a> ·
  <a href="#release">Release</a>
</p>

## Features

- Provide a modular foundation for Lovstudio.ai macOS menu bar utilities.
- Ship the first module, Lid Sleep Guard, for preventing lid-close sleep during long-running work.
- Support global protection and per-app privileged protection.
- Detect and group running apps, CLI tools, helpers, and background processes.
- Use a privileged helper for `pmset` changes, avoiding repeated admin prompts after setup.
- Ship signed macOS releases with Tauri updater artifacts.

## Install

Download the latest universal macOS build from:

[GitHub Releases](https://github.com/lovstudio/lovstudio-mac-toolkits/releases/latest)

The app is Developer ID signed and notarized. The first time protection changes require privileged system configuration, macOS may ask for administrator approval.

## Usage

Open Lovstudio.ai Mac Menu Manager from the macOS menu bar.

- **Lid Sleep Guard** is the first installed module.
- **Global Mode** protects every detected running app while enabled.
- **Privileged Apps** keeps selected apps protected independently of the global switch.
- **Background processes** are available in a secondary expandable section to keep the main list focused.
- **Refresh** rescans the current process list.

The first module is designed for long-running local work such as AI agents, coding sessions, rendering, builds, and other tasks that should not be paused by system sleep.

## Development

```bash
pnpm install
pnpm dev
```

Run checks locally:

```bash
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
```

Build a macOS bundle:

```bash
pnpm tauri build --target universal-apple-darwin
```

## Tech Stack

- Tauri 2
- Rust
- React 19
- TypeScript
- Tailwind CSS
- TanStack Query
- GitHub Actions

## Release

Releases are created through GitHub Actions from `v0.*` tags. The workflow builds a universal macOS bundle, signs and notarizes it, uploads updater artifacts, then publishes the release.

Current updater endpoint:

```text
https://github.com/lovstudio/lovstudio-mac-toolkits/releases/latest/download/latest.json
```

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=lovstudio/lovstudio-mac-toolkits&type=Date)](https://star-history.com/#lovstudio/lovstudio-mac-toolkits&Date)

## License

Apache-2.0
