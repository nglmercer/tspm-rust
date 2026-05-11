# TSPM (The Server Process Manager)

TSPM is a high-performance, lightweight process manager written in Rust. Inspired by tools like PM2, it provides a robust solution for managing, monitoring, and deploying applications with ease.

## ✨ Features

- **🚀 Performance**: Built with Rust for maximum speed and minimal resource usage.
- **🖥️ Dashboard**: Beautiful, real-time web dashboard embedded directly into the binary.
- **🔄 Auto-Restart**: Automatically restarts processes if they crash.
- **📊 Monitoring**: Real-time system metrics (CPU, Memory, Uptime) and per-process tracking.
- **🐚 Terminal**: Integrated **xterm.js** terminal for real-time command execution and log viewing.
- **🌐 Clustering**: Support for multiple instances and load balancing (round-robin).
- **💓 Health Checks**: Built-in HTTP health checks to ensure your services are running correctly.
- **🚢 Deployment**: Streamlined deployment workflows over SSH.
- **🛡️ Startup**: Native Systemd support to auto-run your processes on boot.

## 📁 Project Structure

This project is organized as a Rust workspace:

- `crates/tspm-core`: Core logic and shared utilities.
- `crates/tspm-engine`: The orchestration engine for managing processes.
- `crates/tspm-server`: The central management server with embedded web dashboard.
- `crates/tspm-cli`: Command-line interface for interacting with TSPM.
- `crates/tspm-monitor`: Monitoring and metrics collection.
- `crates/tspm-events`: Event system for inter-process communication.
- `web-preact`: A modern web dashboard built with Preact and Vite.

## 🚀 Getting Started

### Installation

Clone the repository and build the standalone binary:

```bash
git clone https://github.com/tspm/tspm-rust.git
cd tspm-rust
cargo build --release
```

The binary will be available at `target/release/tspm`.

### Usage

Start the TSPM dashboard:

```bash
./target/release/tspm dashboard
```

List all processes:

```bash
./target/release/tspm list
```

### Auto-run on Startup (Linux)

1. Save your current process list:
   ```bash
   ./target/release/tspm save
   ```
2. Install the Systemd service:
   ```bash
   sudo ./target/release/tspm startup systemd
   ```
3. Enable and start:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now tspm
   ```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
