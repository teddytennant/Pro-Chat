import { useState, useEffect, useRef } from 'react'

function App() {
  const [chats, setChats] = useState([])
  const [currentChatId, setCurrentChatId] = useState(null)
  const [messages, setMessages] = useState([])
  const [apiKey, setApiKey] = useState(localStorage.getItem('openrouter_api_key') || '')
  const [model, setModel] = useState(localStorage.getItem('openrouter_model') || 'x-ai/grok-4')
  const [contextEnabled, setContextEnabled] = useState(localStorage.getItem('context_enabled') !== 'false')
  const [memoryEnabled, setMemoryEnabled] = useState(localStorage.getItem('memory_enabled') === 'true')
  const [isLoading, setIsLoading] = useState(false)
  const [status, setStatus] = useState({ state: 'ready', text: 'Ready' })
  const [showShortcuts, setShowShortcuts] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [inputValue, setInputValue] = useState('')
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [voiceEnabled, setVoiceEnabled] = useState(false)
  const [isListening, setIsListening] = useState(false)

  const chatContainerRef = useRef(null)
  const inputRef = useRef(null)
  const recognitionRef = useRef(null)

  // Load chats from localStorage
  useEffect(() => {
    const savedChats = localStorage.getItem('pro_chat_history')
    if (savedChats) {
      const parsedChats = JSON.parse(savedChats)
      setChats(parsedChats)
      if (parsedChats.length > 0) {
        const lastChat = parsedChats[0]
        setCurrentChatId(lastChat.id)
        setMessages(lastChat.messages || [])
      }
    }
  }, [])

  // Save chats to localStorage
  useEffect(() => {
    if (chats.length > 0 && memoryEnabled) {
      localStorage.setItem('pro_chat_history', JSON.stringify(chats))
    } else if (!memoryEnabled) {
      localStorage.removeItem('pro_chat_history')
    }
  }, [chats, memoryEnabled])

  // Update current chat messages
  useEffect(() => {
    if (currentChatId) {
      setChats(prevChats => 
        prevChats.map(chat => 
          chat.id === currentChatId 
            ? { ...chat, messages, updatedAt: Date.now() }
            : chat
        )
      )
    }
  }, [messages, currentChatId])

  // Initialize speech recognition
  useEffect(() => {
    if ('webkitSpeechRecognition' in window || 'SpeechRecognition' in window) {
      const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition
      recognitionRef.current = new SpeechRecognition()
      recognitionRef.current.continuous = false
      recognitionRef.current.interimResults = false
      
      recognitionRef.current.onresult = (event) => {
        const transcript = event.results[0][0].transcript
        setInputValue(prev => prev + ' ' + transcript)
        setIsListening(false)
      }
      
      recognitionRef.current.onerror = () => {
        setIsListening(false)
        setStatus({ state: 'error', text: 'Voice recognition error' })
      }
      
      recognitionRef.current.onend = () => {
        setIsListening(false)
      }
    }
  }, [])

  useEffect(() => {
    inputRef.current?.focus()
  }, [])

  useEffect(() => {
    const handleKeyDown = (e) => {
      // Ignore if typing in input
      if (document.activeElement === inputRef.current && !e.ctrlKey && !e.metaKey) {
        return
      }

      if (e.key === '?' && !showShortcuts && !showSettings) {
        e.preventDefault()
        setShowShortcuts(true)
      } else if (e.key === 'Escape') {
        setShowShortcuts(false)
        setShowSettings(false)
        inputRef.current?.focus()
      } else if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault()
        inputRef.current?.focus()
      } else if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
        e.preventDefault()
        clearChat()
      } else if ((e.ctrlKey || e.metaKey) && e.key === ',') {
        e.preventDefault()
        setShowSettings(true)
      } else if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
        e.preventDefault()
        createNewChat()
      } else if ((e.ctrlKey || e.metaKey) && e.key === 'b') {
        e.preventDefault()
        setSidebarOpen(prev => !prev)
      } else if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'V') {
        e.preventDefault()
        toggleVoice()
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [showShortcuts, showSettings])

  const createNewChat = () => {
    const newChat = {
      id: Date.now().toString(),
      title: 'New Chat',
      messages: [],
      createdAt: Date.now(),
      updatedAt: Date.now()
    }
    setChats(prev => [newChat, ...prev])
    setCurrentChatId(newChat.id)
    setMessages([])
    setInputValue('')
    inputRef.current?.focus()
  }

  const switchChat = (chatId) => {
    const chat = chats.find(c => c.id === chatId)
    if (chat) {
      setCurrentChatId(chatId)
      setMessages(chat.messages || [])
      inputRef.current?.focus()
    }
  }

  const deleteChat = (chatId, e) => {
    e.stopPropagation()
    const newChats = chats.filter(c => c.id !== chatId)
    setChats(newChats)
    
    if (currentChatId === chatId) {
      if (newChats.length > 0) {
        setCurrentChatId(newChats[0].id)
        setMessages(newChats[0].messages || [])
      } else {
        setCurrentChatId(null)
        setMessages([])
      }
    }
  }

  const updateChatTitle = (chatId, firstMessage) => {
    setChats(prev =>
      prev.map(chat =>
        chat.id === chatId && chat.title === 'New Chat'
          ? { ...chat, title: firstMessage.slice(0, 50) + (firstMessage.length > 50 ? '...' : '') }
          : chat
      )
    )
  }

  const sendMessage = async () => {
    const message = inputValue.trim()
    if (!message || isLoading) return

    if (!apiKey) {
      setStatus({ state: 'error', text: 'Please set your API key in settings' })
      setShowSettings(true)
      return
    }

    // Create new chat if none exists
    if (!currentChatId) {
      createNewChat()
      // Wait for state to update
      setTimeout(() => sendMessage(), 100)
      return
    }

    setInputValue('')
    const newMessages = [...messages, { role: 'user', content: message }]
    setMessages(newMessages)
    
    // Update chat title if it's the first message
    if (messages.length === 0) {
      updateChatTitle(currentChatId, message)
    }

    setIsLoading(true)
    setStatus({ state: 'loading', text: 'Thinking...' })

    try {
      const response = await callAPI(message, newMessages)
      setMessages(prev => [...prev, { role: 'assistant', content: response }])
      setStatus({ state: 'ready', text: 'Ready' })
      
      // Speak response if voice enabled
      if (voiceEnabled) {
        speakText(response)
      }
    } catch (error) {
      setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${error.message}` }])
      setStatus({ state: 'error', text: `Error: ${error.message}` })
    } finally {
      setIsLoading(false)
      inputRef.current?.focus()
    }
  }

  const callAPI = async (message, conversationMessages) => {
    // Build conversation based on context settings
    let conversation = [
      { role: 'system', content: 'You are a helpful AI assistant. Provide clear, concise responses suitable for technical users.' }
    ]
    
    if (contextEnabled) {
      // Include full conversation history
      conversation = [
        ...conversation,
        ...conversationMessages.map(msg => ({ role: msg.role, content: msg.content }))
      ]
    } else {
      // Only include the latest user message
      const lastMessage = conversationMessages[conversationMessages.length - 1]
      if (lastMessage) {
        conversation.push({ role: lastMessage.role, content: lastMessage.content })
      }
    }

    const response = await fetch('https://openrouter.ai/api/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${apiKey}`,
        'HTTP-Referer': window.location.origin,
        'X-Title': 'Pro-Chat'
      },
      body: JSON.stringify({
        model,
        messages: conversation,
        temperature: 0.7,
        max_tokens: 2000
      })
    })

    if (!response.ok) {
      const error = await response.json()
      throw new Error(error.error?.message || 'API request failed')
    }

    const data = await response.json()
    return data.choices[0].message.content
  }

  const speakText = (text) => {
    if ('speechSynthesis' in window) {
      const utterance = new SpeechSynthesisUtterance(text)
      utterance.rate = 1.0
      utterance.pitch = 1.0
      window.speechSynthesis.speak(utterance)
    }
  }

  const toggleVoice = () => {
    if (isListening) {
      recognitionRef.current?.stop()
      setIsListening(false)
    } else {
      recognitionRef.current?.start()
      setIsListening(true)
      setStatus({ state: 'loading', text: 'Listening...' })
    }
  }

  const clearChat = () => {
    if (currentChatId) {
      setMessages([])
      setChats(prev =>
        prev.map(chat =>
          chat.id === currentChatId
            ? { ...chat, messages: [], title: 'New Chat' }
            : chat
        )
      )
    }
    setStatus({ state: 'ready', text: 'Ready' })
    inputRef.current?.focus()
  }

  const saveSettings = () => {
    localStorage.setItem('openrouter_api_key', apiKey)
    localStorage.setItem('openrouter_model', model)
    localStorage.setItem('context_enabled', contextEnabled.toString())
    localStorage.setItem('memory_enabled', memoryEnabled.toString())
    setShowSettings(false)
    setStatus({ state: 'ready', text: 'Settings saved!' })
    setTimeout(() => setStatus({ state: 'ready', text: 'Ready' }), 2000)
    inputRef.current?.focus()
  }

  return (
    <div className="flex h-screen bg-neutral-900 text-neutral-100 overflow-hidden">
      {/* Sidebar */}
      <div className={`${sidebarOpen ? 'w-64' : 'w-0'} transition-all duration-300 bg-neutral-800 text-neutral-100 flex flex-col overflow-hidden border-r border-neutral-700`}>
        <div className="p-3 flex items-center justify-between">
          <button
            onClick={createNewChat}
            className="flex-1 px-4 py-2.5 bg-neutral-700 hover:bg-neutral-600 rounded-lg transition-all duration-200 flex items-center justify-center space-x-2 text-sm font-medium"
            title="New Chat (Ctrl+N)"
          >
            <span>New chat</span>
          </button>
        </div>
        
        <div className="flex-1 overflow-y-auto px-2 py-2 space-y-1 scrollbar-thin scrollbar-thumb-neutral-700 scrollbar-track-transparent">
          {chats.map(chat => (
            <div
              key={chat.id}
              onClick={() => switchChat(chat.id)}
              className={`px-3 py-2.5 rounded-lg cursor-pointer group flex items-center justify-between transition-all duration-150 ${
                currentChatId === chat.id 
                  ? 'bg-neutral-700' 
                  : 'hover:bg-neutral-700/50'
              }`}
            >
              <div className="flex-1 min-w-0 flex items-center space-x-2">
                <div className="flex-1 min-w-0">
                  <div className="text-sm text-neutral-200 truncate">{chat.title}</div>
                </div>
              </div>
              <button
                onClick={(e) => deleteChat(chat.id, e)}
                className="opacity-0 group-hover:opacity-100 px-2 py-1 hover:bg-neutral-600 rounded transition-all ml-1 text-neutral-400 text-sm"
              >
                ×
              </button>
            </div>
          ))}
          
          {chats.length === 0 && (
            <div className="text-center text-neutral-500 text-sm py-8 px-4">
              No previous chats
            </div>
          )}
        </div>

        <div className="p-3 border-t border-neutral-700">
          <button
            onClick={() => setShowSettings(true)}
            className="w-full px-3 py-2.5 hover:bg-neutral-700 rounded-lg transition-all duration-200 flex items-center justify-center text-sm"
          >
            <span>Settings</span>
          </button>
        </div>
      </div>

      {/* Main Chat Area */}
      <div className="flex-1 flex flex-col bg-neutral-900">
        {/* Header */}
        <header className="border-b border-neutral-700 bg-neutral-800/80 backdrop-blur-lg sticky top-0 z-10">
          <div className="max-w-4xl mx-auto px-4 h-14 flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <button
                onClick={() => setSidebarOpen(prev => !prev)}
                className="px-3 py-2 hover:bg-neutral-700 rounded-lg transition-colors text-neutral-200 font-medium"
                title="Toggle Sidebar (Ctrl+B)"
              >
                ☰
              </button>
              <h1 className="text-lg font-semibold text-neutral-100">Pro-Chat</h1>
            </div>
            <div className="flex items-center space-x-1">
              <button
                onClick={() => setShowShortcuts(true)}
                className="px-3 py-2 hover:bg-neutral-700 rounded-lg transition-colors text-neutral-200 font-medium"
                title="Keyboard Shortcuts (?)"
              >
                ?
              </button>
            </div>
          </div>
        </header>

        {/* Chat Container */}
        <div className="flex-1 overflow-hidden bg-neutral-900">
          <div
            ref={chatContainerRef}
            className="h-full overflow-y-auto scrollbar-thin scrollbar-thumb-neutral-300 scrollbar-track-transparent"
          >
            <div className="max-w-3xl mx-auto px-4">
              {messages.length === 0 ? (
                <div className="flex items-center justify-center min-h-[calc(100vh-200px)]">
                  <div className="text-center max-w-md">
                    <h2 className="text-3xl font-semibold text-neutral-100 mb-3">How can I help you today?</h2>
                    <p className="text-neutral-400">
                      Start a conversation by typing a message below
                    </p>
                  </div>
                </div>
              ) : (
                <div className="py-8 space-y-6">
                  {messages.map((msg, index) => (
                    <div key={index} className="group">
                      <div className={`flex items-start space-x-4 ${msg.role === 'user' ? '' : 'bg-neutral-800 -mx-4 px-4 py-6'}`}>
                        <div className={`w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 text-sm font-semibold ${
                          msg.role === 'user' 
                            ? 'bg-neutral-700 text-neutral-100' 
                            : 'bg-neutral-600 text-neutral-100'
                        }`}>
                          {msg.role === 'user' ? 'Y' : 'AI'}
                        </div>
                        <div className="flex-1 min-w-0 pt-1">
                          <div className="text-[15px] leading-7 text-neutral-100 whitespace-pre-wrap break-words">
                            {msg.content}
                          </div>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
              {isLoading && (
                <div className="py-6 bg-neutral-800 -mx-4 px-4">
                  <div className="flex items-start space-x-4">
                    <div className="w-8 h-8 rounded-full bg-neutral-600 flex items-center justify-center text-sm font-semibold text-neutral-100">
                      AI
                    </div>
                    <div className="flex-1 pt-1">
                      <div className="flex items-center space-x-2">
                        <div className="flex space-x-1">
                          <div className="w-2 h-2 bg-neutral-500 rounded-full animate-bounce"></div>
                          <div className="w-2 h-2 bg-neutral-500 rounded-full animate-bounce" style={{animationDelay: '0.15s'}}></div>
                          <div className="w-2 h-2 bg-neutral-500 rounded-full animate-bounce" style={{animationDelay: '0.3s'}}></div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              )}
              <div className="h-32"></div>
            </div>
          </div>
        </div>

        {/* Input Area */}
        <div className="border-t border-neutral-700 bg-neutral-800">
          <div className="max-w-3xl mx-auto px-4 py-6">
            <div className="relative">
              <textarea
                ref={inputRef}
                value={inputValue}
                onChange={(e) => setInputValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault()
                    sendMessage()
                  }
                }}
                className="w-full px-4 py-3 pr-12 bg-neutral-700 border border-neutral-600 rounded-2xl resize-none focus:outline-none focus:border-neutral-500 focus:ring-1 focus:ring-neutral-500 transition-all duration-200 placeholder-neutral-400 text-neutral-100 shadow-sm"
                placeholder="Message Pro-Chat..."
                rows={1}
                style={{minHeight: '52px', maxHeight: '200px'}}
                disabled={isLoading}
              />
              <button
                onClick={sendMessage}
                disabled={isLoading || !inputValue.trim()}
                className="absolute right-3 bottom-3 w-10 h-10 bg-neutral-600 hover:bg-neutral-500 disabled:bg-neutral-700 disabled:cursor-not-allowed rounded-full transition-all duration-200 flex items-center justify-center text-xl font-bold"
                title="Send message"
              >
                <span className={isLoading || !inputValue.trim() ? 'text-neutral-500' : 'text-neutral-100'}>↑</span>
              </button>
            </div>
            <div className="mt-2 flex items-center justify-between text-xs text-neutral-400">
              <div className="flex items-center space-x-3">
                <span>{model.split('/').pop()?.toUpperCase()}</span>
              </div>
              <div className="flex items-center space-x-2">
                <kbd className="px-2 py-1 bg-neutral-100 border border-neutral-200 rounded text-xs font-mono">Enter</kbd>
                <span>to send</span>
                <kbd className="px-2 py-1 bg-neutral-100 border border-neutral-200 rounded text-xs font-mono">Shift+Enter</kbd>
                <span>for new line</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Shortcuts Modal */}
      {showShortcuts && (
        <div className="fixed inset-0 bg-black/80 flex items-center justify-center z-50 p-4" onClick={() => setShowShortcuts(false)}>
          <div className="bg-zinc-900 border border-zinc-800 rounded-2xl max-w-md w-full" onClick={(e) => e.stopPropagation()}>
            <div className="p-6">
              <div className="flex items-center justify-between mb-6">
                <h2 className="text-xl font-semibold text-white">Keyboard Shortcuts</h2>
                <button 
                  className="text-neutral-400 hover:text-white transition-colors duration-200 text-2xl"
                  onClick={() => setShowShortcuts(false)}
                >
                  ×
                </button>
              </div>
              <div className="space-y-3">
                <div className="flex items-center justify-between py-2">
                  <span className="text-neutral-300">Show help</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">?</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-neutral-300">Focus input</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+K</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-neutral-300">New chat</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+N</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-neutral-300">Clear chat</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+L</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-neutral-300">Toggle sidebar</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+B</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-neutral-300">Open settings</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+,</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-neutral-300">Send message</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Enter</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-neutral-300">New line</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Shift+Enter</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-neutral-300">Close modal</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Escape</kbd>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Settings Modal */}
      {showSettings && (
        <div className="fixed inset-0 bg-black/80 flex items-center justify-center z-50 p-4" onClick={() => setShowSettings(false)}>
          <div className="bg-zinc-900 border border-zinc-800 rounded-2xl max-w-md w-full" onClick={(e) => e.stopPropagation()}>
            <div className="p-6">
              <div className="flex items-center justify-between mb-6">
                <h2 className="text-xl font-semibold text-white">Settings</h2>
                <button 
                  className="text-neutral-400 hover:text-white transition-colors duration-200 text-2xl"
                  onClick={() => setShowSettings(false)}
                >
                  ×
                </button>
              </div>
              <div className="space-y-6">
                <div>
                  <label className="block text-sm font-medium text-neutral-300 mb-2">OpenRouter API Key</label>
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    className="w-full p-3 bg-black border border-zinc-800 rounded-lg focus:outline-none focus:ring-2 focus:ring-white focus:border-white transition-all duration-200 placeholder-neutral-600 text-white"
                    placeholder="sk-or-v1-..."
                  />
                  <p className="text-xs text-neutral-500 mt-1">Your API key is stored locally and only sent to OpenRouter. Get your key at <a href="https://openrouter.ai/keys" target="_blank" rel="noopener noreferrer" className="text-neutral-400 hover:underline">openrouter.ai/keys</a></p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-neutral-300 mb-2">Model</label>
                  <select
                    value={model}
                    onChange={(e) => setModel(e.target.value)}
                    className="w-full p-3 bg-black border border-zinc-800 rounded-lg focus:outline-none focus:ring-2 focus:ring-white focus:border-white transition-all duration-200 text-white"
                  >
                    <option value="x-ai/grok-4">Grok 4</option>
                    <option value="google/gemini-2.5-pro">Gemini 2.5 Pro</option>
                    <option value="anthropic/claude-sonnet-4.5">Claude Sonnet 4.5</option>
                    <option value="anthropic/claude-opus-4.1">Claude Opus 4.1</option>
                    <option value="moonshot/kimi-k2-thinking">Kimi K2 Thinking</option>
                    <option value="minimax/minimax-m2">MiniMax M2</option>
                    <option value="x-ai/grok-4-fast">Grok 4 Fast</option>
                    <option value="deepseek/deepseek-r1">DeepSeek R1</option>
                    <option value="deepseek/deepseek-v3">DeepSeek V3</option>
                    <option value="qwen/qwen-3">Qwen 3</option>
                    <option value="qwen/qwen-3-max">Qwen 3 Max</option>
                    <option value="qwen/qwen-3-vl">Qwen 3 VL</option>
                  </select>
                </div>
                <div>
                  <label className="flex items-center justify-between cursor-pointer">
                    <div>
                      <div className="text-sm font-medium text-neutral-300">Context</div>
                      <p className="text-xs text-neutral-500 mt-0.5">Remember conversation history within each chat</p>
                    </div>
                    <div className="relative">
                      <input
                        type="checkbox"
                        checked={contextEnabled}
                        onChange={(e) => setContextEnabled(e.target.checked)}
                        className="sr-only peer"
                      />
                      <div className="w-11 h-6 bg-zinc-700 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-white rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-neutral-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-white"></div>
                    </div>
                  </label>
                </div>
                <div>
                  <label className="flex items-center justify-between cursor-pointer">
                    <div>
                      <div className="text-sm font-medium text-neutral-300">Memory</div>
                      <p className="text-xs text-neutral-500 mt-0.5">Save chat history across sessions</p>
                    </div>
                    <div className="relative">
                      <input
                        type="checkbox"
                        checked={memoryEnabled}
                        onChange={(e) => setMemoryEnabled(e.target.checked)}
                        className="sr-only peer"
                      />
                      <div className="w-11 h-6 bg-zinc-700 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-white rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-neutral-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-white"></div>
                    </div>
                  </label>
                </div>
                <button
                  onClick={saveSettings}
                  className="w-full px-4 py-3 bg-white text-black hover:bg-neutral-200 rounded-lg font-medium transition-all duration-200"
                >
                  Save Settings
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

export default App
