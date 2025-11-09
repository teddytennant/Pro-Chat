import { useState, useEffect, useRef } from 'react'

function App() {
  const [chats, setChats] = useState([])
  const [currentChatId, setCurrentChatId] = useState(null)
  const [messages, setMessages] = useState([])
  const [apiKey, setApiKey] = useState(localStorage.getItem('openrouter_api_key') || '')
  const [model, setModel] = useState(localStorage.getItem('openrouter_model') || 'x-ai/grok-4')
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
    if (chats.length > 0) {
      localStorage.setItem('pro_chat_history', JSON.stringify(chats))
    }
  }, [chats])

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
    const conversation = [
      { role: 'system', content: 'You are a helpful AI assistant. Provide clear, concise responses suitable for technical users.' },
      ...conversationMessages.map(msg => ({ role: msg.role, content: msg.content }))
    ]

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
    setShowSettings(false)
    setStatus({ state: 'ready', text: 'Settings saved!' })
    setTimeout(() => setStatus({ state: 'ready', text: 'Ready' }), 2000)
    inputRef.current?.focus()
  }

  return (
    <div className="flex h-screen bg-black text-gray-100 overflow-hidden">
      {/* Sidebar */}
      <div className={`${sidebarOpen ? 'w-64' : 'w-0'} transition-all duration-300 bg-zinc-900 border-r border-zinc-800 flex flex-col overflow-hidden`}>
        <div className="p-4 border-b border-zinc-800 flex items-center justify-between">
          <h2 className="text-lg font-semibold text-white">Chats</h2>
          <button
            onClick={createNewChat}
            className="p-2 hover:bg-zinc-800 rounded transition-colors"
            title="New Chat (Ctrl+N)"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
            </svg>
          </button>
        </div>
        
        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          {chats.map(chat => (
            <div
              key={chat.id}
              onClick={() => switchChat(chat.id)}
              className={`p-3 rounded-lg cursor-pointer group flex items-center justify-between ${
                currentChatId === chat.id ? 'bg-zinc-800' : 'hover:bg-zinc-800/50'
              }`}
            >
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium text-white truncate">{chat.title}</div>
                <div className="text-xs text-gray-500 mt-1">
                  {new Date(chat.updatedAt).toLocaleDateString()}
                </div>
              </div>
              <button
                onClick={(e) => deleteChat(chat.id, e)}
                className="opacity-0 group-hover:opacity-100 p-1 hover:bg-zinc-700 rounded transition-all ml-2"
              >
                <svg className="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
            </div>
          ))}
          
          {chats.length === 0 && (
            <div className="text-center text-gray-500 text-sm py-8">
              No chats yet<br />
              Press Ctrl+N to start
            </div>
          )}
        </div>

        <div className="p-4 border-t border-zinc-800">
          <button
            onClick={() => setShowSettings(true)}
            className="w-full p-2 bg-zinc-800 hover:bg-zinc-700 rounded-lg font-medium transition-colors duration-200 flex items-center justify-center space-x-2"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
            <span>Settings</span>
          </button>
        </div>
      </div>

      {/* Main Chat Area */}
      <div className="flex-1 flex flex-col">
        {/* Header */}
        <header className="bg-zinc-900 border-b border-zinc-800">
          <div className="px-6 py-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-3">
                <button
                  onClick={() => setSidebarOpen(prev => !prev)}
                  className="p-2 hover:bg-zinc-800 rounded transition-colors"
                  title="Toggle Sidebar (Ctrl+B)"
                >
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
                  </svg>
                </button>
                <div>
                  <h1 className="text-xl font-bold text-white">Pro-Chat</h1>
                  <p className="text-xs text-gray-400 mt-0.5">
                    Press <kbd className="px-1.5 py-0.5 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">?</kbd> for shortcuts
                  </p>
                </div>
              </div>
              <div className="flex items-center space-x-2">
                <button
                  onClick={() => setVoiceEnabled(prev => !prev)}
                  className={`p-2 rounded transition-colors ${voiceEnabled ? 'bg-white text-black' : 'bg-zinc-800 hover:bg-zinc-700'}`}
                  title={`Voice Output: ${voiceEnabled ? 'ON' : 'OFF'}`}
                >
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                  </svg>
                </button>
                <button
                  onClick={() => setShowShortcuts(true)}
                  className="p-2 rounded bg-zinc-800 hover:bg-zinc-700 transition-colors"
                  title="Keyboard Shortcuts (?)"
                >
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        </header>

        {/* Chat Container */}
        <div className="flex-1 overflow-hidden">
          <div
            ref={chatContainerRef}
            className="h-full overflow-y-auto px-6 py-8 space-y-6"
          >
            {messages.length === 0 ? (
              <div className="text-center py-16">
                <div className="max-w-md mx-auto">
                  <div className="w-16 h-16 bg-zinc-800 rounded-full mx-auto mb-6 flex items-center justify-center border border-zinc-700">
                    <svg className="w-8 h-8 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                    </svg>
                  </div>
                  <h2 className="text-2xl font-semibold text-white mb-3">Start a Conversation</h2>
                  <p className="text-gray-400 mb-6">
                    Type your message or press <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+Shift+V</kbd> for voice input
                  </p>
                </div>
              </div>
            ) : (
              messages.map((msg, index) => (
                <div key={index} className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                  <div className={`max-w-[80%] ${msg.role === 'user' ? 'order-2' : 'order-1'}`}>
                    <div className={`flex items-start space-x-3 ${msg.role === 'user' ? 'flex-row-reverse space-x-reverse' : ''}`}>
                      <div className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium flex-shrink-0 ${
                        msg.role === 'user' 
                          ? 'bg-white text-black' 
                          : 'bg-zinc-800 text-gray-300 border border-zinc-700'
                      }`}>
                        {msg.role === 'user' ? 'U' : 'AI'}
                      </div>
                      <div className={`rounded-2xl px-4 py-3 ${
                        msg.role === 'user'
                          ? 'bg-white text-black'
                          : 'bg-zinc-900 border border-zinc-800 text-gray-200'
                      }`}>
                        <div className="text-sm font-medium mb-1 opacity-75">
                          {msg.role === 'user' ? 'You' : 'Assistant'}
                        </div>
                        <div className="whitespace-pre-wrap leading-relaxed">{msg.content}</div>
                      </div>
                    </div>
                  </div>
                </div>
              ))
            )}
            {isLoading && (
              <div className="flex justify-start">
                <div className="max-w-[80%]">
                  <div className="flex items-start space-x-3">
                    <div className="w-8 h-8 rounded-full bg-zinc-800 border border-zinc-700 flex items-center justify-center text-sm font-medium text-gray-300">
                      AI
                    </div>
                    <div className="bg-zinc-900 border border-zinc-800 rounded-2xl px-4 py-3">
                      <div className="text-sm font-medium mb-1 opacity-75 text-gray-300">Assistant</div>
                      <div className="flex items-center space-x-2">
                        <div className="flex space-x-1">
                          <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce"></div>
                          <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style={{animationDelay: '0.1s'}}></div>
                          <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style={{animationDelay: '0.2s'}}></div>
                        </div>
                        <span className="text-gray-400">Thinking...</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Input Area */}
        <div className="bg-zinc-900 border-t border-zinc-800">
          <div className="px-6 py-4">
            <div className="space-y-4">
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
                  className="w-full p-4 pr-12 bg-black border border-zinc-800 rounded-xl resize-none focus:outline-none focus:ring-2 focus:ring-white focus:border-white transition-all duration-200 placeholder-gray-500 text-white"
                  placeholder="Type your message... (Enter to send, Shift+Enter for new line)"
                  rows={3}
                  disabled={isLoading}
                />
                <button
                  onClick={toggleVoice}
                  disabled={!recognitionRef.current}
                  className={`absolute right-3 top-3 p-2 rounded-lg transition-colors ${
                    isListening 
                      ? 'bg-white text-black animate-pulse' 
                      : 'bg-zinc-800 hover:bg-zinc-700 text-gray-300'
                  } disabled:opacity-50 disabled:cursor-not-allowed`}
                  title="Voice Input (Ctrl+Shift+V)"
                >
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
                  </svg>
                </button>
              </div>
              <div className="flex justify-between items-center">
                <button
                  onClick={clearChat}
                  className="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 rounded-lg font-medium transition-colors duration-200 flex items-center space-x-2"
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                  <span>Clear</span>
                  <kbd className="px-2 py-1 bg-zinc-700 border border-zinc-600 rounded text-xs font-mono">Ctrl+L</kbd>
                </button>
                <button
                  onClick={sendMessage}
                  disabled={isLoading || !inputValue.trim()}
                  className="px-6 py-2 bg-white text-black hover:bg-gray-200 disabled:bg-zinc-800 disabled:text-gray-600 disabled:cursor-not-allowed rounded-lg font-medium transition-all duration-200 flex items-center space-x-2"
                >
                  <span>{isLoading ? 'Sending...' : 'Send'}</span>
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* Status Bar */}
        <div className="bg-zinc-900 border-t border-zinc-800 px-6 py-2">
          <div className="flex justify-between items-center text-sm">
            <span className={`font-medium ${
              status.state === 'error' ? 'text-gray-400' : 
              status.state === 'loading' ? 'text-gray-400' : 
              'text-gray-500'
            }`}>
              {status.text}
            </span>
            <div className="flex items-center space-x-4">
              {voiceEnabled && (
                <span className="text-gray-500 text-xs flex items-center">
                  <svg className="w-3 h-3 mr-1" fill="currentColor" viewBox="0 0 20 20">
                    <path d="M10 12a2 2 0 100-4 2 2 0 000 4z"/>
                    <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-11a1 1 0 10-2 0v4a1 1 0 002 0V7z" clipRule="evenodd"/>
                  </svg>
                  Voice ON
                </span>
              )}
              <span className="text-gray-500 font-mono text-xs">{model.split('/').pop()?.toUpperCase()}</span>
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
                  className="text-gray-400 hover:text-white transition-colors duration-200"
                  onClick={() => setShowShortcuts(false)}
                >
                  <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              <div className="space-y-3">
                <div className="flex items-center justify-between py-2">
                  <span className="text-gray-300">Show help</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">?</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-gray-300">Focus input</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+K</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-gray-300">New chat</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+N</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-gray-300">Clear chat</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+L</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-gray-300">Toggle sidebar</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+B</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-gray-300">Voice input</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+Shift+V</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-gray-300">Open settings</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Ctrl+,</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-gray-300">Send message</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Enter</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-gray-300">New line</span>
                  <kbd className="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs font-mono">Shift+Enter</kbd>
                </div>
                <div className="flex items-center justify-between py-2">
                  <span className="text-gray-300">Close modal</span>
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
                  className="text-gray-400 hover:text-white transition-colors duration-200"
                  onClick={() => setShowSettings(false)}
                >
                  <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              <div className="space-y-6">
                <div>
                  <label className="block text-sm font-medium text-gray-300 mb-2">OpenRouter API Key</label>
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    className="w-full p-3 bg-black border border-zinc-800 rounded-lg focus:outline-none focus:ring-2 focus:ring-white focus:border-white transition-all duration-200 placeholder-gray-600 text-white"
                    placeholder="sk-or-v1-..."
                  />
                  <p className="text-xs text-gray-500 mt-1">Your API key is stored locally and only sent to OpenRouter. Get your key at <a href="https://openrouter.ai/keys" target="_blank" rel="noopener noreferrer" className="text-gray-400 hover:underline">openrouter.ai/keys</a></p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-300 mb-2">Model</label>
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
                <button
                  onClick={saveSettings}
                  className="w-full px-4 py-3 bg-white text-black hover:bg-gray-200 rounded-lg font-medium transition-all duration-200"
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
