use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;
use tokio::time::{timeout, Duration};
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use hex;
use std::sync::Arc;

#[derive(Clone)]
pub struct SharedFile {
    full_path: PathBuf,
    name: String,
    size: u64,
}

#[derive(Clone)]
pub struct Peer {
    pub ip: String,
    pub port: u16,
    pub shared_files: Vec<SharedFile>,
    pub name: String,
}

impl Peer {
    pub fn new(ip: String, port: u16, shared_files: Vec<String>, name: String) -> Self {
        let shared_files = shared_files.into_iter()
            .filter_map(|path| {
                let path_buf = PathBuf::from(&path);
                if let Ok(metadata) = std::fs::metadata(&path_buf) {
                    Some(SharedFile {
                        name: path_buf.file_name()?.to_string_lossy().to_string(),
                        full_path: path_buf,
                        size: metadata.len(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Self {
            ip,
            port,
            shared_files,
            name,
        }
    }

    fn calculate_checksum(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result)
    }

    async fn send_file_in_blocks(&self, file_path: &Path, socket: &mut TcpStream) -> std::io::Result<()> {
        let file_size = file_path.metadata()?.len();
        println!("Iniciando envio do arquivo: {} ({} bytes)", file_path.display(), file_size);
    
        let size_header = format!("SIZE {}\n", file_size);
        socket.write_all(size_header.as_bytes()).await?;
        socket.flush().await?;
    
        let mut file = File::open(file_path).await?;
        let mut buffer = vec![0; 1024 * 64];
        let mut total_sent = 0;
        let mut ack_buffer = [0u8; 3];
    
        while total_sent < file_size {
            let n = file.read(&mut buffer).await?;
            if n == 0 { break; }
    
            let block_data = &buffer[..n];
            let checksum = Self::calculate_checksum(block_data);
    
            let header = format!("BLOCK {} {}\n", n, checksum);
            socket.write_all(header.as_bytes()).await?;
            socket.flush().await?;
    
            socket.write_all(block_data).await?;
            socket.flush().await?;
            total_sent += n as u64;
            println!("Enviados {}/{} bytes", total_sent, file_size);
    
            // Aguarda confirmação
            let ack = socket.read(&mut ack_buffer).await?;
            if &ack_buffer != b"ACK" {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "Erro na confirmação do bloco"));
            }
        }
    
        socket.write_all(b"END\n").await?;
        socket.flush().await?;
        println!("Envio completo! Total enviado: {} bytes", total_sent);
        Ok(())
    }

    async fn receive_file_in_blocks(&self, file_path: &str, socket: &mut TcpStream) -> std::io::Result<()> {
        let mut buffer = String::new();
        let mut temp_buffer = [0u8; 1024];
        let read_timeout = Duration::from_secs(30);
    
        loop {
            let n = timeout(read_timeout, socket.read(&mut temp_buffer)).await??;
            buffer.push_str(&String::from_utf8_lossy(&temp_buffer[..n]));
    
            if buffer.contains('\n') {
                break;
            }
        }
    
        let size_line = buffer
            .lines()
            .next()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid size header"))?;
    
        let file_size: u64 = size_line
            .strip_prefix("SIZE ")
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid size format"))?
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    
        println!("Starting file reception: {} ({} bytes expected)", file_path, file_size);
    
        let mut file = File::create(file_path).await?;
        let mut total_received = 0;
        let mut data_buffer = vec![0; 1024 * 64];
        let mut header_buffer = String::new();
    
        while total_received < file_size {
            header_buffer.clear();
            loop {
                let n = timeout(read_timeout, socket.read(&mut temp_buffer)).await??;
                if n == 0 {
                    println!("Conexão fechada prematuramente ao ler o cabeçalho do bloco.");
                    return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Conexão fechada prematuramente"));
                }
    
                header_buffer.push_str(&String::from_utf8_lossy(&temp_buffer[..n]));
    
                if header_buffer.contains('\n') {
                    break;
                }
            }
    
            let header_line = header_buffer
                .lines()
                .next()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid block header"))?;
    
            if header_line.trim() == "END" {
                break;
            } else if header_line.trim() == "ERROR" {
                println!("Erro ao receber o arquivo do peer.");
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Erro no peer ao enviar o arquivo"));
            } else if header_line.trim() == "FILE_NOT_FOUND" {
                println!("Arquivo não encontrado no peer.");
                return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Arquivo não encontrado no peer"));
            }
    
            let parts: Vec<&str> = header_line.split_whitespace().collect();
            if parts.len() != 3 || parts[0] != "BLOCK" {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid block format"));
            }
    
            let block_size: usize = parts[1].parse().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?;
            let expected_checksum = parts[2];
    
            let mut received = 0;
            while received < block_size {
                let n = timeout(read_timeout, socket.read(&mut data_buffer[received..block_size])).await??;
                if n == 0 {
                    return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Premature connection close"));
                }
                received += n;
            }
    
            let block_data = &data_buffer[..block_size];
            let calculated_checksum = Self::calculate_checksum(block_data);
    
            if calculated_checksum == expected_checksum {
                file.write_all(block_data).await?;
                total_received += block_size as u64;
                println!("Received {}/{} bytes", total_received, file_size);
                
                // Envia confirmação
                socket.write_all(b"ACK").await?;
                socket.flush().await?;
            } else {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid block checksum"));
            }
        }
    
        println!("Download complete! Total received: {} bytes", total_received);
        Ok(())
    }
    pub async fn list_peer_files(&self, peer_addr: &str) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(peer_addr).await?;
        stream.write_all(b"LIST_FILES").await?;
        
        let mut buffer = [0; 4096];
        let n = stream.read(&mut buffer).await?;
        let files_str = String::from_utf8_lossy(&buffer[..n]).to_string();
        
        let files = files_str
            .split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| {
                let parts: Vec<&str> = s.split('|').collect();
                if parts.len() == 2 {
                    Some((
                        parts[0].to_string(),
                        parts[1].parse::<u64>().unwrap_or(0)
                    ))
                } else {
                    None
                }
            })
            .collect();
            
        Ok(files)
    }

    pub async fn list_network_files(&self, peers: Vec<String>) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let mut network_files = Vec::new();
        
        for peer in peers {
            if peer != format!("{}:{}", self.ip, self.port) {
                match self.list_peer_files(&peer).await {
                    Ok(files) => {
                        for (file_name, _) in files {
                            network_files.push((file_name, peer.clone()));
                        }
                    }
                    Err(e) => println!("Erro ao listar arquivos do peer {}: {}", peer, e)
                }
            }
        }
        
        Ok(network_files)
    }

