# TSPM (The Server Process Manager)

TSPM is a high-performance, lightweight process manager written in Rust. Inspired by tools like PM2, it provides a robust solution for managing, monitoring, and deploying applications with ease.

## ✨ Features

- **🚀 Performance**: Built with Rust for maximum speed and minimal resource usage.
- **🔄 Auto-Restart**: Automatically restarts processes if they crash.
- **📊 Monitoring**: Real-time monitoring of process health, memory, and CPU usage.
- **🌐 Clustering**: Support for multiple instances and load balancing (round-robin).
- **💓 Health Checks**: Built-in HTTP health checks to ensure your services are running correctly.
- **🚢 Deployment**: Streamlined deployment workflows over SSH.
- **🛠️ Extensible**: Plugin system support (including Minecraft plugin integration).

## 📁 Project Structure

This project is organized as a Rust workspace:

- `crates/tspm-core`: Core logic and shared utilities.
- `crates/tspm-engine`: The orchestration engine for managing processes.
- `crates/tspm-server`: The central management server.
- `crates/tspm-cli`: Command-line interface for interacting with TSPM.
- `crates/tspm-monitor`: Monitoring and metrics collection.
- `crates/tspm-events`: Event system for inter-process communication.
- `web-preact`: A modern web dashboard built with Preact.

## 🚀 Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Bun](https://bun.sh/) (optional, for running scripts)

### Installation

Clone the repository and build the project:

```bash
git clone https://github.com/tspm/tspm-rust.git
cd tspm-rust
cargo build --release
```

### Configuration

Create a `tspm.toml` file in your project root. Here is an example:

```toml
[defaults]
autorestart = true
max_restarts = 10

[[processes]]
name = "api-server"
script = "bun"
args = ["run", "src/index.ts"]
instances = 2
lb_strategy = "round-robin"
```

### Usage

Start the TSPM server:

```bash
tspm start
```

List all processes:

```bash
tspm list
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
