import "./style.css";

const registerScreen = document.getElementById("register-screen");
const mainScreen = document.getElementById("main-screen");
const peerNameInput = document.getElementById("peer-name");
const registerBtn = document.getElementById("register-btn");
const userNameSpan = document.getElementById("user-name");
const peersList = document.getElementById("peers");

// Botões da tela principal
const shareBtn = document.getElementById("btn-share");
const downloadBtn = document.getElementById("btn-download");
const listBtn = document.getElementById("btn-list");
const exitBtn = document.getElementById("btn-exit");
const fileInput = document.createElement("input");
fileInput.type = "file";
fileInput.style.display = "none";
document.body.appendChild(fileInput);

// 🔹 URL do Tracker
const TRACKER_URL = "http://127.0.0.1:9500";

// 🔹 Garante que o usuário inicie na tela de registro
document.addEventListener("DOMContentLoaded", async () => {
    const storedName = localStorage.getItem("peerName");
    const storedAddress = localStorage.getItem("peerAddress");

    if (storedName && storedAddress) {
        startSession(storedName, storedAddress);
    } else {
        registerScreen.classList.remove("hidden");
        mainScreen.classList.add("hidden");
    }
});

// 🔹 Verifica se o Peer ainda está registrado no Tracker
async function checkPeerStatus(peerName) {
    try {
        const res = await fetch(`${TRACKER_URL}/list`);
        const peers = await res.json();
        const peer = peers.find(p => p.name === peerName);

        if (peer) {
            localStorage.setItem("peerAddress", `http://${peer.address}`);
            startSession(peer.name, `http://${peer.address}`);
        } else {
            console.warn(`⚠️ Peer '${peerName}' não encontrado. Redirecionando para registro.`);
            localStorage.removeItem("peerName");
            localStorage.removeItem("peerAddress");
            registerScreen.classList.remove("hidden");
            mainScreen.classList.add("hidden");
        }
    } catch (error) {
        console.error("❌ Erro ao verificar status do Peer:", error);
    }
}

// 🔹 Quando clicar em "Entrar", inicia o Peer no Tracker
registerBtn.addEventListener("click", async () => {
    const peerName = peerNameInput.value.trim();
    if (!peerName) return alert("Digite um nome válido!");
    registerBtn.disabled = true;

    try {
        // 🔹 Gera uma porta aleatória entre 8000 e 9000
        const peerPort = Math.floor(Math.random() * 1000) + 8000;
        const peerAddress = `127.0.0.1:${peerPort}`;

        // 🔹 Registra o Peer no Tracker com nome e endereço
        const res = await fetch(`${TRACKER_URL}/register`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ name: peerName, address: peerAddress }),
        });

        if (!res.ok) {
            const errorText = await res.text();
            throw new Error(`Erro do servidor: ${errorText}`);
        }

        // 🔹 Aguarda um curto período para garantir que o Tracker atualizou a lista
        await new Promise(resolve => setTimeout(resolve, 1000));

        // 🔹 Obtém o endereço atualizado do Peer
        const peerRes = await fetch(`${TRACKER_URL}/list`);
        const peers = await peerRes.json();
        const peer = peers.find(p => p.name === peerName);

        if (!peer) {
            throw new Error("Peer registrado, mas não encontrado na lista!");
        }

        const peerFullAddress = `http://${peer.address}`;
        localStorage.setItem("peerName", peerName);
        localStorage.setItem("peerAddress", peerFullAddress);

        startSession(peerName, peerFullAddress);
    } catch (error) {
        console.error("❌ Erro ao iniciar Peer:", error);
        alert(`❌ Erro ao iniciar Peer: ${error.message}`);
    } finally {
        registerBtn.disabled = false;
    }
});

// 🔹 Ativa a tela principal
function startSession(peerName, peerAddress) {
    registerScreen.classList.add("hidden");
    mainScreen.classList.remove("hidden");
    userNameSpan.textContent = peerName;
    loadPeers();
}

// 🔄 Atualiza a lista de peers automaticamente
async function loadPeers() {
    try {
        const res = await fetch(`${TRACKER_URL}/list`);
        if (!res.ok) throw new Error(`Erro HTTP ${res.status}`);

        const peers = await res.json();
        peersList.innerHTML = peers.length > 0
            ? peers.map(p => `<li>${p.name} (${p.address})</li>`).join("")
            : "<li>Nenhum peer registrado.</li>";
    } catch (error) {
        console.error("❌ Erro ao carregar peers:", error);
        peersList.innerHTML = "<li>Erro ao carregar peers...</li>";
    }
}

// 🔹 Atualiza a lista de peers a cada 5 segundos
setInterval(loadPeers, 5000);

// 🟢 Implementação do botão de compartilhamento
shareBtn.addEventListener("click", () => fileInput.click());

fileInput.addEventListener("change", async () => {
    if (fileInput.files.length === 0) return;

    const file = fileInput.files[0];
    console.log("📂 Arquivo selecionado:", file.name);

    const peerAddress = localStorage.getItem("peerAddress");
    if (!peerAddress) {
        alert("⚠️ Erro: Endereço do Peer não encontrado!");
        return;
    }

    try {
        console.log(`📡 Enviando arquivo para ${peerAddress}/share`);

        // Criando um FormData para envio do arquivo
        const formData = new FormData();
        formData.append("file", file, file.name);

        const response = await fetch(`${peerAddress}/share`, {
            method: "POST",
            body: formData,
        });

        if (!response.ok) {
            const errorText = await response.text();
            console.error("❌ Erro ao compartilhar arquivo:", errorText);
            throw new Error(`Erro ao compartilhar: ${errorText}`);
        }

        alert("✅ Arquivo compartilhado com sucesso!");
    } catch (error) {
        console.error("❌ Erro ao compartilhar arquivo:", error);
        alert("❌ Falha ao compartilhar o arquivo!");
    }
});

// 🔹 Botão "Sair" - Remove o Peer do Tracker e volta para o registro
exitBtn.addEventListener("click", async () => {
    const peerName = localStorage.getItem("peerName");
    if (!peerName) return;

    const confirmExit = confirm("Tem certeza que deseja sair?");
    if (!confirmExit) return;

    try {
        const res = await fetch(`${TRACKER_URL}/unregister_peer`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ peer: peerName }),
        });

        if (!res.ok) {
            console.error("❌ Erro ao remover peer do Tracker:", await res.text());
        }
    } catch (error) {
        console.error("❌ Erro ao se desconectar do Tracker:", error);
    }

    // 🔹 Remove os dados salvos e volta para a tela inicial
    localStorage.removeItem("peerName");
    localStorage.removeItem("peerAddress");
    registerScreen.classList.remove("hidden");
    mainScreen.classList.add("hidden");
});