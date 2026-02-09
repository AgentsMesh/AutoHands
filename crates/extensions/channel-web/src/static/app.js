// AutoHands Web Channel Client
const messages = document.getElementById('messages');
const form = document.getElementById('input-form');
const input = document.getElementById('input');
const status = document.getElementById('status');

let ws = null;
let reconnectAttempts = 0;
const maxReconnectAttempts = 5;
const reconnectDelay = 2000;

function connect() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(`${protocol}//${location.host}/ws`);

    ws.onopen = () => {
        console.log('WebSocket connected');
        status.textContent = 'Connected';
        status.className = 'status connected';
        reconnectAttempts = 0;
        input.disabled = false;
    };

    ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            if (data.type === 'message' && data.content) {
                addMessage(data.content, 'assistant');
            }
        } catch (e) {
            console.error('Failed to parse message:', e);
        }
    };

    ws.onclose = () => {
        console.log('WebSocket disconnected');
        status.textContent = 'Disconnected';
        status.className = 'status disconnected';
        input.disabled = true;

        if (reconnectAttempts < maxReconnectAttempts) {
            reconnectAttempts++;
            console.log(`Reconnecting in ${reconnectDelay}ms (attempt ${reconnectAttempts})`);
            setTimeout(connect, reconnectDelay);
        }
    };

    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
    };
}

function addMessage(content, role) {
    const div = document.createElement('div');
    div.className = `message ${role}`;
    div.textContent = content;
    messages.appendChild(div);
    messages.scrollTop = messages.scrollHeight;
}

form.onsubmit = (e) => {
    e.preventDefault();
    const text = input.value.trim();
    if (text && ws && ws.readyState === WebSocket.OPEN) {
        addMessage(text, 'user');
        ws.send(JSON.stringify({ content: text }));
        input.value = '';
    }
};

// Start connection
connect();
