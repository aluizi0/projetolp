mod peer;
mod tracker;

use crate::peer::{Peer, list_local_files};
use crate::tracker::Tracker;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::env;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: cargo run -- tracker [porta] | cargo run -- peer [porta_tracker]");
        return;
    }
    let mode = &args[1];

    if mode == "tracker" {
        // Porta do tracker (padrão: 7000)
        let port = args.get(2).and_then(|p| p.parse::<u16>().ok()).unwrap_or(7000);
        let tracker = Arc::new(Mutex::new(Tracker::new()));
        println!("Iniciando o tracker na porta {}...", port);

        // Chamar o método `start` sem manter o lock do Mutex
        {
            let tracker = tracker.lock().await;
            if let Err(e) = tracker.start(port).await {
                eprintln!("Erro ao iniciar o tracker: {:?}", e);
            }
        }
    } else if mode == "peer" {
        // Porta do tracker (padrão: 7000)
        let tracker_port = args.get(2).and_then(|p| p.parse::<u16>().ok()).unwrap_or(7000);

        print!("Digite seu nome de peer: ");
        io::stdout().flush().unwrap();
        let mut peer_name = String::new();
        io::stdin().read_line(&mut peer_name).unwrap();
        let peer_name = peer_name.trim().to_string();

        // Porta do peer (dinâmica)
        let peer_port: u16 = 7001 + rand::random::<u16>() % 1000;
        let peer = Arc::new(Peer::new(
            "127.0.0.1".to_string(),
            peer_port,
            list_local_files("shared_files"),
            peer_name.clone(),
        ));

        // Registrar o peer no tracker
        if let Err(e) = peer.register_with_tracker("127.0.0.1", tracker_port).await {
            eprintln!("Erro ao registrar no tracker: {:?}", e);
            return;
        }

        let peer_clone = Arc::clone(&peer);
        tokio::spawn(async move {
            if let Err(e) = peer_clone.start_server().await {
                eprintln!("Erro ao iniciar o servidor do peer: {:?}", e);
            }
        });

        // Comandos no terminal
        println!("Digite 'list' para obter a lista de peers ou 'exit' para sair.");
        loop {
            let mut command = String::new();
            io::stdin().read_line(&mut command).unwrap();
            let command = command.trim().to_string();

            if command == "list" {
                match peer.get_peers_from_tracker("127.0.0.1", tracker_port).await {
                    Ok(peers) => println!("Lista de Peers: {:?}", peers),
                    Err(e) => eprintln!("Erro ao obter a lista de peers: {:?}", e),
                }
            } else if command == "exit" {
                // Desregistrar do tracker
                if let Err(e) = peer.unregister_from_tracker("127.0.0.1", tracker_port).await {
                    eprintln!("Erro ao desregistrar do tracker: {:?}", e);
                }
                println!("Desconectando do tracker...");
                break;
            } else {
                println!("Comando desconhecido: {}", command);
            }
        }
    }
}
