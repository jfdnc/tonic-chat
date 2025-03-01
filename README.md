# Tonic Chat

A real-time chat application built with Rust and gRPC.

## Quick Start

### Server
```bash
cd server
cargo run
```

### Client
```bash
cd client
cargo run -- --username YourName
```

## Project Structure

- `proto/` - Protocol Buffers definitions
- `server/` - Server implementation
- `client/` - Client implementation

## Using the Chat Client

### Launch Options

- `--username` or `-u`: Set your username (default: "anonymous")
- `--server` or `-s`: Set server address (default: "http://[::1]:50051")

```bash
cargo run -- --username Alice --server http://localhost:50051
```

### Chat Commands

| Command | Description |
|---------|-------------|
| `/quit` | Exit the application |
| `/help` | Show available commands |
| `/nick <name>` | Change your username |
| `/users` | List connected users |
| `/clear` | Clear the screen |

### Chat Interface

- Type messages and press Enter to send
- Your messages appear in blue, others' in green
- System messages appear in yellow

## Development

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.65+)
- [Protocol Buffers compiler](https://grpc.io/docs/protoc-installation/)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/tonic-chat.git
cd tonic-chat

# Build the project
cargo build --release
```

### Running Tests

```bash
cargo test
```

## Features

- Real-time messaging with gRPC streaming
- User presence tracking
- Username customization
- Command-based interface
- Cross-platform support
