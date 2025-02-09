use axum::{routing::*, extract::{State, Json, Query}, http::StatusCode, Router};
use std::{collections::HashMap, sync::{Arc, Mutex}};
use tokio::net::TcpListener;
use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize, Clone)]
struct Peer {
    name: String,
    address: String,
    last_seen: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChunkRegister {
    peer: String,
    file_name: String,
    chunk_name: String,
    checksum: String,
    peer_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterRequest {
    name: String,
    address: String,
}


#[derive(Debug, Serialize, Deserialize)]
struct PeerInfo {
    pub name: String,
    pub address: String,
    pub files: Vec<String>,
}

struct TrackerState {
    peers: Mutex<HashMap<String, Peer>>,
    chunks: Mutex<HashMap<String, Vec<ChunkRegister>>>,
}

type SharedState = Arc<TrackerState>;

/// **Registra um Peer no Tracker**
async fn register_peer(
    State(state): State<SharedState>,
    Json(payload): Json<RegisterRequest>,
) -> (StatusCode, Json<String>) {
    let mut peers = state.peers.lock().unwrap();

    if peers.contains_key(&payload.name) {
        return (StatusCode::BAD_REQUEST, Json("Nome já registrado".to_string()));
    }

    peers.insert(payload.name.clone(), Peer {
        name: payload.name.clone(),
        address: payload.address.clone(),
        last_seen: current_timestamp(),
    });

    println!("✅ Peer registrado: {:?}", payload);
    (StatusCode::OK, Json(format!("{} registrado com sucesso!", payload.name)))
}

/// **Registra chunks de arquivos no Tracker**
async fn register_chunks(
    State(state): State<SharedState>,
    Json(payload): Json<ChunkRegister>,
) -> (StatusCode, Json<String>) {
    let mut chunks = state.chunks.lock().unwrap();
    let entry = chunks.entry(payload.file_name.clone()).or_insert(vec![]);
    
    // **Evita registrar duplicatas**
    if !entry.iter().any(|c| c.chunk_name == payload.chunk_name && c.peer == payload.peer) {
        entry.push(payload);
        println!("📦 Chunk registrado no Tracker!");
        (StatusCode::OK, Json("Chunk registrado com sucesso!".to_string()))
    } else {
        println!("⚠️ Chunk já registrado, ignorando.");
        (StatusCode::OK, Json("Chunk já registrado, ignorando.".to_string()))
    }
}

/// **Obtém a lista de chunks disponíveis no Tracker**
async fn get_file_chunks(
    State(state): State<SharedState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<ChunkRegister>> {
    let chunks = state.chunks.lock().unwrap();
    let file_name = params.get("file").cloned().unwrap_or_default();
    
    let result = chunks.get(&file_name).cloned().unwrap_or(vec![]);
    println!("📄 Chunks encontrados para '{}': {:?}", file_name, result);

    Json(result)
}

/// **Lista todos os peers e arquivos disponíveis**
async fn list_peers(
    State(state): State<SharedState>,
) -> Json<Vec<PeerInfo>> {
    let peers = state.peers.lock().unwrap();
    let chunks = state.chunks.lock().unwrap();
    
    let mut infos: Vec<PeerInfo> = Vec::new();
    
    // Para cada peer registrado, coletamos os arquivos (caso existam) a partir dos chunks
    for (peer_name, peer) in peers.iter() {
        let mut files_set: std::collections::HashSet<String> = std::collections::HashSet::new();
        
        // Itera por todos os chunks e seleciona os arquivos deste peer
        for (_, chunk_list) in chunks.iter() {
            for chunk in chunk_list {
                if &chunk.peer == peer_name {
                    files_set.insert(chunk.file_name.clone());
                }
            }
        }
        
        let files: Vec<String> = files_set.into_iter().collect();
        
        infos.push(PeerInfo {
            name: peer_name.clone(),
            address: peer.address.clone(),
            files,
        });
    }
    
    println!("📋 Lista de Peers e Arquivos: {:?}", infos);
    Json(infos)
}


/// **Remove arquivos deletados do tracker**
async fn unregister_file(
    State(state): State<SharedState>,
    Json(payload): Json<HashMap<String, String>>,
) -> (StatusCode, Json<String>) {
    let peer_name = payload.get("peer").cloned().unwrap_or_default();
    let file_name = payload.get("file").cloned().unwrap_or_default();
    let mut chunks = state.chunks.lock().unwrap();

    if let Some(entries) = chunks.get_mut(&file_name) {
        entries.retain(|chunk| chunk.peer != peer_name);
        if entries.is_empty() {
            chunks.remove(&file_name);
        }
        println!("🚨 Peer '{}' removeu o arquivo '{}'", peer_name, file_name);
        return (StatusCode::OK, Json(format!("Arquivo '{}' removido para peer '{}'", file_name, peer_name)));
    }

    (StatusCode::NOT_FOUND, Json(format!("Arquivo '{}' não encontrado.", file_name)))
}

/// **Recebe heartbeat dos peers ativos**
/// **Recebe heartbeat dos peers ativos**
async fn heartbeat(
    State(state): State<SharedState>,
    Json(peer_name): Json<String>,  // 🔹 Agora aceita uma string simples
) -> StatusCode {
    let mut peers = state.peers.lock().unwrap();

    if let Some(peer) = peers.get_mut(&peer_name) {
        peer.last_seen = current_timestamp();
        println!("💓 Heartbeat recebido de '{}' (último visto: {}s)", peer_name, peer.last_seen);
        return StatusCode::OK;
    }

    println!("❌ Peer '{}' tentou enviar heartbeat, mas não foi encontrado no Tracker!", peer_name);
    StatusCode::NOT_FOUND
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

async fn unregister_peer(
    State(state): State<SharedState>,
    Json(payload): Json<HashMap<String, String>>,
) -> (StatusCode, Json<String>) {
    let peer_name = payload.get("peer").cloned().unwrap_or_default();
    let mut peers = state.peers.lock().unwrap();

    if peers.remove(&peer_name).is_some() {
        println!("🚨 Peer '{}' saiu da rede e foi removido.", peer_name);
        return (StatusCode::OK, Json(format!("Peer '{}' removido.", peer_name)));
    }

    (StatusCode::NOT_FOUND, Json(format!("Peer '{}' não encontrado.", peer_name)))
}

/// **Remove um Chunk específico de um Peer**
#[allow(dead_code)] 
async fn unregister_chunk(
    State(state): State<SharedState>,
    Json(payload): Json<HashMap<String, String>>,
) -> (StatusCode, Json<String>) {
    let peer_name = payload.get("peer").cloned().unwrap_or_default();
    let chunk_name = payload.get("chunk").cloned().unwrap_or_default();
    let mut chunks = state.chunks.lock().unwrap();

    for (_, chunk_list) in chunks.iter_mut() {
        chunk_list.retain(|chunk| !(chunk.peer == peer_name && chunk.chunk_name == chunk_name));
    }

    println!("🚨 Peer '{}' removeu o chunk '{}'", peer_name, chunk_name);
    (StatusCode::OK, Json(format!("Chunk '{}' removido para peer '{}'", chunk_name, peer_name)))
}


/// **Inicia o Tracker**
pub async fn start_tracker() {
    let state = Arc::new(TrackerState {
        peers: Mutex::new(HashMap::new()),
        chunks: Mutex::new(HashMap::new()),
    });

    let app = Router::new()
        .route("/register", post(register_peer))
        .route("/heartbeat", post(heartbeat))
        .route("/register_chunk", post(register_chunks))
        .route("/get_file_chunks", get(get_file_chunks))
        .route("/list", get(list_peers))
        .route("/unregister_file", post(unregister_file))
        .route("/unregister_peer", post(unregister_peer))
        .with_state(state.clone());
        



    let listener = TcpListener::bind("0.0.0.0:9500").await.unwrap();
    println!("📡 Tracker rodando na porta 9500...");
    axum::serve(listener, app).await.unwrap();
}