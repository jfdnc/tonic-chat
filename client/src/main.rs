use std::error::Error;
use std::sync::Arc;
use std::io::Write;
use std::collections::HashMap;

use clap::Parser;
use colored::Colorize;
use futures_util::StreamExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, oneshot, Mutex as TokioMutex};
use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;

// Import the generated proto code
// Note: This assumes the client and server share the same proto definitions
pub mod chat {
    tonic::include_proto!("chat");
}

use chat::{chat_client::ChatClient, ChatMessage, ListUsersRequest, User};

// Command handler structures
type CommandFn = Arc<dyn Fn(&str, &CommandContext) -> CommandResult + Send + Sync>;

struct Command {
    name: &'static str,
    description: &'static str,
    handler: CommandFn,
}

struct CommandHandler {
    commands: HashMap<String, Command>,
}

enum CommandResult {
    Continue,
    Quit,
}

struct CommandContext {
    username: Arc<TokioMutex<String>>,
    tx: mpsc::Sender<ChatMessage>,
    prompt_tx: mpsc::Sender<()>,
    client: Arc<TokioMutex<ChatClient<tonic::transport::Channel>>>,
}

impl CommandHandler {
    fn new() -> Self {
        let mut handler = Self {
            commands: HashMap::new(),
        };
        handler.register_default_commands();
        handler
    }

    fn register(&mut self, command: Command) {
        self.commands.insert(command.name.to_string(), command);
    }

    fn register_default_commands(&mut self) {
        // Quit command
        self.register(Command {
            name: "quit",
            description: "Exit the chat application",
            handler: Arc::new(|_, _| CommandResult::Quit),
        });

        // Help command
        self.register(Command {
            name: "help",
            description: "Show available commands",
            handler: Arc::new(|_, ctx| {
                println!("\r{}", "Available Commands:".yellow().bold());
                println!("{}", "━".repeat(60).dimmed());
                
                // This would normally list all commands from the handler
                // For simplicity in this example, we hard-code the list
                println!("  {} - Exit the chat application", "/quit".cyan());
                println!("  {} - Show available commands", "/help".cyan());
                println!("  {} - Change your username", "/nick <new_username>".cyan());
                println!("  {} - Clear the screen", "/clear".cyan());
                println!("  {} - List connected users", "/users".cyan());
                println!("{}", "━".repeat(60).dimmed());
                
                let _ = ctx.prompt_tx.try_send(());
                CommandResult::Continue
            }),
        });

        // Nick command - fix lifetime issues by cloning
        self.register(Command {
            name: "nick",
            description: "Change your username",
            handler: Arc::new(|args, ctx| {
                // Clone args to own the data
                let new_name = args.to_string();
                
                if new_name.trim().is_empty() {
                    println!("\r{}", "Usage: /nick <new_username>".yellow());
                } else {
                    // Clone what we need from ctx for the async block
                    let username_arc = ctx.username.clone();
                    let prompt_tx = ctx.prompt_tx.clone();
                    let trimmed_name = new_name.trim().to_string();
                    
                    tokio::spawn(async move {
                        let mut username = username_arc.lock().await;
                        *username = trimmed_name.clone();
                        println!("\r{} {}", "Username changed to:".green(), trimmed_name.cyan().bold());
                        let _ = prompt_tx.send(()).await;
                    });
                    
                    // Return early, the spawned task will handle the prompt redisplay
                    return CommandResult::Continue;
                }
                
                let _ = ctx.prompt_tx.try_send(());
                CommandResult::Continue
            }),
        });

        // Clear command
        self.register(Command {
            name: "clear",
            description: "Clear the screen",
            handler: Arc::new(|_, ctx| {
                // ANSI escape code to clear screen and move cursor to home position
                print!("\x1B[2J\x1B[1;1H");
                let _ = ctx.prompt_tx.try_send(());
                CommandResult::Continue
            }),
        });

        // Users command - list all connected users
        self.register(Command {
            name: "users",
            description: "List all connected users",
            handler: Arc::new(|_, ctx| {
                // Clone what we need for the async block
                let client = ctx.client.clone();
                let prompt_tx = ctx.prompt_tx.clone();
                
                tokio::spawn(async move {
                    println!("\r{}", "Fetching connected users...".yellow());
                    
                    match client.lock().await.list_users(Request::new(ListUsersRequest {})).await {
                        Ok(response) => {
                            let users = response.into_inner().users;
                            
                            println!("\r{} {}", "Connected Users:".green().bold(), 
                                     format!("({})", users.len()).cyan());
                            println!("{}", "━".repeat(60).dimmed());
                            
                            if users.is_empty() {
                                println!("  No other users connected");
                            } else {
                                for (i, user) in users.iter().enumerate() {
                                    println!("  {}. {}", i + 1, user.username.green());
                                }
                            }
                            
                            println!("{}", "━".repeat(60).dimmed());
                        },
                        Err(e) => {
                            println!("\r{} {}", "Failed to fetch users:".red(), e);
                        }
                    }
                    
                    let _ = prompt_tx.send(()).await;
                });
                
                CommandResult::Continue
            }),
        });
    }

