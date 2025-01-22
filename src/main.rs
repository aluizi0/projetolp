mod peer;
mod tracker;

use crate::peer::{Peer, list_local_files};
use crate::tracker::Tracker;
use tokio::sync::Mutex;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let tracker = Arc::new(Tracker::new());

    // Iniciar tracker em uma thread separada
    let tracker_clone = Arc::clone(&tracker);
    tokio::spawn(async move {
        tracker_clone.start(6881).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Criando um peer
    let peer = Arc::new(Peer::new(
        "127.0.0.1".to_string(),
        6882,
        list_local_files("shared_files"),
    ));

    peer.register_with_tracker("127.0.0.1", 6881).await.unwrap();

    let peer_clone = Arc::clone(&peer);
    tokio::spawn(async move {
        peer_clone.start_server().await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    match peer.get_peers_from_tracker("127.0.0.1", 6881).await {
        Ok(peers) => {
            println!("Peers disponíveis: {:?}", peers);

            // Supondo que você queira solicitar um arquivo específico de um peer
            if let Some(peer_address) = peers.first() {
                peer.request_file_from_peer(peer_address, "nome_do_arquivo_que_deseja").await.unwrap();
            }
        }
        Err(e) => {
            eprintln!("Erro ao buscar peers: {}", e);
        }
    }
}