use hex;
use sha2::{Digest, Sha256};
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::io;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

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
        let shared_files = shared_files
            .into_iter()
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

    async fn send_file_in_blocks(&self, file_path: &Path, socket: &mut TcpStream) -> io::Result<()> {
        let file_size = file_path.metadata()?.len();
        println!("Iniciando envio do arquivo: {} ({} bytes)", file_path.display(), file_size);

        // Envia o tamanho do arquivo
        let size_header = format!("SIZE {}\n", file_size);
        socket.write_all(size_header.as_bytes()).await?;
        socket.flush().await?;

        // Abre e envia o arquivo
        let mut file = File::open(file_path).await?;
        let mut buffer = vec![0u8; 64 * 1024]; // Buffer de 64KB
        let mut total_sent = 0;

        while total_sent < file_size {
            // Lê um bloco do arquivo
            let n = file.read(&mut buffer).await?;
            if n == 0 { break; }

            // Envia o bloco
            let mut bytes_written = 0;
            while bytes_written < n {
                let written = socket.write(&buffer[bytes_written..n]).await?;
                if written == 0 {
                    return Err(io::Error::new(io::ErrorKind::WriteZero, 
                        "Falha ao escrever no socket"));
                }
                bytes_written += written;
            }

            socket.flush().await?;
            total_sent += n as u64;

            // Atualiza o progresso
            println!("Enviados {}/{} bytes ({:.1}%)", 
                total_sent, 
                file_size, 
                (total_sent as f64 / file_size as f64) * 100.0);
        }

        println!("Envio completo! Total enviado: {} bytes", total_sent);
        Ok(())
    }
    
    async fn receive_file_in_blocks(&self, file_path: &str, socket: &mut TcpStream) -> io::Result<()> {
        let read_timeout = Duration::from_secs(30);
        
        // Lê cabeçalho de tamanho
        let mut size_buffer = [0u8; 1024];
        let n = socket.read(&mut size_buffer).await?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Conexão fechada prematuramente"));
        }
        
        let size_str = String::from_utf8_lossy(&size_buffer[..n]);
        let size_line = size_str.lines().next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Cabeçalho de tamanho inválido")
        })?;
        
        if !size_line.starts_with("SIZE ") {
            return Err(io::Error::new(io::ErrorKind::InvalidData, 
                format!("Formato inválido: {}", size_line)));
        }

        let file_size: u64 = size_line[5..]
            .trim()
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, 
                "Falha ao converter tamanho do arquivo"))?;

        println!("Iniciando recepção do arquivo: {} ({} bytes esperados)", file_path, file_size);

        // Cria o arquivo e diretório se necessário
        if let Some(parent) = std::path::Path::new(file_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = File::create(file_path).await?;
        let mut total_received = 0;
        let mut buffer = vec![0u8; 64 * 1024]; // Buffer de 64KB

        // Loop principal de recebimento
        while total_received < file_size {
            let to_read = std::cmp::min(buffer.len() as u64, file_size - total_received) as usize;
            
            // Lê um bloco com timeout
            let n = match timeout(read_timeout, socket.read(&mut buffer[..to_read])).await {
                Ok(Ok(n)) => n,
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(io::Error::new(io::ErrorKind::TimedOut, "Timeout na leitura")),
            };

            if n == 0 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, 
                    format!("Conexão fechada após receber {} de {} bytes", total_received, file_size)));
            }

            // Escreve o bloco no arquivo
            file.write_all(&buffer[..n]).await?;
            total_received += n as u64;

            // Atualiza o progresso
            println!("Recebidos {}/{} bytes ({:.1}%)", 
                total_received, 
                file_size, 
                (total_received as f64 / file_size as f64) * 100.0);
        }

        println!("Download completo! Total recebido: {} bytes", total_received);
        Ok(())
    }


    pub async fn list_peer_files(
        &self,
        peer_addr: &str,
    ) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
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
                    Some((parts[0].to_string(), parts[1].parse::<u64>().unwrap_or(0)))
                } else {
                    None
                }
            })
            .collect();

        Ok(files)
    }

    pub async fn list_network_files(
        &self,
        peers: Vec<String>,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let mut network_files = Vec::new();

        for peer in peers {
            if peer != format!("{}:{}", self.ip, self.port) {
                match self.list_peer_files(&peer).await {
                    Ok(files) => {
                        for (file_name, _) in files {
                            network_files.push((file_name, peer.clone()));
                        }
                    }
                    Err(e) => println!("Erro ao listar arquivos do peer {}: {}", peer, e),
                }
            }
        }

        Ok(network_files)
    }

    pub async fn download_blocks_from_peers(
        &self,
        peers: Vec<String>,
        file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for peer in peers {
            if peer != format!("{}:{}", self.ip, self.port) {
                println!("Tentando baixar {} do peer {}", file_name, peer);

                if let Ok(mut socket) = TcpStream::connect(&peer).await {
                    let request = format!("REQUEST_FILE {}", file_name);
                    socket.write_all(request.as_bytes()).await?;

                    // Cria diretório de downloads se não existir
                    let download_dir =
                        dirs::download_dir().unwrap_or_else(|| PathBuf::from("downloads"));
                    tokio::fs::create_dir_all(&download_dir).await?;

                    let download_path = download_dir.join(file_name);
                    match self
                        .receive_file_in_blocks(download_path.to_str().unwrap(), &mut socket)
                        .await
                    {
                        Ok(_) => {
                            println!("Download concluído com sucesso!");
                            return Ok(());
                        }
                        Err(e) => println!("Erro no download de {}: {}", peer, e),
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
                        socket
                            .write_all(response.as_bytes())
                            .await
                            .unwrap_or_else(|e| {
                                println!("Erro ao enviar lista de arquivos: {}", e);
                            });
                    } else if request.starts_with("REQUEST_FILE") {
                        let requested_name = request.split_whitespace().nth(1).unwrap_or("");
                        println!(
                            "Requisição de download recebida para o arquivo: {}",
                            requested_name
                        );

                        // Procura o arquivo pelo nome
                        if let Some(shared_file) =
                            shared_files.iter().find(|sf| sf.name == requested_name)
                        {
                            println!("Arquivo encontrado: {}", shared_file.full_path.display());
                            match peer_clone
                                .send_file_in_blocks(&shared_file.full_path, &mut socket)
                                .await
                            {
                                Ok(_) => {
                                    println!("Arquivo {} enviado com sucesso", shared_file.name)
                                }
                                Err(e) => {
                                    println!("Erro ao enviar arquivo {}: {}", shared_file.name, e);
                                    socket.write_all(b"ERROR\n").await.ok(); // Envia um aviso de erro
                                }
                            }
                        } else {
                            println!("Arquivo {} não encontrado", requested_name);
                            socket.write_all(b"FILE_NOT_FOUND\n").await.ok(); // Envia um aviso de arquivo não encontrado
                        }
                    }
                }
            });
        }
    }

    pub async fn register_with_tracker(
        &self,
        tracker_ip: &str,
        tracker_port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", tracker_ip, tracker_port)).await?;
        let message = format!("REGISTER {}:{}:{}", self.name, self.ip, self.port);
        stream.write_all(message.as_bytes()).await?;
        println!("Registrado no tracker {}:{}", tracker_ip, tracker_port);
        Ok(())
    }

    pub async fn unregister_from_tracker(
        &self,
        tracker_ip: &str,
        tracker_port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", tracker_ip, tracker_port)).await?;
        let message = format!("UNREGISTER {}:{}", self.ip, self.port);
        stream.write_all(message.as_bytes()).await?;
        println!("Desregistrado do tracker {}:{}", tracker_ip, tracker_port);
        Ok(())
    }

    pub async fn get_peers_from_tracker(
        &self,
        tracker_ip: &str,
        tracker_port: u16,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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
        ],
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
