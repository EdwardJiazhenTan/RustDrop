# RustDrop

RustDrop is a cross-platform local network file transfer tool built with Rust. It allows you to easily transfer files between devices on the same local network without requiring any login or third-party services.

## Features

- **Cross-platform**: Works on iPhone, Mac, Linux, Windows
- **Web UI**: Simple and intuitive web interface
- **CLI**: Command-line interface for power users
- **Zeroconf/mDNS**: Automatic device discovery on the local network
- **QR Code**: Scan to connect from mobile devices
- **No Login Required**: Works without any authentication or third-party services
- **Fast**: Built with Rust for high performance

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/rustdrop.git
cd rustdrop

# Build the project
cargo build --release

# Run the application
./target/release/rustdrop
```

### Using Cargo

```bash
cargo install rustdrop
```

## Usage

### Basic Usage

```bash
# Start RustDrop in the current directory
rustdrop

# Specify a port
rustdrop -p 8000

# Specify a directory to serve files from
rustdrop -d /path/to/directory

# Open web browser automatically
rustdrop -o
```

### Command Line Options

```
USAGE:
    rustdrop [OPTIONS]

OPTIONS:
    -p, --port <PORT>       Port to listen on [default: 8080]
    -d, --directory <DIR>   Directory to serve files from
    -o, --open              Open web browser automatically
    --no-mdns               Disable mDNS service discovery
    --no-qr                 Disable QR code display
    -h, --help              Print help information
    -V, --version           Print version information
```

## How It Works

1. RustDrop starts a web server on your device
2. It registers an mDNS service for discovery by other devices
3. You can access the web interface from any device on the same network
4. Upload and download files through the web interface
5. Discover other RustDrop instances on the network

## License

This project is licensed under the MIT License - see the LICENSE file for details.
