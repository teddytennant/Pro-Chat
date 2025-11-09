// Pro-Chat - AI Chat Application with Full Keyboard Navigation
// Built for techies who love keyboard shortcuts

class ProChat {
    constructor() {
        this.conversations = this.loadConversations();
        this.currentConversationId = this.loadCurrentConversationId();
        this.apiKey = localStorage.getItem('openai_api_key') || '';
        this.model = localStorage.getItem('openai_model') || 'gpt-4';
        this.isLoading = false;
        this.sidebarOpen = localStorage.getItem('sidebar_open') !== 'false';
        
        this.elements = {
            sidebar: document.getElementById('sidebar'),
            conversationsList: document.getElementById('conversations-list'),
            newChatBtn: document.getElementById('new-chat-btn'),
            toggleSidebarBtn: document.getElementById('toggle-sidebar-btn'),
            sidebarToggleMobile: document.getElementById('sidebar-toggle-mobile'),
            conversationTitle: document.getElementById('conversation-title'),
            chatContainer: document.getElementById('chat-container'),
            userInput: document.getElementById('user-input'),
            sendBtn: document.getElementById('send-btn'),
            deleteChatBtn: document.getElementById('delete-chat-btn'),
            exportBtn: document.getElementById('export-btn'),
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
        this.updateSidebarState();
        
        // Initialize or create first conversation
        if (!this.currentConversationId || !this.conversations[this.currentConversationId]) {
            this.createNewConversation();
        } else {
            this.loadConversation(this.currentConversationId);
        }
        
        this.renderConversationsList();
        this.updateStatus('ready', 'Ready');
        this.elements.userInput.focus();
    }
    
    loadConversations() {
        const stored = localStorage.getItem('conversations');
        return stored ? JSON.parse(stored) : {};
    }
    
    saveConversations() {
        localStorage.setItem('conversations', JSON.stringify(this.conversations));
    }
    
    loadCurrentConversationId() {
        return localStorage.getItem('current_conversation_id');
    }
    
    saveCurrentConversationId(id) {
        localStorage.setItem('current_conversation_id', id);
        this.currentConversationId = id;
    }
    
    createNewConversation() {
        const id = 'conv_' + Date.now();
        const conversation = {
            id: id,
            title: 'New Conversation',
            messages: [],
            created: Date.now(),
            updated: Date.now()
        };
        
        this.conversations[id] = conversation;
        this.saveConversations();
        this.saveCurrentConversationId(id);
        this.loadConversation(id);
        this.renderConversationsList();
        this.elements.userInput.focus();
    }
    
    loadConversation(id) {
        const conversation = this.conversations[id];
        if (!conversation) {
            this.createNewConversation();
            return;
        }
        
        this.currentConversationId = id;
        this.saveCurrentConversationId(id);
        
        // Update title
        this.elements.conversationTitle.textContent = conversation.title;
        
        // Clear and reload messages
        this.elements.chatContainer.innerHTML = '';
        
        if (conversation.messages.length === 0) {
            this.elements.chatContainer.innerHTML = `
                <div class="welcome-message">
                    <p>Welcome to Pro-Chat! A lightweight AI chat interface built for keyboard navigation.</p>
                    <p>Type your message below and press <kbd>Enter</kbd> to send.</p>
                </div>
            `;
        } else {
            conversation.messages.forEach(msg => {
                this.renderMessage(msg.role, msg.content, msg.timestamp);
            });
            this.scrollToBottom();
        }
        
        this.renderConversationsList();
    }
    
    deleteConversation(id) {
        if (!confirm('Are you sure you want to delete this conversation?')) {
            return;
        }
        
        delete this.conversations[id];
        this.saveConversations();
        
        // If deleting current conversation, switch to another or create new
        if (id === this.currentConversationId) {
            const remainingIds = Object.keys(this.conversations);
            if (remainingIds.length > 0) {
                this.loadConversation(remainingIds[0]);
            } else {
                this.createNewConversation();
            }
        }
        
        this.renderConversationsList();
    }
    
    updateConversationTitle(id, title) {
        if (this.conversations[id]) {
            this.conversations[id].title = title;
            this.conversations[id].updated = Date.now();
            this.saveConversations();
            this.renderConversationsList();
        }
    }
    
    renderConversationsList() {
        const sortedConversations = Object.values(this.conversations)
            .sort((a, b) => b.updated - a.updated);
        
        this.elements.conversationsList.innerHTML = '';
        
        sortedConversations.forEach(conv => {
            const item = document.createElement('div');
            item.className = 'conversation-item' + (conv.id === this.currentConversationId ? ' active' : '');
            item.setAttribute('role', 'listitem');
            item.setAttribute('data-conversation-id', conv.id);
            
            const title = document.createElement('div');
            title.className = 'conversation-title-text';
            title.textContent = conv.title;
            
            const meta = document.createElement('div');
            meta.className = 'conversation-meta';
            meta.textContent = `${conv.messages.length} msgs • ${this.formatDate(conv.updated)}`;
            
            const deleteBtn = document.createElement('button');
            deleteBtn.className = 'conversation-delete-btn';
            deleteBtn.innerHTML = '×';
            deleteBtn.setAttribute('aria-label', 'Delete conversation');
            deleteBtn.onclick = (e) => {
                e.stopPropagation();
                this.deleteConversation(conv.id);
            };
            
            item.appendChild(title);
            item.appendChild(meta);
            item.appendChild(deleteBtn);
            
            item.onclick = () => this.loadConversation(conv.id);
            
            this.elements.conversationsList.appendChild(item);
        });
    }
    
    formatDate(timestamp) {
        const date = new Date(timestamp);
        const now = new Date();
        const diff = now - date;
        
        // Less than 1 minute
        if (diff < 60000) {
            return 'Just now';
        }
        // Less than 1 hour
        if (diff < 3600000) {
            const mins = Math.floor(diff / 60000);
            return `${mins}m ago`;
        }
        // Less than 24 hours
        if (diff < 86400000) {
            const hours = Math.floor(diff / 3600000);
            return `${hours}h ago`;
        }
        // Less than 7 days
        if (diff < 604800000) {
            const days = Math.floor(diff / 86400000);
            return `${days}d ago`;
        }
        // Otherwise show date
        return date.toLocaleDateString();
    }
    
    formatTime(timestamp) {
        const date = new Date(timestamp);
        return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }
    
    exportConversation() {
        const conversation = this.conversations[this.currentConversationId];
        if (!conversation) return;
        
        let text = `${conversation.title}\n`;
        text += `Created: ${new Date(conversation.created).toLocaleString()}\n`;
        text += `Messages: ${conversation.messages.length}\n`;
        text += `\n${'='.repeat(50)}\n\n`;
        
        conversation.messages.forEach(msg => {
            const time = new Date(msg.timestamp).toLocaleString();
            const role = msg.role === 'user' ? 'You' : 'Assistant';
            text += `[${time}] ${role}:\n${msg.content}\n\n`;
        });
        
        // Create and download file
        const blob = new Blob([text], { type: 'text/plain' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `${conversation.title.replace(/[^a-z0-9]/gi, '_')}_${Date.now()}.txt`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        
        this.updateStatus('ready', 'Conversation exported!');
        setTimeout(() => this.updateStatus('ready', 'Ready'), 2000);
    }
    
    updateSidebarState() {
        if (this.sidebarOpen) {
            this.elements.sidebar.classList.add('open');
            this.elements.toggleSidebarBtn.textContent = '◀';
        } else {
            this.elements.sidebar.classList.remove('open');
            this.elements.toggleSidebarBtn.textContent = '▶';
        }
        localStorage.setItem('sidebar_open', this.sidebarOpen);
    }
    
    toggleSidebar() {
        this.sidebarOpen = !this.sidebarOpen;
        this.updateSidebarState();
    }
    
    setupEventListeners() {
        // New conversation
        this.elements.newChatBtn.addEventListener('click', () => this.createNewConversation());
        
        // Toggle sidebar
        this.elements.toggleSidebarBtn.addEventListener('click', () => this.toggleSidebar());
        this.elements.sidebarToggleMobile.addEventListener('click', () => this.toggleSidebar());
        
        // Conversation title editing
        this.elements.conversationTitle.addEventListener('blur', () => {
            const newTitle = this.elements.conversationTitle.textContent.trim();
            if (newTitle && newTitle !== this.conversations[this.currentConversationId]?.title) {
                this.updateConversationTitle(this.currentConversationId, newTitle);
            } else {
                this.elements.conversationTitle.textContent = this.conversations[this.currentConversationId]?.title || 'New Conversation';
            }
        });
        
        this.elements.conversationTitle.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                this.elements.conversationTitle.blur();
            }
            if (e.key === 'Escape') {
                e.preventDefault();
                this.elements.conversationTitle.textContent = this.conversations[this.currentConversationId]?.title || 'New Conversation';
                this.elements.conversationTitle.blur();
            }
        });
        
        // Send message
        this.elements.sendBtn.addEventListener('click', () => this.sendMessage());
        
        // Delete conversation
        this.elements.deleteChatBtn.addEventListener('click', () => this.deleteConversation(this.currentConversationId));
        
        // Export conversation
        this.elements.exportBtn.addEventListener('click', () => this.exportConversation());
        
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
        
        // New conversation - Ctrl+N or Cmd+N
        if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
            e.preventDefault();
            this.createNewConversation();
            return;
        }
        
