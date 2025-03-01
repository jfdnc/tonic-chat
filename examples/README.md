# Tonic Chat Examples

## Client Example

This directory contains an example client implementation that can connect to the Tonic Chat server.

### Building and Running the Client

From the `examples/client` directory:

```bash
cargo run -- --username YourName
```

#### Command-line Options:

- `--username` or `-u`: Set your username (default: "anonymous")
- `--server` or `-s`: Set the server address (default: "http://[::1]:50051")

Example with custom server address:
```bash
cargo run -- --username Alice --server http://192.168.1.100:50051
```

### Usage

1. Start the chat server in one terminal (from the project root):
   ```bash
   cargo run --release
   ```

2. Start one or more clients in separate terminals:
   ```bash
   cd examples/client
   cargo run -- --username Alice
   ```

   ```bash
   cd examples/client
   cargo run -- --username Bob
   ```

3. Type messages in any client terminal and press Enter to send
4. Type `quit` to exit the client

### Features

- Colored output to distinguish between your messages and others
- Command-line argument parsing for username and server address
- Clean disconnection handling
