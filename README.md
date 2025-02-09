# AluBox - Sistema P2P de Compartilhamento de Arquivos e Chat

## 📋 Descrição

AluBox é um sistema P2P desenvolvido em Rust que permite compartilhar e baixar arquivos em uma rede P2P, além de oferecer um sistema de chat entre peers.

## 🚀 Funcionalidades

- **Compartilhamento P2P**: Troca de arquivos entre peers
- **Chat em Tempo Real**: Comunicação direta entre peers
- **Verificação de Integridade**: Checksums para validação
- **Monitoramento**: Controle de arquivos e chunks

## ⚙️ Requisitos

- Rust (via [rustup](https://rustup.rs/))
- Cargo
- Portas disponíveis:
  - 9500 (Tracker)
  - 8000-9000 (Peers)

## 📥 Instalação

```sh
# Clone o repositório
git clone https://github.com/aluizi0/projetolp.git
cd projetolp/alubox

# Instale dependências
cargo build
```

## 🎮 Uso

### Iniciar Componentes

```sh
# Tracker (necessário primeiro)
cargo run -- tracker
```
Saída esperada:
```
📡 Tracker rodando na porta 9500...
```

```sh
# Peer
cargo run -- peer
```
Saída esperada:
```
Digite seu nome de usuário: alulol
✅ Peer 'alulol' registrado com sucesso!
📡 Peer 'alulol' rodando em 127.0.0.1:8132
```

### Comandos do Peer

- `share`: Compartilha um arquivo
  - Exemplo:
    ```sh
    📜 Comandos: share | get | list | chat | exit
    share
    Digite o nome do arquivo para compartilhar:
    conteudo.txt
    ✅ Arquivo 'conteudo.txt' dividido em 1 chunk(s).
    ✅ Chunk 'conteudo.txt.chunk0' registrado no Tracker!
    ```

- `get`: Baixa um arquivo
  - Exemplo:
    ```sh
    📜 Comandos: share | get | list | chat | exit
    get
    Digite o nome do arquivo que deseja baixar:
    ```

- `list`: Lista peers e arquivos disponíveis
  - Exemplo:
    ```sh
    📜 Comandos: share | get | list | chat | exit
    list
    📋 Lista de Peers e Arquivos:
    🔹 Peer: alulol (127.0.0.1:8132)
      📄 conteudo.txt
    ```

- `chat`: Inicia chat com outro peer
  - Exemplo:
    ```sh
    📜 Comandos: share | get | list | chat | exit
    chat
    Digite o endereço do peer destinatário (ex: 127.0.0.1:8000): 
    ```

- `exit`: Encerra o peer
  - Exemplo:
    ```sh
    📜 Comandos: share | get | list | chat | exit
    exit
    👋 Saindo...
    👋 Peer 'alulol' removido do Tracker com sucesso!
    ```

## 📚 Documentação

```sh
# Gerar docs
cargo doc

# Abrir no navegador
cargo doc --open
```

## 🔧 Arquitetura

### Componentes
- **Tracker**: Coordena a rede
- **Peer**: Cliente P2P
- **Chat**: Sistema de mensagens

### Características
- Divisão em chunks
- Verificação via checksums
- Download multi-peer
- Monitoramento em tempo real

## 👥 Contribuição

1. Fork o projeto
2. Crie uma branch (`git checkout -b feature/nova-funcao`)
3. Commit (`git commit -am 'Adiciona nova função'`)
4. Push (`git push origin feature/nova-funcao`)
5. Abra Pull Request

## 📝 Licença

MIT License - Veja [LICENSE](LICENSE)

## ✨ Autor

[@aluizi0](https://github.com/aluizi0)