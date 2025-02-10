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

// 🔹 URL do Tracker
const TRACKER_URL = "http://127.0.0.1:9500";

// 🔹 Garante que o usuário inicie na tela de registro
document.addEventListener("DOMContentLoaded", async () => {
    const storedName = localStorage.getItem("peerName");

    if (storedName) {
        // Verifica se o peer ainda está registrado no Tracker
        await checkPeerStatus(storedName);
    } else {
        // Mantém na tela de registro
        registerScreen.classList.remove("hidden");
        mainScreen.classList.add("hidden");
    }
});

// 🔹 Verifica se o Peer ainda está registrado no Tracker
async function checkPeerStatus(peerName) {
    try {
        const res = await fetch(`${TRACKER_URL}/list`);
        const peers = await res.json();
        const isRegistered = peers.some(p => p.name === peerName);

        if (isRegistered) {
            startSession(peerName);
        } else {
            console.warn(`⚠️ Peer '${peerName}' não encontrado. Redirecionando para registro.`);
            localStorage.removeItem("peerName");
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

    registerBtn.disabled = true; // Evita múltiplos cliques

    try {
        const res = await fetch(`${TRACKER_URL}/register`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ name: peerName, address: "127.0.0.1:8000" }), // ⚠️ Ajuste o endereço dinamicamente se necessário
        });

        if (!res.ok) {
            const errorText = await res.text();
            throw new Error(`Erro do servidor: ${errorText}`);
        }

        localStorage.setItem("peerName", peerName);
        startSession(peerName);
    } catch (error) {
        console.error("❌ Erro ao iniciar Peer:", error);
        alert(`❌ Erro ao iniciar Peer: ${error.message}`);
    } finally {
        registerBtn.disabled = false; // Reativa o botão após resposta
    }
});

// 🔹 Ativa a tela principal
function startSession(peerName) {
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

// 🟢 Implementação dos botões principais
shareBtn.addEventListener("click", () => {
    alert("⚡ Função de compartilhamento ainda será implementada!");
});

downloadBtn.addEventListener("click", () => {
    alert("⬇️ Função de download ainda será implementada!");
});

listBtn.addEventListener("click", loadPeers);

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

    // 🔹 Remove o nome salvo e volta para a tela inicial
    localStorage.removeItem("peerName");
    registerScreen.classList.remove("hidden");
    mainScreen.classList.add("hidden");
});
