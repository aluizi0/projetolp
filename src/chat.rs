use axum::{
    extract::Json,
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};

/// Estrutura que representa uma mensagem de chat entre peers.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: String,
    pub message: String,
    pub timestamp: u64,
}

/// Retorna o timestamp atual (em segundos).
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Handler para receber mensagens de chat via endpoint `/chat`.
///
/// Ao receber uma mensagem, o handler a exibe no console e responde com um OK.
pub async fn receive_chat(Json(payload): Json<ChatMessage>) -> impl IntoResponse {
    println!("[CHAT] {} diz: {}", payload.sender, payload.message);
    (StatusCode::OK, Json("Mensagem recebida".to_string()))
}

/// Envia uma mensagem de chat para o peer destinatário.
///
/// # Parâmetros
/// - `recipient_address`: endereço do peer (ex: "127.0.0.1:8000") que receberá a mensagem.
/// - `chat_message`: estrutura com os dados da mensagem.
///
/// Retorna um `Result` indicando se a mensagem foi enviada com sucesso.
pub async fn send_chat_message(
    recipient_address: &str,
    chat_message: ChatMessage,
) -> Result<(), reqwest::Error> {
    let client = Client::new();
    let url = format!("http://{}/chat", recipient_address);
    
    let response = client.post(&url)
        .json(&chat_message)
        .send()
        .await?;
    
    if response.status().is_success() {
        println!("✅ Mensagem enviada para {}!", recipient_address);
    } else {
        println!("❌ Falha ao enviar mensagem para {}: HTTP {}", recipient_address, response.status());
    }
    
    Ok(())
}