mod peer;
mod tracker;
mod chat;

use crate::peer::{Peer, list_local_files};
use crate::tracker::Tracker;
use crate::chat::{ChatServer, start_chat_client, message_receiver};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::env;
use std::io::{self, Write};
use tokio::sync::mpsc;

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
        let port: u16 = args.get(2).and_then(|p| p.parse::<u16>().ok()).unwrap_or(7000);
        let tracker = Arc::new(Mutex::new(Tracker::new()));
        println!("📡 Iniciando o tracker na porta {}...", port);

        // Tentando iniciar o tracker e tratando erros
        let tracker_clone = Arc::clone(&tracker);
        let result = tracker_clone.lock().await.start(port).await;

        if let Err(e) = result {
            eprintln!("❌ Erro ao iniciar o tracker: {:?}\nVerifique se a porta {} está em uso e tente outra.", e, port);
        }
    } else if mode == "peer" {
        // Porta do tracker (padrão: 7000)
        let tracker_port: u16 = args.get(2).and_then(|p| p.parse::<u16>().ok()).unwrap_or(7000);

        print!("Digite seu nome de peer: ");
        io::stdout().flush().unwrap();
        let mut peer_name = String::new();
        io::stdin().read_line(&mut peer_name).unwrap();
        let peer_name = peer_name.trim().to_string();

        // Escolhe uma porta aleatória para o peer (7001+ para evitar conflitos)
        let peer_port: u16 = 7001 + rand::random::<u16>() % 1000;
        let peer = Arc::new(Peer::new(
            "127.0.0.1".to_string(),
            peer_port,
            list_local_files("shared_files"),
            peer_name.clone(),
        ));

        // Tenta registrar o peer no tracker e captura erros
        if let Err(e) = peer.register_with_tracker("127.0.0.1", tracker_port).await {
            eprintln!("❌ Erro ao registrar no tracker: {:?}\nVerifique se o tracker está rodando na porta {}.", e, tracker_port);
            return;
        }

        let peer_clone = Arc::clone(&peer);
        tokio::spawn(async move {
            if let Err(e) = peer_clone.start_server().await {
                eprintln!("❌ Erro ao iniciar o servidor do peer: {:?}", e);
            }
        });

        // Configuração do chat entre peers
        let (sender, receiver) = mpsc::channel(100);
        let chat_server = ChatServer::new(sender);

        tokio::spawn(async move {
            if let Err(e) = chat_server.start_chat_server(peer_port + 1000).await {
                eprintln!("❌ Erro ao iniciar o servidor de chat: {:?}", e);
            }
        });

        tokio::spawn(async move {
            message_receiver(receiver).await;
        });

        println!("🔹 Digite 'list' para obter a lista de peers, 'chat' para iniciar um chat ou 'exit' para sair.");
        loop {
            let mut command = String::new();
            io::stdin().read_line(&mut command).unwrap();
            let command = command.trim().to_string();

            if command == "list" {
                match peer.get_peers_from_tracker("127.0.0.1", tracker_port).await {
                    Ok(peers) => println!("📜 Lista de Peers: {:?}", peers),
                    Err(e) => eprintln!("❌ Erro ao obter a lista de peers: {:?}", e),
                }
            } else if command == "chat" {
                print!("🔹 Digite o número da porta do peer para iniciar o chat: ");
                io::stdout().flush().unwrap();
                let mut target_port_str = String::new();
                io::stdin().read_line(&mut target_port_str).unwrap();
                
                if let Ok(target_port) = target_port_str.trim().parse::<u16>() {
                    if let Err(e) = start_chat_client(&peer_name, target_port + 1000).await {
                        eprintln!("❌ Erro ao iniciar chat: {:?}", e);
                    }
                } else {
                    println!("❌ Porta inválida.");
                }
            } else if command == "exit" {
                if let Err(e) = peer.unregister_from_tracker("127.0.0.1", tracker_port).await {
                    eprintln!("❌ Erro ao desregistrar do tracker: {:?}", e);
                }
                println!("🔻 Desconectando do tracker...");
                break;
            } else {
                println!("❓ Comando desconhecido: {}", command);
            }
        }
    }
}
