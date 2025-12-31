# AiBal

<p align="center">
  <img src="image/icon-1024.png" alt="AiBal Logo" width="128" height="128">
</p>

<p align="center">
  <strong>macOS Menu Bar AI Usage Tracker</strong>
</p>

<p align="center">
  Track your AI API usage, quotas, and balances across multiple providers in one place.
</p>

---

## Preview

<p align="center">
  <img src="image/preview.png" alt="AiBal Preview" width="800">
</p>

## Features

- **Multi-Provider Support** - Track usage across Claude, GPT, Gemini, and more
- **Real-time Monitoring** - View quota consumption and remaining balance at a glance
- **Menu Bar Integration** - Quick access from macOS menu bar tray
- **Plugin System** - Extend functionality with community plugins
- **Plugin Marketplace** - Discover and install plugins easily
- **Dark Mode** - Native dark theme UI

## Screenshots

### Dashboard
Monitor all your AI model quotas in one unified dashboard.

<p align="center">
  <img src="image/dashboard.png" alt="Dashboard" width="600">
</p>

### Plugin Management
Manage installed plugins with one-click enable/disable.

<p align="center">
  <img src="image/plugins.png" alt="Plugins" width="600">
</p>

### Plugin Marketplace
Browse and install plugins from the community marketplace.

<p align="center">
  <img src="image/marketplace.png" alt="Marketplace" width="600">
</p>

## Installation

### Download Release

Download the latest `.dmg` from [Releases](https://github.com/DDG0808/aibal/releases).

### Build from Source

#### Requirements

| Dependency | Version |
|------------|---------|
| macOS | 10.15+ (Catalina or later) |
| Node.js | 18+ |
| pnpm | 8+ |
| Rust | 1.77+ |
| Xcode CLI | Required |

#### Steps

```bash
# Clone the repository
git clone https://github.com/DDG0808/aibal.git
cd aibal

# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

## Tech Stack

### Frontend
- Vue 3 + Composition API
- Pinia (State Management)
- Vue Router
- Vite
- TypeScript

### Backend
- Tauri 2.x
- Rust
- QuickJS (Plugin Runtime)
- Tokio (Async Runtime)

## Project Structure

```
aibal/
├── src/                    # Frontend (Vue 3 + TypeScript)
│   ├── components/         # UI Components
│   ├── views/              # Page Views
│   ├── stores/             # Pinia Stores
│   ├── services/           # Service Layer
│   └── types/              # Type Definitions
│
├── src-tauri/              # Backend (Rust)
│   ├── src/
│   │   ├── commands/       # Tauri IPC Commands
│   │   ├── plugin/         # Plugin System
│   │   └── tray/           # System Tray
│   └── tauri.conf.json     # Tauri Config
│
└── contracts/              # Plugin API Contracts
    └── types/              # TypeScript Definitions
```

## Plugin Development

AiBal supports a plugin system that allows you to add custom AI provider integrations.

Plugins are JavaScript modules that run in a sandboxed QuickJS environment with controlled API access.

See [Plugin Development Guide](https://github.com/DDG0808/aibal/wiki/Plugin-Development) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

[MIT](LICENSE)

## Acknowledgments

Built with [Tauri](https://tauri.app/) and [Vue.js](https://vuejs.org/).