        // Delete conversation - Ctrl+D or Cmd+D
        if ((e.ctrlKey || e.metaKey) && e.key === 'd') {
            e.preventDefault();
            this.deleteConversation(this.currentConversationId);
            return;
        }
        
        // Clear chat - Ctrl+L or Cmd+L
        if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
            e.preventDefault();
            this.clearCurrentChat();
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
        
        const conversation = this.conversations[this.currentConversationId];
        if (!conversation) return;
        
        // Clear input
        this.elements.userInput.value = '';
        
        // Add user message
        const timestamp = Date.now();
        conversation.messages.push({ role: 'user', content: message, timestamp });
        this.renderMessage('user', message, timestamp);
        
        // Update conversation
        conversation.updated = timestamp;
        
        // Update title if this is the first message
        if (conversation.messages.length === 1) {
            const title = message.substring(0, 50) + (message.length > 50 ? '...' : '');
            this.updateConversationTitle(this.currentConversationId, title);
            this.elements.conversationTitle.textContent = title;
        }
        
        this.saveConversations();
        
        // Show loading state
        this.isLoading = true;
        this.updateStatus('loading', 'Thinking...');
        this.elements.sendBtn.disabled = true;
        
        // Add loading message
        const loadingMsgId = this.addLoadingMessage();
        
