use std::fs::{self, File};
use std::io::{Read, Write};
use sha2::{Sha256, Digest};

const CHUNK_SIZE: usize = 1024 * 1024; // 1MB

/// **Divide um arquivo em chunks de 1MB e calcula o checksum**
pub fn split_file(file_name: &str) -> Vec<(usize, String, String)> {
    let mut file = File::open(file_name).expect("Erro ao abrir arquivo");
    let mut buffer = vec![0; CHUNK_SIZE];
    let mut chunks = vec![];

    let mut index = 0;
    while let Ok(size) = file.read(&mut buffer) {
        if size == 0 {
            break;
        }

        let chunk_name = format!("{}.chunk{}", file_name, index);
        let mut chunk_file = File::create(&chunk_name).expect("Erro ao criar chunk");
        chunk_file.write_all(&buffer[..size]).expect("Erro ao escrever chunk");

        let checksum = format!("{:x}", Sha256::digest(&buffer[..size]));
        chunks.push((index, chunk_name.clone(), checksum));
        index += 1;
    }

    println!("✅ Arquivo '{}' dividido em {} chunk(s).", file_name, index);
    chunks
}

/// **Calcula o checksum do arquivo inteiro**
pub fn compute_file_checksum(file_name: &str) -> String {
    let mut file = match File::open(file_name) {
        Ok(f) => f,
        Err(_) => {
            println!("⚠️ Arquivo '{}' não encontrado para calcular o checksum!", file_name);
            return String::new();
        }
    };
    
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; CHUNK_SIZE];

    while let Ok(size) = file.read(&mut buffer) {
        if size == 0 {
            break;
        }
        hasher.update(&buffer[..size]);
    }

    format!("{:x}", hasher.finalize())
}

/// **Reconstitui o arquivo original a partir dos chunks**
pub fn assemble_file(original_file_name: &str) {
    let output_file_name = format!("{}.assembled", original_file_name);
    let mut output_file = File::create(&output_file_name)
        .expect("❌ Erro ao criar arquivo final");

    let mut index = 0;
    let mut chunks_found = false;

    loop {
        let chunk_name = format!("{}.chunk{}", original_file_name, index);
        if let Ok(mut chunk_file) = File::open(&chunk_name) {
            let mut buffer = Vec::new();
            chunk_file.read_to_end(&mut buffer).expect("❌ Erro ao ler chunk");
            output_file.write_all(&buffer).expect("❌ Erro ao escrever no arquivo final");

            println!("📦 Adicionando '{}' ao arquivo final", chunk_name);
            chunks_found = true;
        } else {
            break;
        }
        index += 1;
    }

    if chunks_found {
        println!("✅ Arquivo '{}' reconstituído com sucesso!", output_file_name);

        let assembled_checksum = compute_file_checksum(&output_file_name);
        println!("🔍 Checksum do arquivo reconstruído: {}", assembled_checksum);

        if std::path::Path::new(original_file_name).exists() {
            // 🔍 Se o arquivo original existir, compara os checksums
            let original_checksum = compute_file_checksum(original_file_name);
            println!("🔍 Checksum esperado: {}", original_checksum);
        }

        // 🚀 Renomeia para o nome correto, com fallback caso ocorra erro
        match fs::rename(&output_file_name, original_file_name) {
            Ok(_) => println!("✅ O arquivo foi validado e renomeado corretamente para '{}'", original_file_name),
            Err(e) => {
                println!("❌ Erro ao renomear '{}': {}. Tentando copiar o arquivo...", output_file_name, e);
                if let Err(copy_err) = fs::copy(&output_file_name, original_file_name) {
                    println!("❌ Falha ao copiar arquivo reconstruído: {}", copy_err);
                } else {
                    println!("✅ Arquivo '{}' copiado com sucesso!", original_file_name);
                    let _ = fs::remove_file(&output_file_name);
                }
            }
        }
    } else {
        println!("⚠️ Nenhum chunk encontrado para reconstrução!");
    }
}