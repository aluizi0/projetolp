use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;


use std::fs::read_dir;

#[derive(Clone)]
pub struct Peer {
    pub ip: String,
    pub port: u16,
    pub shared_files: Vec<String>,
    pub name: String,
}

impl Peer {
    pub fn new(ip: String, port: u16, shared_files: Vec<String>, name: String) -> Self {
        Self {
            ip,
            port,
            shared_files,
            name,
        }
    }

    pub async fn register_with_tracker(&self, tracker_ip: &str, tracker_port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", tracker_ip, tracker_port)).await?;
        let message = format!("REGISTER {}:{}", self.name, self.port);
        stream.write_all(message.as_bytes()).await?;
        println!("Registrado no tracker {}:{}", tracker_ip, tracker_port);
        Ok(())
    }

    pub async fn unregister_from_tracker(&self, tracker_ip: &str, tracker_port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", tracker_ip, tracker_port)).await?;
        let message = format!("UNREGISTER {}:{}", self.name, self.port);
        stream.write_all(message.as_bytes()).await?;
        println!("Desregistrado do tracker {}:{}", tracker_ip, tracker_port);
        Ok(())
    }

    pub async fn get_peers_from_tracker(&self, tracker_ip: &str, tracker_port: u16) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", tracker_ip, tracker_port)).await?;
        stream.write_all(b"GET_PEERS").await?;

        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await?;
        let peer_list = String::from_utf8_lossy(&buffer[..n]).to_string();
        let peers = peer_list.split(',').map(|s| s.to_string()).collect();
        Ok(peers)
    }

    

    

    pub async fn start_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, self.port)).await?;
        println!("Peer rodando em {}:{}", self.ip, self.port);

        loop {
            let (mut socket, _) = listener.accept().await?;
            let shared_files = self.shared_files.clone();

            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                if let Ok(n) = socket.read(&mut buffer).await {
                    let request = String::from_utf8_lossy(&buffer[..n]).to_string();

                    if request.starts_with("LIST_FILES") {
                        let file_list = shared_files.join(",");
                        socket.write_all(file_list.as_bytes()).await.unwrap();
                    }
                    
                    if request.starts_with("REQUEST_FILE") {
                        let filename = &request[13..];
                        if let Some(file_path) = shared_files.iter().find(|f| f.as_str() == filename) {
                            if let Ok(mut file) = File::open(file_path).await {
                                let mut file_buffer = vec![0; 1024];
                                while let Ok(bytes_read) = file.read(&mut file_buffer).await {
                                    if bytes_read == 0 {
                                        break;
                                    }
                                    socket.write_all(&file_buffer[..bytes_read]).await.unwrap();
                                }
                            }
                        }
                    }

                    if !request.is_empty() {
                        println!("Mensagem recebida: {}", request);
                    }
                }
            });
        }
    }
}

pub fn list_local_files(directory: &str) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = read_dir(directory) {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                files.push(file_name);
            }
        }
    }
    files
}