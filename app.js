// Pro-Chat - AI Chat Application with Full Keyboard Navigation
// Built for techies who love keyboard shortcuts

class ProChat {
    constructor() {
        this.messages = [];
        this.apiKey = localStorage.getItem('openai_api_key') || '';
        this.model = localStorage.getItem('openai_model') || 'gpt-4';
        this.isLoading = false;
        
        this.elements = {
            chatContainer: document.getElementById('chat-container'),
            userInput: document.getElementById('user-input'),
            sendBtn: document.getElementById('send-btn'),
            clearBtn: document.getElementById('clear-btn'),
            status: document.getElementById('status'),
            modelInfo: document.getElementById('model-info'),
            shortcutsModal: document.getElementById('shortcuts-modal'),
            settingsModal: document.getElementById('settings-modal'),
            settingsBtn: document.getElementById('settings-btn'),
            apiKeyInput: document.getElementById('api-key'),
            modelSelect: document.getElementById('model-select'),
            saveSettingsBtn: document.getElementById('save-settings')
        };
        
        this.init();
    }
    
    init() {
        this.setupEventListeners();
        this.loadSettings();
        this.updateStatus('ready', 'Ready');
        this.elements.userInput.focus();
    }
    
    setupEventListeners() {
        // Send message
        this.elements.sendBtn.addEventListener('click', () => this.sendMessage());
        
        // Clear chat
        this.elements.clearBtn.addEventListener('click', () => this.clearChat());
        
        // Settings
        this.elements.settingsBtn.addEventListener('click', () => this.openSettings());
        this.elements.saveSettingsBtn.addEventListener('click', () => this.saveSettings());
        
        // Modal close buttons
        document.querySelectorAll('.modal-close').forEach(btn => {
            btn.addEventListener('click', (e) => {
                e.target.closest('.modal').classList.remove('active');
                e.target.closest('.modal').setAttribute('aria-hidden', 'true');
                this.elements.userInput.focus();
            });
        });
        
        // Click outside modal to close
        document.querySelectorAll('.modal').forEach(modal => {
            modal.addEventListener('click', (e) => {
                if (e.target === modal) {
                    modal.classList.remove('active');
                    modal.setAttribute('aria-hidden', 'true');
                    this.elements.userInput.focus();
                }
            });
        });
        
        // Keyboard shortcuts
        document.addEventListener('keydown', (e) => this.handleKeyboardShortcuts(e));
        
        // Input textarea handling
        this.elements.userInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                this.sendMessage();
            }
        });
    }
    
    handleKeyboardShortcuts(e) {
        // Help modal - ?
        if (e.key === '?' && !this.isModalOpen()) {
            e.preventDefault();
            this.openShortcuts();
            return;
        }
        
        // Close modal - Escape
        if (e.key === 'Escape') {
            const activeModal = document.querySelector('.modal.active');
            if (activeModal) {
                e.preventDefault();
                activeModal.classList.remove('active');
                activeModal.setAttribute('aria-hidden', 'true');
                this.elements.userInput.focus();
            }
            return;
        }
        
        // Focus input - Ctrl+K or Cmd+K
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
            e.preventDefault();
            this.elements.userInput.focus();
            return;
        }
        
        // Clear chat - Ctrl+L or Cmd+L
        if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
            e.preventDefault();
            this.clearChat();
            return;
        }
        
        // Settings - Ctrl+, or Cmd+,
        if ((e.ctrlKey || e.metaKey) && e.key === ',') {
            e.preventDefault();
            this.openSettings();
            return;
        }
        
        // Scroll chat with arrow keys when chat container is focused
        if (document.activeElement === this.elements.chatContainer) {
            if (e.key === 'ArrowUp') {
                e.preventDefault();
                this.elements.chatContainer.scrollBy(0, -50);
            } else if (e.key === 'ArrowDown') {
                e.preventDefault();
                this.elements.chatContainer.scrollBy(0, 50);
            }
        }
    }
    
    isModalOpen() {
        return document.querySelector('.modal.active') !== null;
    }
    
    openShortcuts() {
        this.elements.shortcutsModal.classList.add('active');
        this.elements.shortcutsModal.setAttribute('aria-hidden', 'false');
        this.elements.shortcutsModal.querySelector('.modal-close').focus();
    }
    
    openSettings() {
        this.elements.settingsModal.classList.add('active');
        this.elements.settingsModal.setAttribute('aria-hidden', 'false');
        this.elements.apiKeyInput.focus();
    }
    
    loadSettings() {
        this.elements.apiKeyInput.value = this.apiKey;
        this.elements.modelSelect.value = this.model;
        this.elements.modelInfo.textContent = this.model.toUpperCase();
    }
    
    saveSettings() {
        this.apiKey = this.elements.apiKeyInput.value.trim();
        this.model = this.elements.modelSelect.value;
        
        if (this.apiKey) {
            localStorage.setItem('openai_api_key', this.apiKey);
        }
        localStorage.setItem('openai_model', this.model);
        
        this.elements.modelInfo.textContent = this.model.toUpperCase();
        this.elements.settingsModal.classList.remove('active');
        this.elements.settingsModal.setAttribute('aria-hidden', 'true');
        this.updateStatus('ready', 'Settings saved!');
        this.elements.userInput.focus();
        
        setTimeout(() => {
            this.updateStatus('ready', 'Ready');
        }, 2000);
    }
    
    async sendMessage() {
        const message = this.elements.userInput.value.trim();
        
        if (!message || this.isLoading) {
            return;
        }
        
        if (!this.apiKey) {
            this.updateStatus('error', 'Please set your API key in settings');
            this.openSettings();
            return;
        }
        
        // Clear input
        this.elements.userInput.value = '';
        
        // Add user message
        this.addMessage('user', message);
        
        // Show loading state
        this.isLoading = true;
        this.updateStatus('loading', 'Thinking...');
        this.elements.sendBtn.disabled = true;
        
        // Add loading message
        const loadingMsgId = this.addMessage('assistant', '', true);
        
        try {
            const response = await this.callOpenAI(message);
            
            // Remove loading message
            const loadingMsg = document.querySelector(`[data-message-id="${loadingMsgId}"]`);
            if (loadingMsg) {
                loadingMsg.remove();
            }
            
            // Add assistant response
            this.addMessage('assistant', response);
            this.updateStatus('ready', 'Ready');
            
        } catch (error) {
            // Remove loading message
            const loadingMsg = document.querySelector(`[data-message-id="${loadingMsgId}"]`);
            if (loadingMsg) {
                loadingMsg.remove();
            }
            
            this.addMessage('assistant', `Error: ${error.message}`);
            this.updateStatus('error', `Error: ${error.message}`);
        } finally {
            this.isLoading = false;
            this.elements.sendBtn.disabled = false;
            this.elements.userInput.focus();
        }
    }
    
    async callOpenAI(message) {
        // Build conversation history
        const messages = [
            { role: 'system', content: 'You are a helpful AI assistant. Provide clear, concise responses suitable for technical users.' },
            ...this.messages.map(msg => ({
                role: msg.role,
                content: msg.content
            })),
            { role: 'user', content: message }
        ];
        
        const response = await fetch('https://api.openai.com/v1/chat/completions', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${this.apiKey}`
            },
            body: JSON.stringify({
                model: this.model,
                messages: messages,
                temperature: 0.7,
                max_tokens: 2000
            })
        });
        
        if (!response.ok) {
            const error = await response.json();
            throw new Error(error.error?.message || 'API request failed');
        }
        
        const data = await response.json();
        return data.choices[0].message.content;
    }
    
    addMessage(role, content, isLoading = false) {
        const messageId = Date.now() + Math.random();
        
        // Store in history (not loading messages)
        if (!isLoading) {
            this.messages.push({ role, content });
        }
        
        // Remove welcome message if it exists
        const welcomeMsg = this.elements.chatContainer.querySelector('.welcome-message');
        if (welcomeMsg) {
            welcomeMsg.remove();
        }
        
        // Create message element
        const messageDiv = document.createElement('div');
        messageDiv.className = `message ${role}${isLoading ? ' loading' : ''}`;
        messageDiv.setAttribute('data-message-id', messageId);
        
        const headerDiv = document.createElement('div');
        headerDiv.className = 'message-header';
        headerDiv.textContent = role === 'user' ? 'You' : 'Assistant';
        
        const contentDiv = document.createElement('div');
        contentDiv.className = 'message-content';
        contentDiv.textContent = content;
        
        messageDiv.appendChild(headerDiv);
        messageDiv.appendChild(contentDiv);
        
        this.elements.chatContainer.appendChild(messageDiv);
        
        // Scroll to bottom
        this.scrollToBottom();
        
        return messageId;
    }
    
    clearChat() {
        this.messages = [];
        this.elements.chatContainer.innerHTML = `
            <div class="welcome-message">
                <p>Chat cleared. Ready for a new conversation!</p>
            </div>
        `;
        this.updateStatus('ready', 'Ready');
        this.elements.userInput.focus();
    }
    
    scrollToBottom() {
        this.elements.chatContainer.scrollTop = this.elements.chatContainer.scrollHeight;
    }
    
    updateStatus(state, text) {
        this.elements.status.textContent = text;
        this.elements.status.className = `status-text ${state}`;
    }
}

// Initialize the app when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    window.proChat = new ProChat();
});
