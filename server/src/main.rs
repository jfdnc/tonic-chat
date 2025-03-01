use std::{collections::HashMap, pin::Pin, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

// Generated protobuf code
pub mod chat {
    tonic::include_proto!("chat");
}

use chat::{
    chat_server::{Chat, ChatServer},
    ChatMessage, ListUsersRequest, ListUsersResponse, User,
};

/// Stream of chat messages sent from the server to clients
type ResponseStream =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<ChatMessage, Status>> + Send>>;

/// Channel for sending messages to a specific client
type ClientSender = mpsc::Sender<Result<ChatMessage, Status>>;

/// Shared state containing all connected clients
type ClientRegistry = Arc<Mutex<HashMap<String, (String, ClientSender)>>>;

struct ChatService {
    clients: ClientRegistry,
}

impl ChatService {
    fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Broadcasts a message to all connected clients
    async fn broadcast(clients: &ClientRegistry, message: ChatMessage) {
        let clients = clients.lock().await;
        
        for (client_id, (username, sender)) in clients.iter() {
            match sender.try_send(Ok(message.clone())) {
                Ok(_) => println!("Message sent to {} ({})", username, client_id),
                Err(e) => println!("Failed to send message to {} ({}): {:?}", username, client_id, e),
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
        let remote_addr = request.remote_addr()
            .map_or("unknown".to_string(), |addr| addr.to_string());
        println!("New client connected from {}", remote_addr);

        let client_id = Uuid::new_v4().to_string();
        let initial_username = format!("user_{}", &client_id[..6]);
        let mut inbound = request.into_inner();
        
        // Create channel for sending messages to this client
        let (tx, rx) = mpsc::channel(100);
        let clients = Arc::clone(&self.clients);

        // Register the new client
        {
            let mut client_map = self.clients.lock().await;
            client_map.insert(client_id.clone(), (initial_username.clone(), tx.clone()));
        }

        // Handle incoming messages from this client
        tokio::spawn(async move {
            let mut current_username = initial_username;
            
            // Process incoming messages until client disconnects or error occurs
            while let Some(result) = inbound.next().await {
                match result {
                    Ok(msg) => {
                        println!("Received from {} ({}): {:?}", msg.username, client_id, msg.message);

                        // Update username if changed
                        if current_username != msg.username {
                            println!("Username changed: {} → {}", current_username, msg.username);
                            if let Some((username, _)) = clients.lock().await.get_mut(&client_id) {
                                *username = msg.username.clone();
                                current_username = msg.username.clone();
                            }
                        }

                        // Relay message to all clients
                        ChatService::broadcast(&clients, msg).await;
                    }
                    Err(e) => {
                        println!("Error receiving message: {:?}", e);
                        break;
                    }
                }
            }
            
            // Clean up when client disconnects
            let mut client_map = clients.lock().await;
            if let Some((username, _)) = client_map.remove(&client_id) {
                println!("Client disconnected: {} ({})", username, client_id);
            } else {
                println!("Unknown client disconnected: {}", client_id);
            }
        });

        // Return stream of messages back to the client
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn list_users(
        &self,
        _request: Request<ListUsersRequest>,
    ) -> Result<Response<ListUsersResponse>, Status> {
        let client_map = self.clients.lock().await;
        
        let users = client_map
            .values()
            .map(|(username, _)| User {
                username: username.clone(),
            })
            .collect();
            
        Ok(Response::new(ListUsersResponse { users }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let chat_service = ChatService::new();

    println!("Chat server listening on {}", addr);

    Server::builder()
        .add_service(ChatServer::new(chat_service))
        .serve(addr)
        .await?;

    Ok(())
}
