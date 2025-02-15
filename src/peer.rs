use axum::{extract::Query, http::StatusCode, Router}; // Framework web para criar APIs HTTP
use reqwest::Client; // Cliente HTTP para comunicação com o tracker
use serde::{Serialize, Deserialize}; // Serialização e deserialização de JSON
use std::collections::{HashMap, HashSet}; // Estruturas de dados para mapear peers e arquivos
use std::{error::Error, sync::Arc, io, fs}; // Tratamento de erros e manipulação de arquivos
use tokio::net::TcpListener; // Listener TCP para aceitar conexões de outros peers
use rand::Rng; // Gerador de números aleatórios
use std::fs::File; // Manipulação de arquivos
use std::io::{Read, Write}; // Leitura e escrita de arquivos
use tokio::time::{self, Duration}; // Utilitários para tempo e delays assíncronos
use axum::routing::{get, post}; // Rotas HTTP para interações P2P
use rand::prelude::SliceRandom; // Escolha aleatória de peers ao baixar arquivos
use rfd::FileDialog;
use std::path::Path;
use axum::extract::Multipart;

use crate::chat;
use crate::file_utils::{split_file, assemble_file, compute_file_checksum};

// Estrutura para registrar um novo peer no tracker
#[derive(Debug, Serialize, Deserialize)]
struct RegisterRequest {
    name: String,      // Nome do peer
    address: String,   // Endereço do peer
}

// Estrutura para registrar chunks de arquivos
#[derive(Debug, Serialize, Deserialize)]
struct ChunkRegister {
    peer: String,          // Nome do peer que possui o chunk
    file_name: String,     // Nome do arquivo original
    chunk_name: String,    // Nome do chunk específico
    checksum: String,      // Checksum para verificação de integridade
    peer_address: String,  // Endereço do peer que possui o chunk
}

// Estado compartilhado do peer
#[allow(dead_code)]
struct PeerState {
    name: String,          // Nome do peer
    tracker_url: String,   // URL do tracker
    address: String,       // Endereço do peer
}

// Informações sobre um peer
#[derive(Debug, Serialize, Deserialize)]
struct PeerInfo {
    name: String,          // Nome do peer
    address: String,       // Endereço do peer
    files: Vec<String>,    // Lista de arquivos compartilhados
}

#[allow(dead_code)]
type SharedState = Arc<PeerState>;

/// Registra um novo peer no tracker
async fn register_peer(name: &str, address: &str) -> bool {
    let client = Client::new();
    let request = RegisterRequest {
        name: name.to_string(),
        address: address.to_string(),
    };

    // Envia requisição POST para registro
    let res = client.post("http://127.0.0.1:9500/register")
        .json(&request)
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            println!("✅ Peer '{}' registrado com sucesso!", name);
            true
        }
        _ => {
            println!("❌ Nome de usuário já está em uso. Escolha outro.");
            false
        }
    }
}


fn select_file() -> Option<String> {
    FileDialog::new()
        .set_title("Selecione um arquivo para compartilhar")
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}


/// **Copia um arquivo para o diretório do peer**
fn copy_file_to_peer_directory(file_path: &str) -> Option<String> {
    let path = Path::new(file_path);

    if let Some(file_name) = path.file_name() {
        let destination = format!("./{}", file_name.to_string_lossy());

        if let Err(e) = fs::copy(file_path, &destination) {
            println!("❌ Erro ao copiar arquivo: {}", e);
            return None;
        }

        println!("📂 Arquivo copiado para '{}'", destination);
        return Some(destination);
    }

    None
}



