use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use std::io;

#[derive(Clone)]
pub struct ChatServer {
    sender: Arc<Mutex<mpsc::Sender<(String, String)>>>,
}

impl ChatServer {
    pub fn new(sender: mpsc::Sender<(String, String)>) -> Self {
        Self {
            sender: Arc::new(Mutex::new(sender)),
        }
    }

    pub async fn start_chat_server(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        println!("Servidor de chat rodando na porta {}", port);

        loop {
            let (mut socket, _) = listener.accept().await?;
            let sender = Arc::clone(&self.sender);

            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                while let Ok(n) = socket.read(&mut buffer).await {
                    if n == 0 {
                        break; // Se a conexão for fechada, sai do loop
                    }

                    let message = String::from_utf8_lossy(&buffer[..n]).to_string();
                    // Envia a mensagem recebida para o canal
                    let sender = sender.lock().await;
                    sender.send((String::from("peer"), message)).await.unwrap();
                }
            });
        }
    }
}

pub async fn start_chat_client(target_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", target_port)).await?;
    println!("Conectado ao chat na porta {}", target_port);

    println!("Digite seu nome de peer novamente:");
    let mut peer_name = String::new();
    io::stdin().read_line(&mut peer_name).unwrap();
    let peer_name = peer_name.trim().to_string();

    loop {
        let mut message = String::new();
        io::stdin().read_line(&mut message).unwrap();

        if message.trim() == "exit" {
            break;
        }

        let full_message = format!("{}: {}", peer_name, message);
        stream.write_all(full_message.as_bytes()).await?;
    }

    Ok(())
}

pub async fn message_receiver(mut receiver: mpsc::Receiver<(String, String)>) {
    while let Some((peer_name, message)) = receiver.recv().await {
        println!("📩 Mensagem recebida de {}: {}", peer_name, message);
    }
}