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
    
        let port = 6881;
        
        // Obtém o lock e inicia o tracker
        {
            let tracker_guard = tracker.lock().await;
            match tracker_guard.start(port).await {
                Ok(_) => println!("Tracker iniciado na porta {}", port),
                Err(e) => eprintln!("Erro ao iniciar o tracker: {}", e),
            }
        } // <- Aqui, `tracker_guard` sai do escopo e libera o lock antes de continuar
    
    } else if mode == "peer" {
        print!("Digite seu nome de peer: ");
        io::stdout().flush().unwrap();
        let mut peer_name = String::new();
        io::stdin().read_line(&mut peer_name).unwrap();
        let peer_name = peer_name.trim().to_string();

        // Lista arquivos disponíveis
        let files = list_local_files(None);
        if files.is_empty() {
            println!("Nenhum arquivo encontrado para compartilhamento.");
            return;
        }

        println!("\nArquivos disponíveis para compartilhar:");
        for (index, (file_name, path)) in files.iter().enumerate() {
            println!("{}: {} ({})", index, file_name, path.display());
        }

        // Solicita ao usuário escolher um arquivo
        print!("\nEscolha o número do arquivo para compartilhar: ");
        io::stdout().flush().unwrap();
        let mut file_choice = String::new();
        io::stdin().read_line(&mut file_choice).unwrap();

        let file_choice: Option<usize> = file_choice.trim().parse().ok();
        let shared_files = match file_choice {
            Some(index) if index < files.len() => {
                println!("Compartilhando arquivo: {}", files[index].1.display());
                vec![files[index].1.to_string_lossy().to_string()]
            }
            _ => {
                println!("Índice inválido. Nenhum arquivo será compartilhado.");
                vec![]
            }
        };

        let peer_port: u16 = 6882 + rand::random::<u16>() % 1000;
        let peer = Arc::new(Peer::new(
            "127.0.0.1".to_string(),
            peer_port,
            shared_files.clone(),
            peer_name.clone(),
        ));

        if let Err(e) = peer.register_with_tracker("127.0.0.1", 6881).await {
            eprintln!("Erro ao registrar com o tracker: {}", e);
            return;
        }

        let peer_clone = Arc::clone(&peer);
        tokio::spawn(async move {
            if let Err(e) = peer_clone.start_server().await {
                eprintln!("Erro ao iniciar o servidor do peer: {}", e);
            }
        });

        // Criando canal para chat
        let (sender, receiver) = mpsc::channel(100);
        let chat_server = ChatServer::new(sender);

        tokio::spawn(async move {
            if let Err(e) = chat_server.start_chat_server(peer_port + 1000).await {
                eprintln!("Erro ao iniciar servidor de chat: {}", e);
            }
        });

        tokio::spawn(async move {
            message_receiver(receiver).await;
        });

        // Interface de comandos do terminal
        println!("\nComandos disponíveis:");
        println!("- 'list': Lista peers conectados");
        println!("- 'files': Lista arquivos disponíveis na rede");
        println!("- 'chat': Inicia chat com outro peer");
        println!("- 'download': Baixa um arquivo");
        println!("- 'exit': Sair");

        loop {
            print!("> ");
            io::stdout().flush().unwrap();
            let mut command = String::new();
            io::stdin().read_line(&mut command).unwrap();
            let command = command.trim();

            match command {
                "list" => {
                    match peer.get_peers_from_tracker("127.0.0.1", 6881).await {
                        Ok(peers) => println!("Peers conectados: {:?}", peers),
                        Err(e) => eprintln!("Erro ao obter peers: {}", e),
                    }
                }
                "files" => {
                    match peer.get_peers_from_tracker("127.0.0.1", 6881).await {
                        Ok(peers) => match peer.list_network_files(peers).await {
                            Ok(files) => {
                                println!("\nArquivos disponíveis na rede:");
                                for (index, (file, peer_addr)) in files.iter().enumerate() {
                                    println!("{}: {} (disponível em {})", index, file, peer_addr);
                                }
                            }
                            Err(e) => eprintln!("Erro ao listar arquivos: {}", e),
                        },
                        Err(e) => eprintln!("Erro ao obter peers: {}", e),
                    }
                }
                "chat" => {
                    print!("Digite a porta do peer para iniciar o chat: ");
                    io::stdout().flush().unwrap();
                    let mut target_port_str = String::new();
                    io::stdin().read_line(&mut target_port_str).unwrap();

                    if let Ok(target_port) = target_port_str.trim().parse::<u16>() {
                        if let Err(e) = start_chat_client(target_port + 1000).await {
                            eprintln!("Erro ao iniciar chat: {}", e);
                        }
                    } else {
                        eprintln!("Porta inválida!");
                    }
                }
                "download" => {
                    match peer.get_peers_from_tracker("127.0.0.1", 6881).await {
                        Ok(peers) => match peer.list_network_files(peers.clone()).await {
                            Ok(files) => {
                                if files.is_empty() {
                                    println!("Nenhum arquivo disponível para download.");
                                    continue;
                                }
                                println!("\nArquivos disponíveis:");
                                for (index, (file, peer_addr)) in files.iter().enumerate() {
                                    println!("{}: {} (em {})", index, file, peer_addr);
                                }

                                print!("\nDigite o número do arquivo que deseja baixar: ");
                                io::stdout().flush().unwrap();
                                let mut choice = String::new();
                                io::stdin().read_line(&mut choice).unwrap();

                                if let Ok(index) = choice.trim().parse::<usize>() {
                                    if index < files.len() {
                                        let (file_name, peer_addr) = &files[index];
                                        println!("Baixando {} de {}", file_name, peer_addr);
                                        if let Err(e) = peer.download_blocks_from_peers(vec![peer_addr.clone()], file_name).await {
                                            eprintln!("Erro no download: {}", e);
                                        }
                                    } else {
                                        eprintln!("Índice inválido!");
                                    }
                                } else {
                                    eprintln!("Entrada inválida!");
                                }
                            }
                            Err(e) => eprintln!("Erro ao listar arquivos: {}", e),
                        },
                        Err(e) => eprintln!("Erro ao obter peers: {}", e),
                    }
                }
                "exit" => {
                    let _ = peer.unregister_from_tracker("127.0.0.1", 6881).await;
                    println!("Desconectando...");
                    break;
                }
                _ => println!("Comando inválido!"),
            }
        }
    }
}