/// Registra chunks de arquivos no Tracker
/// **Registra um arquivo a partir de qualquer diretório**
async fn register_chunks(peer_name: &str, peer_address: &str, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Copia o arquivo para o diretório do peer antes de processá-lo
    let local_file_path = match copy_file_to_peer_directory(file_path) {
        Some(path) => path,
        None => {
            println!("❌ Falha ao copiar arquivo '{}'", file_path);
            return Ok(());
        }
    };

    // Obtém apenas o nome do arquivo, sem o caminho absoluto
    let file_name = Path::new(&local_file_path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    println!("📂 Processando '{}'", file_name);

    let client = Client::new();
    
    // Verifica se o arquivo já está registrado no Tracker
    let url = format!("http://127.0.0.1:9500/list");
    let res = client.get(&url).send().await?;

    if res.status().is_success() {
        let list: Vec<PeerInfo> = res.json().await?;
        for peer_info in list {
            if peer_info.name == peer_name && peer_info.files.contains(&file_name) {
                println!("⚠️ O arquivo '{}' já está registrado no Tracker. Ignorando...", file_name);
                return Ok(());
            }
        }
    }

    // Divide o arquivo em chunks
    let chunks = split_file(&file_name);
    if chunks.is_empty() {
        println!("❌ Nenhum chunk foi criado para '{}'. Verifique se o arquivo existe.", file_name);
        return Ok(());
    }

    // Registra cada chunk no Tracker
    for (_, chunk_name, expected_checksum) in &chunks {
        let computed_checksum = compute_file_checksum(chunk_name);
        if computed_checksum != *expected_checksum {
            println!("❌ Erro: Checksum inválido para '{}'. Chunk corrompido.", chunk_name);
            continue;
        }

        let chunk_data = ChunkRegister {
            peer: peer_name.to_string(),
            peer_address: peer_address.to_string(),
            file_name: file_name.clone(), // 🔹 Apenas o nome do arquivo, sem caminho absoluto
            chunk_name: chunk_name.to_string(),
            checksum: expected_checksum.to_string(),
        };

        let res = client.post("http://127.0.0.1:9500/register_chunk")
            .json(&chunk_data)
            .send()
            .await?;

        if res.status().is_success() {
            println!("✅ Chunk '{}' registrado no Tracker!", chunk_name);
        } else {
            println!("❌ Erro ao registrar chunk '{}'", chunk_name);
        }
    }

    Ok(())
}



/// Obtém a lista de chunks disponíveis no tracker
async fn get_chunks(file_name: &str) -> Result<Vec<ChunkRegister>, Box<dyn Error>> {
    let client = Client::new();
    let url = format!("http://127.0.0.1:9500/get_file_chunks?file={}", file_name);
    let res = client.get(&url).send().await?;

    if res.status().is_success() {
        let chunks: Vec<ChunkRegister> = res.json().await?;
        Ok(chunks)
    } else {
        println!("❌ Erro ao buscar chunks do arquivo '{}'.", file_name);
        Ok(vec![])
    }
}

/// Lista todos os peers e arquivos disponíveis na rede
async fn list_peers() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let url = "http://127.0.0.1:9500/list".to_string();
    let res = client.get(&url).send().await?;
    
    if res.status().is_success() {
        let list: Vec<PeerInfo> = res.json().await?;
        println!("📋 Lista de Peers e Arquivos:");
        for peer in list {
            println!("🔹 Peer: {} ({})", peer.name, peer.address);
            if peer.files.is_empty() {
                println!("  📄 Sem arquivos compartilhados");
            } else {
                for file in peer.files {
                    println!("  📄 {}", file);
                }
            }
        }
    } else {
        println!("❌ Erro ao buscar a lista de peers.");
    }
    Ok(())
}