    pub async fn download_blocks_from_peers(&self, peers: Vec<String>, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        for peer in peers {
            if peer != format!("{}:{}", self.ip, self.port) {
                println!("Tentando baixar {} do peer {}", file_name, peer);
                
                if let Ok(mut socket) = TcpStream::connect(&peer).await {
                    let request = format!("REQUEST_FILE {}", file_name);
                    socket.write_all(request.as_bytes()).await?;

                    // Cria diretório de downloads se não existir
                    let download_dir = dirs::download_dir()
                        .unwrap_or_else(|| PathBuf::from("downloads"));
                    tokio::fs::create_dir_all(&download_dir).await?;

                    let download_path = download_dir.join(file_name);
                    match self.receive_file_in_blocks(download_path.to_str().unwrap(), &mut socket).await {
                        Ok(_) => {
                            println!("Download concluído com sucesso!");
                            return Ok(());
                        },
                        Err(e) => println!("Erro no download de {}: {}", peer, e)
                    }
                }
            }
        }
        
        Err("Não foi possível baixar o arquivo de nenhum peer".into())
    }

    pub async fn start_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, self.port)).await?;
        println!("Peer rodando em {}:{}", self.ip, self.port);

        loop {
            let (mut socket, _) = listener.accept().await?;
            let shared_files = self.shared_files.clone();
            let peer_clone = self.clone();
            
            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                if let Ok(n) = socket.read(&mut buffer).await {
                    let request = String::from_utf8_lossy(&buffer[..n]).to_string();

                    if request.starts_with("LIST_FILES") {
                        // Envia lista de arquivos com nome e tamanho
                        let file_list: Vec<String> = shared_files
                            .iter()
                            .map(|sf| format!("{}|{}", sf.name, sf.size))
                            .collect();
                        
                        let response = file_list.join(",");
                        socket.write_all(response.as_bytes()).await.unwrap_or_else(|e| {
                            println!("Erro ao enviar lista de arquivos: {}", e);
                        });
                    } else if request.starts_with("REQUEST_FILE") {
                        let requested_name = request.split_whitespace().nth(1).unwrap_or("");
                        println!("Requisição de download recebida para o arquivo: {}", requested_name);
                        
                        // Procura o arquivo pelo nome
                        if let Some(shared_file) = shared_files.iter().find(|sf| sf.name == requested_name) {
                            println!("Arquivo encontrado: {}", shared_file.full_path.display());
                            match peer_clone.send_file_in_blocks(&shared_file.full_path, &mut socket).await {
                                Ok(_) => println!("Arquivo {} enviado com sucesso", shared_file.name),
                                Err(e) => {
                                    println!("Erro ao enviar arquivo {}: {}", shared_file.name, e);
                                    socket.write_all(b"ERROR\n").await.ok(); // Envia um aviso de erro
                                }
                            }
                        } else  {
                            println!("Arquivo {} não encontrado", requested_name);
                            socket.write_all(b"FILE_NOT_FOUND\n").await.ok(); // Envia um aviso de arquivo não encontrado
                        }
                    }
                }
            });
        }
    }

    pub async fn register_with_tracker(&self, tracker_ip: &str, tracker_port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", tracker_ip, tracker_port)).await?;
        let message = format!("REGISTER {}:{}:{}", self.name, self.ip, self.port);
        stream.write_all(message.as_bytes()).await?;
        println!("Registrado no tracker {}:{}", tracker_ip, tracker_port);
        Ok(())
    }

    pub async fn unregister_from_tracker(&self, tracker_ip: &str, tracker_port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", tracker_ip, tracker_port)).await?;
        let message = format!("UNREGISTER {}:{}", self.ip, self.port);
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
}

pub fn list_local_files(directory: Option<&str>) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    
    let directories = match directory {
        Some(dir) => vec![PathBuf::from(dir)],
        None => vec![
            dirs::home_dir().unwrap_or_default().join("Documents"),
            dirs::home_dir().unwrap_or_default().join("Downloads"),
        ]
    };

    for dir in directories {
        if let Ok(entries) = read_dir(&dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Ok(file_name) = entry.file_name().into_string() {
                            files.push((file_name, entry.path()));
                        }
                    }
                }
            }
        }
    }
    
    files
}