        try {
            const response = await this.callOpenAI();
            
            // Remove loading message
            const loadingMsg = document.querySelector(`[data-message-id="${loadingMsgId}"]`);
            if (loadingMsg) {
                loadingMsg.remove();
            }
            
            // Add assistant response
            const responseTimestamp = Date.now();
            conversation.messages.push({ role: 'assistant', content: response, timestamp: responseTimestamp });
            conversation.updated = responseTimestamp;
            this.saveConversations();
            this.renderMessage('assistant', response, responseTimestamp);
            this.updateStatus('ready', 'Ready');
            this.renderConversationsList();
            
        } catch (error) {
            // Remove loading message
            const loadingMsg = document.querySelector(`[data-message-id="${loadingMsgId}"]`);
            if (loadingMsg) {
                loadingMsg.remove();
            }
            
            this.renderMessage('assistant', `Error: ${error.message}`, Date.now());
            this.updateStatus('error', `Error: ${error.message}`);
        } finally {
            this.isLoading = false;
            this.elements.sendBtn.disabled = false;
            this.elements.userInput.focus();
        }
    }
    
    async callOpenAI() {
        const conversation = this.conversations[this.currentConversationId];
        if (!conversation) return;
        
        // Build conversation history
        const messages = [
            { role: 'system', content: 'You are a helpful AI assistant. Provide clear, concise responses suitable for technical users.' },
            ...conversation.messages.map(msg => ({
                role: msg.role,
                content: msg.content
            }))
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
    
    renderMessage(role, content, timestamp) {
        // Remove welcome message if it exists
        const welcomeMsg = this.elements.chatContainer.querySelector('.welcome-message');
        if (welcomeMsg) {
            welcomeMsg.remove();
        }
        
        // Create message element
        const messageDiv = document.createElement('div');
        messageDiv.className = `message ${role}`;
        
        const headerDiv = document.createElement('div');
        headerDiv.className = 'message-header';
        
        const roleSpan = document.createElement('span');
        roleSpan.textContent = role === 'user' ? 'You' : 'Assistant';
        
        const timeSpan = document.createElement('span');
        timeSpan.className = 'message-time';
        timeSpan.textContent = this.formatTime(timestamp);
        
        headerDiv.appendChild(roleSpan);
        headerDiv.appendChild(timeSpan);
        
        const contentDiv = document.createElement('div');
        contentDiv.className = 'message-content';
        contentDiv.textContent = content;
        
        const actionsDiv = document.createElement('div');
        actionsDiv.className = 'message-actions';
        
        const copyBtn = document.createElement('button');
        copyBtn.className = 'message-action-btn';
        copyBtn.innerHTML = '📋 Copy';
        copyBtn.title = 'Copy message';
        copyBtn.onclick = () => this.copyMessageContent(content);
        
        actionsDiv.appendChild(copyBtn);
        
        messageDiv.appendChild(headerDiv);
        messageDiv.appendChild(contentDiv);
        messageDiv.appendChild(actionsDiv);
        
        this.elements.chatContainer.appendChild(messageDiv);
        
        // Scroll to bottom
        this.scrollToBottom();
    }
    
    addLoadingMessage() {
        const messageId = Date.now() + Math.random();
        
        // Remove welcome message if it exists
        const welcomeMsg = this.elements.chatContainer.querySelector('.welcome-message');
        if (welcomeMsg) {
            welcomeMsg.remove();
        }
        
        // Create message element
        const messageDiv = document.createElement('div');
        messageDiv.className = 'message assistant loading';
        messageDiv.setAttribute('data-message-id', messageId);
        
        const headerDiv = document.createElement('div');
        headerDiv.className = 'message-header';
        headerDiv.textContent = 'Assistant';
        
        const contentDiv = document.createElement('div');
        contentDiv.className = 'message-content';
        contentDiv.textContent = '';
        
        messageDiv.appendChild(headerDiv);
        messageDiv.appendChild(contentDiv);
        
        this.elements.chatContainer.appendChild(messageDiv);
        
        // Scroll to bottom
        this.scrollToBottom();
        
        return messageId;
    }
    
    copyMessageContent(content) {
        navigator.clipboard.writeText(content).then(() => {
            this.updateStatus('ready', 'Message copied to clipboard!');
            setTimeout(() => this.updateStatus('ready', 'Ready'), 2000);
        }).catch(() => {
            this.updateStatus('error', 'Failed to copy message');
            setTimeout(() => this.updateStatus('ready', 'Ready'), 2000);
        });
    }
    
    clearCurrentChat() {
        if (!confirm('Clear all messages in this conversation?')) {
            return;
        }
        
        const conversation = this.conversations[this.currentConversationId];
        if (conversation) {
            conversation.messages = [];
            conversation.updated = Date.now();
            this.saveConversations();
            this.loadConversation(this.currentConversationId);
        }
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