/// Baixa os chunks diretamente dos peers e os salva localmente
/// Baixa os chunks diretamente dos peers e os salva localmente.
/// Agora, ele continua tentando até baixar todos os chunks necessários.
async fn download_chunks(chunks: Vec<ChunkRegister>, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut chunk_map: HashMap<String, Vec<ChunkRegister>> = HashMap::new();

    // Agrupa os chunks pelo nome para saber quais peers possuem quais partes
    for chunk in chunks {
        chunk_map.entry(chunk.chunk_name.clone()).or_default().push(chunk);
    }

    let mut missing_chunks: HashSet<String> = chunk_map.keys().cloned().collect();
    
    while !missing_chunks.is_empty() {
        let mut tasks: Vec<tokio::task::JoinHandle<Result<String, (String, String)>>> = vec![];

        for chunk_name in missing_chunks.clone() {
            if let Some(chunk_peers) = chunk_map.get_mut(&chunk_name) {
                // Escolhe um peer aleatório entre os que ainda possuem o chunk
                if let Some(selected_peer) = chunk_peers.choose(&mut rand::thread_rng()) {
                    let chunk_name_clone = chunk_name.clone();
                    let peer_address = selected_peer.peer_address.clone();
                    let checksum = selected_peer.checksum.clone();
                    let client_clone = client.clone();

                    tasks.push(tokio::spawn(async move {
                        let chunk_url = format!("http://{}/get_chunk?name={}", peer_address, chunk_name_clone);
                        println!("⬇️ Tentando baixar chunk '{}' de '{}'", chunk_name_clone, peer_address);
                    
                        match client_clone.get(&chunk_url).send().await {
                            Ok(res) if res.status().is_success() => {
                                let bytes = res.bytes().await.unwrap();
                                let mut file = File::create(&chunk_name_clone).unwrap();
                                file.write_all(&bytes).unwrap();
                    
                                let downloaded_checksum = compute_file_checksum(&chunk_name_clone);
                                if downloaded_checksum != checksum {
                                    println!("❌ Checksum inválido para '{}'. Excluindo chunk corrompido.", chunk_name_clone);
                                    std::fs::remove_file(&chunk_name_clone).unwrap();
                                    return Err((chunk_name_clone, peer_address)); // Retorna o chunk e o peer que falhou
                                }
                    
                                println!("✅ Chunk '{}' baixado com sucesso!", chunk_name_clone);
                                Ok(chunk_name_clone) // Indica sucesso no download
                            }
                            _ => Err((chunk_name_clone, peer_address)), // Se falhar, retorna o nome do chunk e o peer que falhou
                        }
                    }));
                }
            }
        }

        let results = futures::future::join_all(tasks).await;

        // Processa os resultados
        for result in results {
            match result {
                Ok(Ok(chunk_name)) => {
                    missing_chunks.remove(&chunk_name); // Remove da lista de chunks pendentes
                }
                Ok(Err((chunk_name, failed_peer))) => {
                    println!("❌ Falha ao baixar '{}'. Removendo '{}' da lista de peers válidos.", chunk_name, failed_peer);
                    // Remove o peer que falhou da lista de peers disponíveis para esse chunk
                    if let Some(peers) = chunk_map.get_mut(&chunk_name) {
                        peers.retain(|peer| peer.peer_address != failed_peer);
                    }
                }
                _ => {}
            }
        }

        if !missing_chunks.is_empty() {
            println!("🔄 Alguns chunks falharam no download. Tentando novamente...");
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await; // Pequeno delay antes de tentar novamente
        }
    }

    println!("✅ Todos os chunks foram baixados!");
    println!("🔄 Tentando reconstruir o arquivo original '{}'", file_name);
    assemble_file(file_name);

    Ok(())
}

async fn upload_file(mut multipart: Multipart) -> Result<String, StatusCode> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        let file_path = Path::new("./").join(&file_name);
        let mut file = File::create(&file_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        file.write_all(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        println!("📂 Arquivo '{}' salvo!", file_name);
    }
    Ok("✅ Arquivo recebido!".to_string())
}