    fn execute(&self, input: &str, ctx: &CommandContext) -> CommandResult {
        if !input.starts_with('/') {
            return CommandResult::Continue;
        }

        // Parse command and arguments
        let input = input.trim_start_matches('/');
        let mut parts = input.splitn(2, ' ');
        let command_name = parts.next().unwrap_or("");
        let args = parts.next().unwrap_or("");

        // Execute command if found
        if let Some(command) = self.commands.get(command_name) {
            (command.handler)(args, ctx)
        } else {
            println!("\r{} {}", "Unknown command:".red(), format!("/{}", command_name).yellow());
            let _ = ctx.prompt_tx.try_send(());
            CommandResult::Continue
        }
    }
}

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
    println!("Type messages and press Enter to send. Type {} to exit.", "/quit".yellow());
    println!("Type {} for available commands.", "/help".cyan());
    println!("{}", "━".repeat(60).dimmed());

    // Store username in a thread-safe way that can be modified
    let username = Arc::new(TokioMutex::new(args.username.clone()));
    let username_for_prompt = username.clone();
    let username_for_receive = username.clone();

    // Connect to the server
    let mut client = ChatClient::connect(args.server).await?;
    // Store client in an Arc<Mutex> so it can be safely shared with command handlers
    let client_arc = Arc::new(TokioMutex::new(client.clone()));
    
    println!("{}", "Connected to chat server!".green());
    println!("{}", "━".repeat(60).dimmed());
    
    // Display initial prompt
    let init_username = username_for_prompt.lock().await.clone();
    display_prompt(&init_username);

    // Create channel for sending messages
    let (tx, rx) = mpsc::channel(32);
    let outgoing_stream = ReceiverStream::new(rx);

    // Set up the bidirectional stream
    let response = client.chat_stream(Request::new(outgoing_stream)).await?;
    let mut incoming_stream = response.into_inner();

    // Clone tx for the input handling task
    let tx_input = tx.clone();

    // Send an initial "join" message to establish username on the server
    let join_username = username.lock().await.clone();
    let join_message = ChatMessage {
        username: join_username.clone(),
        message: format!("has joined the chat"),
    };
    tx_input.send(join_message).await?;
    println!("{}: {}", "System".yellow(), format!("You've joined as {}", join_username.cyan()));

    // Channel for signaling program termination
    let (quit_tx, quit_rx) = oneshot::channel::<()>();
    let quit_tx = Arc::new(std::sync::Mutex::new(Some(quit_tx)));

    // Create a channel for signaling when to redisplay the prompt
    let (prompt_tx, mut prompt_rx) = mpsc::channel::<()>(10);
    let prompt_tx_clone = prompt_tx.clone();

    // Create command context
    let command_ctx = CommandContext {
        username: username.clone(),
        tx: tx_input.clone(),
        prompt_tx: prompt_tx.clone(),
        client: client_arc.clone(),
    };

    // Create command handler
    let command_handler = Arc::new(CommandHandler::new());
    let command_handler_clone = command_handler.clone();

    // Spawn a task to handle user input using tokio's async I/O
    let input_task = tokio::spawn(async move {
        let mut stdin = BufReader::new(tokio::io::stdin()).lines();
        
        while let Ok(Some(line)) = stdin.next_line().await {
            // Clear the current line for better UX when messages arrive during input
            print!("\r{}", " ".repeat(100));
            
            let trimmed = line.trim();
            if trimmed.starts_with('/') {
                // Handle command
                match command_handler_clone.execute(trimmed, &command_ctx) {
                    CommandResult::Quit => {
                        println!("\r{}", "Disconnecting from chat...".yellow());
                        if let Some(quit_tx) = quit_tx.lock().unwrap().take() {
                            let _ = quit_tx.send(());
                        }
                        break;
                    }
                    CommandResult::Continue => continue,
                }
            } else if !trimmed.is_empty() {
                // Send regular message
                let current_username = username.lock().await.clone();
                let message = ChatMessage {
                    username: current_username,
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
                    let current_username = username_for_receive.lock().await.clone();
                    if (msg.username == current_username) {
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
            let current_username = username_for_prompt.lock().await.clone();
            display_prompt(&current_username);
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
