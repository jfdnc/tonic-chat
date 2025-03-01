use std::error::Error;
use std::sync::Arc;
use std::io::Write;

use clap::Parser;
use colored::Colorize;
use futures_util::StreamExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;

// Import the generated proto code
// Note: This assumes the client and server share the same proto definitions
pub mod chat {
    tonic::include_proto!("chat");
}

use chat::{chat_client::ChatClient, ChatMessage};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Your username for the chat
    #[arg(short, long, default_value = "anonymous")]
    username: String,

    /// Server address to connect to
    #[arg(short, long, default_value = "http://[::1]:50051")]
    server: String,
}

// Function to display the chat prompt
fn display_prompt(username: &str) {
    print!("{} {} ", format!("[{}]", username).green().bold(), "→".cyan().bold());
    std::io::stdout().flush().unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    println!("Connecting to chat server: {}", args.server);
    println!("Username: {}", args.username);
    println!("Type messages and press Enter to send. Type {} to exit.", "quit".yellow());
    println!("{}", "━".repeat(60).dimmed());

    // Clone the username upfront for use in different tasks
    let username_for_input = args.username.clone();
    let username_for_receive = args.username.clone();
    let username_for_prompt = args.username.clone();

    // Connect to the server
    let mut client = ChatClient::connect(args.server).await?;
    println!("{}", "Connected to chat server!".green());
    println!("{}", "━".repeat(60).dimmed());
    
    // Display initial prompt
    display_prompt(&username_for_prompt);

    // Create channel for sending messages
    let (tx, rx) = mpsc::channel(32);
    let outgoing_stream = ReceiverStream::new(rx);

    // Set up the bidirectional stream
    let response = client.chat_stream(Request::new(outgoing_stream)).await?;
    let mut incoming_stream = response.into_inner();

    // Clone tx for the input handling task
    let tx_input = tx.clone();

    // Channel for signaling program termination
    let (quit_tx, quit_rx) = oneshot::channel::<()>();
    let quit_tx = Arc::new(std::sync::Mutex::new(Some(quit_tx)));

    // Create a channel for signaling when to redisplay the prompt
    let (prompt_tx, mut prompt_rx) = mpsc::channel::<()>(10);
    let prompt_tx_clone = prompt_tx.clone();

    // Spawn a task to handle user input using tokio's async I/O
    let input_task = tokio::spawn(async move {
        let mut stdin = BufReader::new(tokio::io::stdin()).lines();
        
        while let Ok(Some(line)) = stdin.next_line().await {
            // Clear the current line for better UX when messages arrive during input
            print!("\r{}", " ".repeat(100));
            
            if line.trim() == "quit" {
                println!("\r{}", "Disconnecting from chat...".yellow());
                if let Some(quit_tx) = quit_tx.lock().unwrap().take() {
                    let _ = quit_tx.send(());
                }
                break;
            }

            if !line.trim().is_empty() {
                let message = ChatMessage {
                    username: username_for_input.clone(),
                    message: line,
                };

                if let Err(e) = tx_input.send(message).await {
                    eprintln!("Failed to send message: {}", e);
                    break;
                }
            } else {
                // Redisplay prompt for empty messages
                let _ = prompt_tx.send(()).await;
            }
        }
    });

    // Task to receive messages from the server
    let receive_task = tokio::spawn(async move {
        while let Some(result) = incoming_stream.next().await {
            match result {
                Ok(msg) => {
                    // Clear the current line (in case user is typing)
                    print!("\r{}\r", " ".repeat(100));
                    
                    // Print the message differently based on whether it's from the current user
                    if msg.username == username_for_receive {
                        println!("{}: {}", "You".blue().bold(), msg.message);
                    } else {
                        println!("{}: {}", msg.username.green().bold(), msg.message);
                    }
                    
                    // Redisplay the prompt after printing a message
                    let _ = prompt_tx_clone.send(()).await;
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                    break;
                }
            }
        }
    });

    // Prompt redisplay task
    let prompt_task = tokio::spawn(async move {
        while prompt_rx.recv().await.is_some() {
            display_prompt(&username_for_prompt);
        }
    });

    // Wait for quit signal or error in any task
    tokio::select! {
        _ = quit_rx => {
            println!("Exiting chat client...");
        }
        _ = receive_task => {
            println!("Server connection closed");
        }
        _ = input_task => {
            println!("Input handling ended");
        }
        _ = prompt_task => {
            println!("Prompt handling ended");
        }
    }

    Ok(())
}
