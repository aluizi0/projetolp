# AluBox - P2P File Sharing and Chat

## Descrição

AluBox é um cliente P2P desenvolvido em Rust. Ele permite que você compartilhe e baixe arquivos em uma rede P2P, além de oferecer um sistema de chat entre peers.

## Requisitos

- Rust (instale via [rustup](https://rustup.rs/))
- Cargo (gerenciador de pacotes do Rust)

## Instalação

Clone o repositório:

```sh
git clone https://github.com/aluizi0/projetolp.git
cd projetolp/alubox

#Instale as dependências:

cargo build

#Para iniciar o tracker, execute:

cargo run -- tracker

#Para iniciar um peer, execute:

cargo run -- peer
```