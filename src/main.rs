use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tonic::transport::Server;

// Import the generated proto code
pub mod chat {
    tonic::include_proto!("chat");
}

use chat::{ChatMessage, chat_server::{Chat, ChatServer}};

// Type aliases for better readability
type ResponseStream = Pin<Box<dyn futures_util::Stream<Item = Result<ChatMessage, Status>> + Send>>;
type Broadcaster = mpsc::Sender<Result<ChatMessage, Status>>;
type ClientMap = Arc<Mutex<HashMap<String, Broadcaster>>>;

#[derive(Debug)]
struct ChatService {
    clients: ClientMap,
}

impl ChatService {
    fn new() -> Self {
        ChatService {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    // Helper method to broadcast a message to all connected clients
    // Now a static method that takes the client map as an argument
    fn broadcast(clients: &ClientMap, message: ChatMessage) {
        let clients = clients.lock().unwrap();
        
        for (client_id, tx) in clients.iter() {
            match tx.try_send(Ok(message.clone())) {
                Ok(_) => println!("Message sent to client {}", client_id),
                Err(e) => println!("Failed to send message to client {}: {:?}", client_id, e),
            }
        }
    }
}

#[tonic::async_trait]
impl Chat for ChatService {
    type ChatStreamStream = ResponseStream;

    async fn chat_stream(
        &self,
        request: Request<tonic::Streaming<ChatMessage>>,
    ) -> Result<Response<Self::ChatStreamStream>, Status> {
        println!("New client connected: {:?}", request.remote_addr());
        
        // Generate a unique client ID
        let client_id = format!("{:?}", SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos());
            
        let mut request_stream = request.into_inner();
        
        // Create a channel for this client
        let (tx, rx) = mpsc::channel(100);
        let rx_stream = ReceiverStream::new(rx);
        
        // Store the sender in our clients map
        {
            let mut clients = self.clients.lock().unwrap();
            clients.insert(client_id.clone(), tx.clone());
        }
        
        // Clone the clients map for the task
        let clients_for_task = self.clients.clone();
        
        // Spawn a task to process incoming messages from this client
        tokio::spawn(async move {
            while let Some(result) = request_stream.next().await {
                match result {
                    Ok(msg) => {
                        println!("Received message from {}: {:?}", client_id, msg);
                        
                        // Broadcast the message to all clients
                        ChatService::broadcast(&clients_for_task, msg);
                    }
                    Err(e) => {
                        println!("Error receiving message: {:?}", e);
                        break;
                    }
                }
            }
            
            // Remove the client when they disconnect
            let mut clients = clients_for_task.lock().unwrap();
            clients.remove(&client_id);
            println!("Client disconnected: {}", client_id);
        });
        
        // Return the receiver as a stream to the client
        Ok(Response::new(Box::pin(rx_stream) as ResponseStream))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let chat_service = ChatService::new();
    
    println!("Chat server starting on {}", addr);
    
    Server::builder()
        .add_service(ChatServer::new(chat_service))
        .serve(addr)
        .await?;
    
    Ok(())
}