/// Servidor que permite que outros peers baixem chunks deste peer
async fn send_chunk(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Vec<u8>, StatusCode> {
    if let Some(chunk_name) = params.get("name") {
        let mut file = match File::open(chunk_name) {
            Ok(f) => f,
            Err(_) => return Err(StatusCode::NOT_FOUND),
        };

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        Ok(buffer)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// Função auxiliar para download e registro automático de arquivos
async fn download_and_register(peer_name: &str, peer_address: &str, file_name: &str) {
    println!("🔄 Buscando chunks de '{}'...", file_name);
    match get_chunks(file_name).await {
        Ok(chunks) if chunks.is_empty() => println!("⚠️ Nenhum chunk encontrado."),
        Ok(chunks) => {
            if let Err(e) = download_chunks(chunks, file_name).await {
                println!("❌ Erro ao baixar chunks: {}", e);
            } else {
                println!("✅ Download concluído e arquivo reconstruído!");
                println!("📢 Registrando automaticamente o arquivo baixado...");
                if let Err(e) = register_chunks(peer_name, peer_address, file_name).await {
                    println!("❌ Erro ao registrar '{}': {}", file_name, e);
                }
            }
        }
        Err(e) => println!("❌ Erro ao buscar arquivo '{}': {}", file_name, e),
    }
}

/// Monitor de arquivos deletados
async fn monitor_deleted_files(peer_name: String) {
    loop {
        time::sleep(Duration::from_secs(1)).await;

        // Verifica arquivos atuais no diretório
        if let Ok(entries) = fs::read_dir(".") {
            let current_files: Vec<String> = entries
                .flatten()
                .filter_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
                .collect();

            // Verifica arquivos registrados no tracker
            let client = Client::new();
            let url = "http://127.0.0.1:9500/list".to_string();
            let res = client.get(&url).send().await;

            if let Ok(response) = res {
                if response.status().is_success() {
                    let list: std::collections::HashMap<String, Vec<String>> = response.json().await.unwrap_or_default();

                    if let Some(files) = list.get(&peer_name) {
                        for file in files {
                            if !current_files.contains(file) {
                                println!("🚨 O arquivo '{}' foi deletado! Removendo do Tracker...", file);
                                if let Err(e) = unregister_file(&peer_name, file).await {
                                    println!("❌ Erro ao remover '{}': {}", file, e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
async fn monitor_lost_chunks(peer_name: String) {
    loop {
        time::sleep(Duration::from_secs(10)).await;

        // Lista todos os arquivos no diretório
        let mut current_chunks: HashSet<String> = HashSet::new();
        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.contains(".chunk") {
                        current_chunks.insert(file_name.to_string());
                    }
                }
            }
        }

        // Obtém do Tracker quais chunks esse Peer deveria ter
        let client = Client::new();
        let url = format!("http://127.0.0.1:9500/get_peer_chunks?peer={}", peer_name);
        let res = client.get(&url).send().await;

        if let Ok(response) = res {
            if response.status().is_success() {
                let expected_chunks: Vec<String> = response.json().await.unwrap_or_default();

                for chunk in expected_chunks {
                    if !current_chunks.contains(&chunk) {
                        println!("🚨 Chunk '{}' foi perdido! Removendo do Tracker...", chunk);
                        let payload = serde_json::json!({ "peer": peer_name, "chunk": chunk });
                        let _ = client.post("http://127.0.0.1:9500/unregister_chunk")
                            .json(&payload)
                            .send()
                            .await;
                    }
                }
            }
        }
    }
}


/// Remove um arquivo do tracker
async fn unregister_file(peer_name: &str, file_name: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let payload = serde_json::json!({ "peer": peer_name, "file": file_name });

    let res = client.post("http://127.0.0.1:9500/unregister_file")
        .json(&payload)
        .send()
        .await?;

    let status = res.status();
    let response_text = res.text().await?;

    if status.is_success() {
        println!("🚨 Arquivo '{}' removido do Tracker! Resposta: {}", file_name, response_text);
    } else {
        println!("❌ Falha ao remover '{}': HTTP {} - {}", file_name, status, response_text);
    }

    Ok(())
}

/// Remove um peer do tracker
async fn unregister_peer(peer_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let payload = serde_json::json!({ "peer": peer_name });

    let res = client.post("http://127.0.0.1:9500/unregister_peer")
        .json(&payload)
        .send()
        .await?;

    if res.status().is_success() {
        println!("👋 Peer '{}' removido do Tracker com sucesso!", peer_name);
    } else {
        println!("❌ Falha ao remover peer '{}'.", peer_name);
    }

    Ok(())
}

/// Monitor de arquivos ausentes - verifica periodicamente se arquivos registrados ainda existem
async fn monitor_missing_files(peer_name: String) {
    loop {
        time::sleep(Duration::from_secs(1)).await;

        // Cria um conjunto com os arquivos atualmente presentes no diretório
        let mut current_files: HashSet<String> = HashSet::new();
        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    current_files.insert(file_name.to_string());
                }
            }
        }

        // Consulta a lista de arquivos registrados no tracker
        let client = Client::new();
        let url = "http://127.0.0.1:9500/list".to_string();
        let res = client.get(&url).send().await;

        if let Ok(response) = res {
            if response.status().is_success() {
                let list: Vec<PeerInfo> = response.json().await.unwrap_or_default();

                // Verifica os arquivos registrados para este peer
                for peer in list {
                    if peer.name == peer_name {
                        for file in peer.files {
                            // Verifica se existem chunks do arquivo
                            let has_chunks = current_files.iter().any(|f| f.starts_with(&file) && f.contains(".chunk"));

                            // Se o arquivo não existe e não há chunks, remove do tracker
                            if !current_files.contains(&file) && !has_chunks {
                                println!("🚨 Arquivo '{}' sumiu! Removendo do Tracker...", file);
                                if let Err(e) = unregister_file(&peer_name, &file).await {
                                    println!("❌ Erro ao remover '{}': {}", file, e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Função principal que inicia o peer
pub async fn start_peer() {
    // Solicita e valida o nome do usuário
    let mut name = String::new();
    loop {
        print!("Digite seu nome de usuário: ");
        io::Write::flush(&mut io::stdout()).unwrap();
        io::stdin().read_line(&mut name).unwrap();
        name = name.trim().to_string();

        if !name.is_empty() {
            break;
        }
        println!("❌ Nome inválido. Tente novamente.");
    }

    // Gera uma porta aleatória entre 8000 e 9000
    let port = rand::thread_rng().gen_range(8000..9000);
    let address = format!("127.0.0.1:{}", port);

    // Tenta registrar o peer no tracker
    if !register_peer(&name, &address).await {
        return;
    }

    // Inicia os monitores de arquivos em background
    tokio::spawn(monitor_deleted_files(name.clone()));
    tokio::spawn(monitor_missing_files(name.clone())); 

    // Configura o estado compartilhado do peer
    let state = Arc::new(PeerState {
        name: name.clone(),
        tracker_url: "http://127.0.0.1:9500".to_string(),
        address: address.clone(),
    });

    // Configura as rotas do servidor
    let app = Router::new()
        .route("/get_chunk", get(send_chunk))
        .route("/chat", post(chat::receive_chat)) 
        .with_state(state.clone());
    
    // Inicia o servidor na porta escolhida
    let listener = TcpListener::bind(&address).await.unwrap();
    println!("📡 Peer '{}' rodando em {}", name, address);

    // Inicia o servidor em uma task separada
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Verifica e compartilha automaticamente arquivos .txt existentes
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "txt" {
                        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
                        println!("📂 Arquivo encontrado: '{}' - Compartilhando automaticamente...", file_name);
                        if let Err(e) = register_chunks(&name, &address, &file_name).await {
                            println!("❌ Erro ao compartilhar '{}': {}", file_name, e);
                        }
                    }
                }
            }
        }
    }

    // Loop principal de comandos
    loop {
        println!("\n📜 Comandos: share | get | list | chat | exit");

        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        let parts: Vec<&str> = command.trim().split_whitespace().collect();

        // Processa os comandos do usuário
        match parts.as_slice() {
            ["chat"] => {
                // Implementação do chat peer-to-peer
                println!("Digite o endereço do peer destinatário (ex: 127.0.0.1:8000): ");
                let mut recipient_address = String::new();
                io::stdin().read_line(&mut recipient_address).unwrap();
                let recipient_address = recipient_address.trim();
            
                println!("Digite sua mensagem: ");
                let mut message = String::new();
                io::stdin().read_line(&mut message).unwrap();
                let message = message.trim();
            
                // Cria e envia a mensagem
                let chat_message = chat::ChatMessage {
                    sender: name.clone(),
                    message: message.to_string(),
                    timestamp: chat::current_timestamp(),
                };
            
                if let Err(e) = chat::send_chat_message(recipient_address, chat_message).await {
                    println!("❌ Erro ao enviar a mensagem: {}", e);
                }
            }
            
            // Comando para compartilhar arquivo 
            ["share"] => {
                // Abre o explorador de arquivos para seleção
                if let Some(file_path) = select_file() {
                    println!("📂 Arquivo selecionado: {}", file_path);
                    if let Err(e) = register_chunks(&name, &address, &file_path).await {
                        println!("❌ Erro ao compartilhar arquivo '{}': {}", file_path, e);
                    }
                } else {
                    println!("⚠️ Nenhum arquivo foi selecionado.");
                }
            }
            

            // Comando para baixar arquivo (sem nome do arquivo)
            ["get"] => {
                println!("Digite o nome do arquivo que deseja baixar:");
                let mut file_name = String::new();
                io::stdin().read_line(&mut file_name).unwrap();
                let file_name = file_name.trim();

                if file_name.is_empty() {
                    println!("❌ Nome do arquivo inválido.");
                } else {
                    download_and_register(&name, &address, file_name).await;
                }
            }

            // Comando para baixar arquivo (com nome do arquivo)
            ["get", file] => {
                download_and_register(&name, &address, file).await;
            }

            // Comando para listar peers e arquivos
            ["list"] => {
                if let Err(e) = list_peers().await {
                    println!("❌ Erro ao listar peers: {}", e);
                }
            }

            // Comando para sair do programa
            ["exit"] => {
                println!("👋 Saindo...");
                if let Err(e) = unregister_peer(&name).await {
                    println!("❌ Erro ao remover peer: {}", e);
                }
                break;
            }
            
            // Comando inválido
            _ => println!("❌ Comando inválido!"),
        }
    }
}