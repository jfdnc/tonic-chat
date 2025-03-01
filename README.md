# Tonic Chat

A real-time chat application using gRPC (Tonic) and Tokio with bidirectional streaming.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.65 or newer)
- [Protocol Buffers compiler](https://grpc.io/docs/protoc-installation/) (required for compiling protocol buffers)
- [Docker](https://docs.docker.com/get-docker/) (optional, for containerized deployment)

## Getting Started

### Building the Server

Clone the repository and build the project:

```bash
git clone https://github.com/yourusername/tonic-chat.git
cd tonic-chat
cargo build --release
```

### Running the Server

Run the chat server:

```bash
cargo run --release
```

The server will start and listen on `[::1]:50051` (localhost on IPv6).

## Project Structure

```
tonic-chat/
├── Cargo.toml         # Project dependencies
├── build.rs           # Protocol buffer compilation setup
├── proto/
│   └── chat.proto     # gRPC service and message definitions
├── src/
│   └── main.rs        # Server implementation
├── examples/
│   └── client/        # Example client implementation
└── Dockerfile         # Docker build configuration
```

## Implementation Details

The application provides:

- A gRPC server using Tonic for handling connections
- Bidirectional streaming for real-time chat
- Broadcasting of messages to all connected clients
- Client connection tracking and management

## Using Docker

Build and run with Docker:

```bash
# Build the Docker image
docker build -t tonic-chat .

# Run the container
docker run -p 50051:50051 tonic-chat
```

## Testing the Chat Service

### Using the Example Client

This repository includes a fully functional chat client in the `examples/client` directory. This is the easiest way to test the chat service:

1. First, start the server in one terminal:
   ```bash
   # In the project root directory
   cargo run
   ```

2. Then, in a second terminal, start a client:
   ```bash
   # From the project root
   cd examples/client
   cargo run -- --username Alice
   ```

3. Open a third terminal for another client:
   ```bash
   cd examples/client
   cargo run -- --username Bob
   ```

4. Type messages in either client terminal and see them appear in both clients.
   
5. Type `quit` to gracefully exit the client.

#### Client Features

- **Interactive prompt** with colored output showing your username
- **Real-time message delivery** with bidirectional streaming
- **Command-line options**:
  - `--username` or `-u` - Set your chat username (default: "anonymous")
  - `--server` or `-s` - Set the server address (default: "http://[::1]:50051")
- **Visual differentiation** between your messages and those from others
- **Clean disconnection handling**

### Using grpcurl

You can also use [grpcurl](https://github.com/fullstorydev/grpcurl) to test the service:

```bash
# Install grpcurl (if you haven't already)
# Example command for sending a message (though bidirectional streaming is hard to test this way)
grpcurl -plaintext -d '{"username": "test_user", "message": "Hello world!"}' [::1]:50051 chat.Chat/ChatStream
```

### Using Custom Clients

You can implement your own client using any gRPC-compatible programming language. Here's a basic Rust client example:

```rust
use chat::{ChatMessage, chat_client::ChatClient};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ChatClient::connect("http://[::1]:50051").await?;
    
    // Create a channel for sending messages to server
    let (tx, rx) = mpsc::channel(32);
    let outbound = ReceiverStream::new(rx);
    
    // Establish bidirectional streaming connection
    let response = client.chat_stream(Request::new(outbound)).await?;
    let mut inbound = response.into_inner();
    
    // Send a test message
    tx.send(ChatMessage {
        username: "client_user".to_string(),
        message: "Hello from Rust client!".to_string(),
    }).await?;
    
    // Listen for incoming messages
    while let Some(message) = inbound.next().await {
        match message {
            Ok(msg) => {
                println!("{}: {}", msg.username, msg.message);
            }
            Err(e) => {
                println!("Error receiving message: {:?}", e);
                break;
            }
        }
    }
    
    Ok(())
}
```

## Future Improvements

- Authentication and user management
- Persistent chat history
- Private messaging
- Multiple chat rooms
- Web client integration

## License

[MIT License](LICENSE)
