# Sistema P2P-Compartilhamento de Arquivos

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

### Iniciar Tracker
```sh
# Tracker (necessário primeiro)
cargo run -- tracker
```

Saída esperada:
```
📡 Tracker rodando na porta 9500...
```

### Iniciar WebSite
Na pasta frontend
```sh
# WebSite
npm run dev
```

Saída esperada:
```
VITE v6.1.0  ready in 142 ms

  ➜  Local:   http://localhost:.../
  ➜  Network: use --host to expose
  ➜  press h + enter to show help
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
- **File_Utils**: Sistema de Chunks

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
[@GuiHenriqueOlv](https://github.com/GuiHenriqueOlv)
