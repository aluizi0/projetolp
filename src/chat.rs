/// Importa `TcpListener` e `TcpStream` do Tokio para gerenciar conexões TCP assíncronas.
use tokio::net::{TcpListener, TcpStream};

/// Importa `AsyncReadExt` e `AsyncWriteExt` do Tokio para leitura e escrita assíncronas.
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Importa `Arc` do módulo `std::sync` para criar ponteiros inteligentes de contagem de referência.
use std::sync::Arc;

/// Importa `Mutex` e `mpsc` do Tokio para sincronização e comunicação entre tarefas assíncronas.
use tokio::sync::{Mutex, mpsc};

/// Importa `io` do módulo `std` para operações de entrada e saída.
use std::io;

/// Estrutura que representa o servidor de chat.
#[derive(Clone)]
pub struct ChatServer {
    sender: Arc<Mutex<mpsc::Sender<(String, String)>>>,
}

impl ChatServer {
    /// Cria uma nova instância do servidor de chat.
    ///
    /// # Argumentos
    ///
    /// * `sender` - Canal para enviar mensagens.
    pub fn new(sender: mpsc::Sender<(String, String)>) -> Self {
        Self {
            sender: Arc::new(Mutex::new(sender)),
        }
    }

    /// Inicia o servidor de chat na porta especificada.
    ///
    /// # Argumentos
    ///
    /// * `port` - Porta na qual o servidor de chat será iniciado.
    ///
    /// # Retornos
    ///
    /// Retorna um `Result` indicando sucesso ou erro.
    pub async fn start_chat_server(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        println!("Servidor de chat iniciado na porta {}", port);

        loop {
            let (mut socket, _) = listener.accept().await?;
            let sender = Arc::clone(&self.sender);

            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                loop {
                    let n = match socket.read(&mut buffer).await {
                        Ok(n) if n == 0 => return,
                        Ok(n) => n,
                        Err(_) => return,
                    };

                    let message = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let sender = sender.lock().await;
                    if let Err(_) = sender.send((socket.peer_addr().unwrap().to_string(), message)).await {
                        return;
                    }
                }
            });
        }
    }
}

/// Inicia o cliente de chat e conecta a um peer na porta especificada.
///
/// # Argumentos
///
/// * `peer_name` - Nome do peer.
/// * `target_port` - Porta do peer alvo.
///
/// # Retornos
///
/// Retorna um `Result` indicando sucesso ou erro.
pub async fn start_chat_client(peer_name: String, target_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", target_port)).await?;
    println!("Conectado ao chat na porta {}", target_port);
    println!("Digite sua mensagem:");

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

/// Recebe mensagens do canal e as imprime no console.
///
/// # Argumentos
///
/// * `mut receiver` - Canal para receber mensagens.
///
/// # Retornos
///
/// Retorna um `Result` indicando sucesso ou erro.
pub async fn message_receiver(mut receiver: mpsc::Receiver<(String, String)>) -> Result<(), Box<dyn std::error::Error>> {
    while let Some((peer, message)) = receiver.recv().await {
        println!("{}: {}", peer, message);
    }
    Ok(())
}