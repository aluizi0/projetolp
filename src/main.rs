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
        eprintln!("Uso: cargo run -- tracker | cargo run -- peer");
        return;
    }
    let mode = &args[1];

    if mode == "tracker" {
        let tracker = Arc::new(Mutex::new(Tracker::new()));
        println!("Iniciando o tracker...");
        tracker.lock().await.start(6881).await.unwrap();
    } else if mode == "peer" {
        print!("Digite seu nome de peer: ");
        io::stdout().flush().unwrap();
        let mut peer_name = String::new();
        io::stdin().read_line(&mut peer_name).unwrap();
        let peer_name = peer_name.trim().to_string();

        // Lista arquivos dos diretórios padrão
        let files = list_local_files(None);
        println!("\nArquivos disponíveis para compartilhar:");
        for (index, (file_name, path)) in files.iter().enumerate() {
            println!("{}: {} ({})", index, file_name, path.display());
        }

        print!("\nEscolha o número do arquivo para compartilhar: ");
        io::stdout().flush().unwrap();
        let mut file_choice = String::new();
        io::stdin().read_line(&mut file_choice).unwrap();
        let file_choice: usize = file_choice.trim().parse().unwrap_or(0);

        let shared_files = if file_choice < files.len() {
            println!("Compartilhando arquivo: {}", files[file_choice].1.display());
            vec![files[file_choice].1.to_string_lossy().to_string()]
        } else {
            println!("Índice inválido, nenhum arquivo será compartilhado.");
            vec![]
        };

        let peer_port: u16 = 6882 + rand::random::<u16>() % 1000;
        let peer = Arc::new(Peer::new(
    "127.0.0.1".to_string(),
    peer_port,
    vec![files[file_choice].1.to_string_lossy().to_string()],
    peer_name.clone(),
));

        // Registrar o peer no tracker
        peer.register_with_tracker("127.0.0.1", 6881).await.unwrap();

        let peer_clone = Arc::clone(&peer);
        tokio::spawn(async move {
            peer_clone.start_server().await.unwrap();
        });

        // Criação do canal para comunicação das mensagens
        let (sender, receiver) = mpsc::channel(100);
        let chat_server = ChatServer::new(sender);

        tokio::spawn(async move {
            chat_server.start_chat_server(peer_port + 1000).await.unwrap();
        });

        // Escuta de mensagens em paralelo
        tokio::spawn(async move {
            message_receiver(receiver).await;
        });

        // Comandos no terminal
        println!("\nComandos disponíveis:");
        println!("- 'list': lista peers conectados");
        println!("- 'files': lista arquivos disponíveis na rede");
        println!("- 'chat': inicia chat com outro peer");
        println!("- 'download': baixa um arquivo");
        println!("- 'exit': sair");

        loop {
            let mut command = String::new();
            io::stdin().read_line(&mut command).unwrap();
            let command = command.trim().to_string();

            match command.as_str() {
                "list" => {
                    let peers = peer.get_peers_from_tracker("127.0.0.1", 6881).await.unwrap();
                    println!("Peers conectados: {:?}", peers);
                }
                "files" => {
                    let peers = peer.get_peers_from_tracker("127.0.0.1", 6881).await.unwrap();
                    match peer.list_network_files(peers).await {
                        Ok(files) => {
                            println!("\nArquivos disponíveis na rede:");
                            for (index, (file, peer_addr)) in files.iter().enumerate() {
                                println!("{}: {} (disponível em {})", index, file, peer_addr);
                            }
                        }
                        Err(e) => println!("Erro ao listar arquivos: {}", e)
                    }
                }
                "chat" => {
                    print!("Digite o número da porta do peer para iniciar o chat: ");
                    io::stdout().flush().unwrap();
                    let mut target_port_str = String::new();
                    io::stdin().read_line(&mut target_port_str).unwrap();
                    let target_port: u16 = target_port_str.trim().parse().unwrap();
                    
                    start_chat_client(target_port + 1000).await.unwrap();
                }
                "download" => {
                    let peers = peer.get_peers_from_tracker("127.0.0.1", 6881).await.unwrap();
                    
                    // Primeiro lista os arquivos disponíveis
                    match peer.list_network_files(peers.clone()).await {
                        Ok(files) => {
                            println!("\nArquivos disponíveis para download:");
                            for (index, (file, peer_addr)) in files.iter().enumerate() {
                                println!("{}: {} (em {})", index, file, peer_addr);
                            }
                            
                            print!("\nDigite o número do arquivo que deseja baixar: ");
                            io::stdout().flush().unwrap();
                            let mut choice = String::new();
                            io::stdin().read_line(&mut choice).unwrap();
                            let choice = choice.trim();
                            
                            if let Ok(index) = choice.parse::<usize>() {
                                if index < files.len() {
                                    let (file_name, peer_addr) = &files[index];
                                    println!("Iniciando download de {} do peer {}", file_name, peer_addr);
                                    match (*peer).download_blocks_from_peers(vec![peer_addr.clone()], file_name).await {
                                        Ok(_) => println!("Download concluído com sucesso!"),
                                        Err(e) => println!("Erro no download: {}", e)
                                    }
                                } else {
                                    println!("Índice inválido!");
                                }
                            } else {
                                println!("Por favor, digite um número válido!");
                            }
                        }
                        Err(e) => println!("Erro ao listar arquivos: {}", e)
                    }
                }
                "exit" => {
                    peer.unregister_from_tracker("127.0.0.1", 6881).await.unwrap();
                    println!("Desconectando do tracker...");
                    break;
                }
                _ => {
                    println!("Comando desconhecido. Comandos disponíveis:");
                    println!("- 'list': lista peers conectados");
                    println!("- 'files': lista arquivos disponíveis na rede");
                    println!("- 'chat': inicia chat com outro peer");
                    println!("- 'download': baixa um arquivo");
                    println!("- 'exit': sair");
                }
            }
        }
    }
} 