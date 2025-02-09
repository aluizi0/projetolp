mod tracker;
mod peer;
mod file_utils;
mod chat;

use std::env;
use tokio::runtime::Runtime;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Uso: cargo run -- (tracker | peer)");
        return;
    }

    let mode = args[1].as_str();

    match mode {
        "tracker" => {
            let rt = Runtime::new().unwrap();
            rt.block_on(tracker::start_tracker());
        }
        "peer" => {
            let rt = Runtime::new().unwrap();
            rt.block_on(peer::start_peer());
        }
        _ => {
            println!("Modo inválido! Use 'tracker' ou 'peer'.");
        }
    }
}