mod tracker;
mod peer;
mod file_utils;
mod chat;

use std::env;
use tokio::runtime::Runtime;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("❌ Uso: cargo run -- (tracker | peer)");
        return;
    }

    let mode = args[1].as_str();

    let rt = Runtime::new().expect("❌ Falha ao iniciar o runtime do Tokio");

    match mode {
        "tracker" => {
            println!("🚀 Iniciando Tracker...");
            rt.block_on(tracker::start_tracker()); // ❌ REMOVIDO `if let Err(e) =`
        }
        "peer" => {
            println!("📡 Iniciando Peer...");
            rt.block_on(peer::start_peer()); // ❌ REMOVIDO `if let Err(e) =`
        }
        _ => {
            eprintln!("❌ Modo inválido! Use 'tracker' ou 'peer'.");
        }
    }
}