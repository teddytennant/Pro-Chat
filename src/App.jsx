import { useState, useEffect, useRef } from 'react'
import ReactMarkdown from 'react-markdown'

function App() {
  const [chats, setChats] = useState(() => {
    const saved = localStorage.getItem('chat_history')
    return saved ? JSON.parse(saved) : [{ id: Date.now(), title: 'New Chat', messages: [], createdAt: Date.now() }]
  })
  const [currentChatId, setCurrentChatId] = useState(() => {
    const saved = localStorage.getItem('chat_history')
    return saved ? JSON.parse(saved)[0].id : Date.now()
  })
  const [apiKey, setApiKey] = useState(localStorage.getItem('openrouter_api_key') || '')
  const [model, setModel] = useState(localStorage.getItem('openrouter_model') || 'x-ai/grok-4')
  const [isLoading, setIsLoading] = useState(false)
  const [status, setStatus] = useState({ state: 'ready', text: 'Ready' })
  const [showShortcuts, setShowShortcuts] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [showSidebar, setShowSidebar] = useState(false)
  const [inputValue, setInputValue] = useState('')
  const [searchQuery, setSearchQuery] = useState('')

  const chatContainerRef = useRef(null)
  const inputRef = useRef(null)
  const searchRef = useRef(null)

  const currentChat = chats.find(c => c.id === currentChatId) || chats[0]
  const messages = currentChat?.messages || []

  // Save chats to localStorage whenever they change
  useEffect(() => {
    localStorage.setItem('chat_history', JSON.stringify(chats))
  }, [chats])

  // Save chats to localStorage whenever they change
  useEffect(() => {
    localStorage.setItem('chat_history', JSON.stringify(chats))
  }, [chats])

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (chatContainerRef.current) {
      chatContainerRef.current.scrollTop = chatContainerRef.current.scrollHeight
    }
  }, [messages])

  useEffect(() => {
    inputRef.current.focus()
  }, [])

  useEffect(() => {
    const handleKeyDown = (e) => {
      // Ignore if typing in input or settings
      const isTyping = e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT'
      
      if (e.key === '?' && !showShortcuts && !showSettings && !isTyping) {
        e.preventDefault()
        setShowShortcuts(true)
      } else if (e.key === 'Escape') {
        setShowShortcuts(false)
        setShowSettings(false)
        setShowSidebar(false)
        inputRef.current.focus()
      } else if ((e.ctrlKey || e.metaKey) && e.key === 'k' && !isTyping) {
        e.preventDefault()
        inputRef.current.focus()
      } else if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
        e.preventDefault()
        clearChat()
      } else if ((e.ctrlKey || e.metaKey) && e.key === ',') {
        e.preventDefault()
        setShowSettings(true)
      } else if ((e.ctrlKey || e.metaKey) && e.key === 'h') {
        e.preventDefault()
        setShowSidebar(prev => !prev)
      } else if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
        e.preventDefault()
        createNewChat()
      } else if ((e.ctrlKey || e.metaKey) && e.key === 'd') {
        e.preventDefault()
        deleteCurrentChat()
      } else if ((e.ctrlKey || e.metaKey) && e.key === '/') {
        e.preventDefault()
        if (showSidebar && searchRef.current) {
          searchRef.current.focus()
        } else {
          setShowSidebar(true)
          setTimeout(() => searchRef.current?.focus(), 100)
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key >= '1' && e.key <= '9') {
        e.preventDefault()
        const index = parseInt(e.key) - 1
        if (chats[index]) {
          switchToChat(chats[index].id)
        }
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [showShortcuts, showSettings, showSidebar, chats])

  const sendMessage = async () => {
    const message = inputValue.trim()
    if (!message || isLoading) return

    if (!apiKey) {
      setStatus({ state: 'error', text: 'Please set your API key in settings' })
      setShowSettings(true)
      return
    }

    setInputValue('')
    
    // Update the current chat with the new message
    setChats(prevChats => prevChats.map(chat => 
      chat.id === currentChatId 
        ? { ...chat, messages: [...chat.messages, { role: 'user', content: message }], title: chat.messages.length === 0 ? message.slice(0, 50) : chat.title }
        : chat
    ))
    
    setIsLoading(true)
    setStatus({ state: 'loading', text: 'Thinking...' })

    try {
      const response = await callOpenAI(message)
      setChats(prevChats => prevChats.map(chat => 
        chat.id === currentChatId 
          ? { ...chat, messages: [...chat.messages, { role: 'assistant', content: response }] }
          : chat
      ))
      setStatus({ state: 'ready', text: 'Ready' })
    } catch (error) {
      setChats(prevChats => prevChats.map(chat => 
        chat.id === currentChatId 
          ? { ...chat, messages: [...chat.messages, { role: 'assistant', content: `Error: ${error.message}` }] }
          : chat
      ))
      setStatus({ state: 'error', text: `Error: ${error.message}` })
    } finally {
      setIsLoading(false)
      inputRef.current.focus()
    }
  }

  const callOpenAI = async (message) => {
    const conversation = [
      { role: 'system', content: 'You are a helpful AI assistant. Provide clear, concise responses suitable for technical users. Format your responses using markdown when appropriate.' },
      ...messages.map(msg => ({ role: msg.role, content: msg.content })),
      { role: 'user', content: message }
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

  const createNewChat = () => {
    const newChat = {
      id: Date.now(),
      title: 'New Chat',
      messages: [],
      createdAt: Date.now()
    }
    setChats(prev => [newChat, ...prev])
    setCurrentChatId(newChat.id)
    setInputValue('')
    inputRef.current.focus()
  }

  const switchToChat = (chatId) => {
    setCurrentChatId(chatId)
    setInputValue('')
    setShowSidebar(false)
    inputRef.current.focus()
  }

  const deleteCurrentChat = () => {
    if (chats.length === 1) {
      // Don't delete the last chat, just clear it
      clearChat()
      return
    }
    
    const currentIndex = chats.findIndex(c => c.id === currentChatId)
    const newChats = chats.filter(c => c.id !== currentChatId)
    setChats(newChats)
    
    // Switch to next or previous chat
    const newCurrentChat = newChats[currentIndex] || newChats[currentIndex - 1] || newChats[0]
    setCurrentChatId(newCurrentChat.id)
  }

  const clearChat = () => {
    setChats(prevChats => prevChats.map(chat => 
      chat.id === currentChatId 
        ? { ...chat, messages: [], title: 'New Chat' }
        : chat
    ))
    setStatus({ state: 'ready', text: 'Ready' })
    inputRef.current.focus()
  }

  const saveSettings = () => {
    localStorage.setItem('openrouter_api_key', apiKey)
    localStorage.setItem('openrouter_model', model)
    setShowSettings(false)
    setStatus({ state: 'ready', text: 'Settings saved!' })
    setTimeout(() => setStatus({ state: 'ready', text: 'Ready' }), 2000)
    inputRef.current.focus()
  }

  const filteredChats = chats.filter(chat => 
    chat.title.toLowerCase().includes(searchQuery.toLowerCase())
  )

  return (
    <div className="flex h-screen bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900 text-slate-100">
      {/* Sidebar */}
      <div className={`${showSidebar ? 'w-64' : 'w-0'} transition-all duration-300 overflow-hidden bg-slate-800/30 backdrop-blur-sm border-r border-slate-700/50 flex flex-col`}>
        {showSidebar && (
          <>
            <div className="p-4 border-b border-slate-700/50">
              <button
                onClick={createNewChat}
                className="w-full px-4 py-2 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 rounded-lg font-medium transition-all duration-200 flex items-center justify-center space-x-2 shadow-lg"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                </svg>
                <span>New Chat</span>
              </button>
              <div className="mt-3">
                <input
                  ref={searchRef}
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search chats..."
                  className="w-full px-3 py-2 bg-slate-700/50 border border-slate-600/50 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-400/50 focus:border-blue-400/50 transition-all duration-200 placeholder-slate-400"
                />
              </div>
            </div>
            <div className="flex-1 overflow-y-auto p-2 space-y-1">
              {filteredChats.map((chat, index) => (
                <button
                  key={chat.id}
                  onClick={() => switchToChat(chat.id)}
                  className={`w-full text-left px-3 py-2 rounded-lg transition-all duration-200 group ${
                    chat.id === currentChatId
                      ? 'bg-blue-500/20 border border-blue-400/50'
                      : 'hover:bg-slate-700/50 border border-transparent'
                  }`}
                  title={chat.title}
                >
                  <div className="flex items-center justify-between">
                    <span className="text-sm truncate flex-1">
                      {index < 9 && <kbd className="text-xs text-slate-500 mr-2">⌘{index + 1}</kbd>}
                      {chat.title}
                    </span>
                    <span className="text-xs text-slate-500 ml-2">
                      {chat.messages.length}
                    </span>
                  </div>
                </button>
              ))}
            </div>
          </>
        )}
      </div>

      {/* Main Content */}
      <div className="flex-1 flex flex-col">
      {/* Header */}
      <header className="bg-slate-800/50 backdrop-blur-sm border-b border-slate-700/50 shadow-lg">
        <div className="max-w-5xl mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <button
                onClick={() => setShowSidebar(prev => !prev)}
                className="p-2 rounded-lg bg-slate-700/50 hover:bg-slate-600/50 transition-colors duration-200"
                title="Toggle Sidebar (Ctrl+H)"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
                </svg>
              </button>
              <div>
                <h1 className="text-xl font-bold bg-gradient-to-r from-blue-400 to-purple-400 bg-clip-text text-transparent">
                  {currentChat?.title || 'Pro-Chat'}
                </h1>
                <p className="text-xs text-slate-400 mt-0.5">
                  Press <kbd className="px-1.5 py-0.5 bg-slate-700 border border-slate-600 rounded text-xs font-mono">?</kbd> for shortcuts
                </p>
              </div>
            </div>
            <div className="flex items-center space-x-2">
              <button
                onClick={createNewChat}
                className="p-2 rounded-lg bg-slate-700/50 hover:bg-slate-600/50 transition-colors duration-200"
                title="New Chat (Ctrl+N)"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                </svg>
              </button>
              <button
                onClick={() => setShowSettings(true)}
                className="p-2 rounded-lg bg-slate-700/50 hover:bg-slate-600/50 transition-colors duration-200"
                title="Settings (Ctrl+,)"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
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
          className="h-full overflow-y-auto px-4 py-6"
        >
          <div className="max-w-4xl mx-auto space-y-6">
          {messages.length === 0 ? (
            <div className="text-center py-16">
              <div className="max-w-md mx-auto">
                <div className="w-16 h-16 bg-gradient-to-r from-blue-500 to-purple-500 rounded-2xl mx-auto mb-6 flex items-center justify-center">
                  <svg className="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                  </svg>
                </div>
                <h2 className="text-2xl font-semibold text-slate-200 mb-3">Welcome to Pro-Chat</h2>
                <p className="text-slate-400 mb-6">
                  A clean, keyboard-driven AI chat interface for personal use.
                </p>
                <div className="text-sm text-slate-500 space-y-2">
                  <p>Press <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Ctrl+H</kbd> to toggle chat history</p>
                  <p>Press <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Ctrl+N</kbd> to start a new chat</p>
                  <p>Type your message and press <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Enter</kbd> to send</p>
                </div>
              </div>
            </div>
          ) : (
            messages.map((msg, index) => (
              <div key={index} className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                <div className={`max-w-[85%] flex ${msg.role === 'user' ? 'flex-row-reverse' : 'flex-row'} items-start space-x-3 ${msg.role === 'user' ? 'space-x-reverse' : ''}`}>
                  <div className={`flex-shrink-0 w-8 h-8 rounded-lg flex items-center justify-center text-sm font-semibold ${
                    msg.role === 'user' 
                      ? 'bg-gradient-to-br from-blue-500 to-blue-600 text-white' 
                      : 'bg-gradient-to-br from-purple-500 to-purple-600 text-white'
                  }`}>
                    {msg.role === 'user' ? 'U' : 'AI'}
                  </div>
                  <div className={`rounded-2xl px-4 py-3 shadow-lg ${
                    msg.role === 'user'
                      ? 'bg-gradient-to-br from-blue-500 to-blue-600 text-white'
                      : 'bg-slate-800/70 backdrop-blur-sm border border-slate-700/50 text-slate-100'
                  }`}>
                    {msg.role === 'assistant' ? (
                      <div className="prose prose-invert prose-sm max-w-none">
                        <ReactMarkdown
                          components={{
                            code: ({node, inline, className, children, ...props}) => {
                              return inline ? (
                                <code className="bg-slate-900/50 px-1.5 py-0.5 rounded text-sm font-mono text-blue-300" {...props}>
                                  {children}
                                </code>
                              ) : (
                                <code className="block bg-slate-900/50 p-3 rounded-lg text-sm font-mono overflow-x-auto" {...props}>
                                  {children}
                                </code>
                              )
                            },
                            p: ({children}) => <p className="mb-2 last:mb-0 leading-relaxed">{children}</p>,
                            ul: ({children}) => <ul className="list-disc list-inside mb-2 space-y-1">{children}</ul>,
                            ol: ({children}) => <ol className="list-decimal list-inside mb-2 space-y-1">{children}</ol>,
                            li: ({children}) => <li className="leading-relaxed">{children}</li>,
                            h1: ({children}) => <h1 className="text-xl font-bold mb-2 mt-4 first:mt-0">{children}</h1>,
                            h2: ({children}) => <h2 className="text-lg font-bold mb-2 mt-3 first:mt-0">{children}</h2>,
                            h3: ({children}) => <h3 className="text-base font-bold mb-2 mt-3 first:mt-0">{children}</h3>,
                            blockquote: ({children}) => <blockquote className="border-l-4 border-slate-600 pl-4 italic my-2">{children}</blockquote>,
                          }}
                        >
                          {msg.content}
                        </ReactMarkdown>
                      </div>
                    ) : (
                      <div className="leading-relaxed whitespace-pre-wrap">{msg.content}</div>
                    )}
                  </div>
                </div>
              </div>
            ))
          )}
          {isLoading && (
            <div className="flex justify-start">
              <div className="max-w-[85%] flex items-start space-x-3">
                <div className="flex-shrink-0 w-8 h-8 rounded-lg bg-gradient-to-br from-purple-500 to-purple-600 flex items-center justify-center text-sm font-semibold text-white">
                  AI
                </div>
                <div className="bg-slate-800/70 backdrop-blur-sm border border-slate-700/50 rounded-2xl px-4 py-3 shadow-lg">
                  <div className="flex items-center space-x-2">
                    <div className="flex space-x-1">
                      <div className="w-2 h-2 bg-purple-400 rounded-full animate-bounce"></div>
                      <div className="w-2 h-2 bg-purple-400 rounded-full animate-bounce" style={{animationDelay: '0.1s'}}></div>
                      <div className="w-2 h-2 bg-purple-400 rounded-full animate-bounce" style={{animationDelay: '0.2s'}}></div>
                    </div>
                    <span className="text-slate-300 text-sm">Thinking...</span>
                  </div>
                </div>
              </div>
            </div>
          )}
          </div>
        </div>
      </div>

      {/* Input Area */}
      <div className="bg-slate-800/50 backdrop-blur-sm border-t border-slate-700/50">
        <div className="max-w-4xl mx-auto px-4 py-4">
          <div className="space-y-3">
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
                className="w-full p-4 pr-12 bg-slate-700/50 backdrop-blur-sm border border-slate-600/50 rounded-xl resize-none focus:outline-none focus:ring-2 focus:ring-blue-400/50 focus:border-blue-400/50 transition-all duration-200 placeholder-slate-400"
                placeholder="Message AI... (⏎ send • ⇧⏎ new line)"
                rows={1}
                style={{minHeight: '52px', maxHeight: '200px'}}
                onInput={(e) => {
                  e.target.style.height = 'auto'
                  e.target.style.height = Math.min(e.target.scrollHeight, 200) + 'px'
                }}
                disabled={isLoading}
              />
              <button
                onClick={sendMessage}
                disabled={isLoading || !inputValue.trim()}
                className="absolute right-2 bottom-2 p-2 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 disabled:from-slate-600 disabled:to-slate-700 disabled:cursor-not-allowed rounded-lg transition-all duration-200 shadow-lg"
                title="Send message (Enter)"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
                </svg>
              </button>
            </div>
            <div className="flex justify-between items-center text-xs">
              <div className="flex items-center space-x-2">
                <span className={`font-medium flex items-center ${
                  status.state === 'error' ? 'text-red-400' : 
                  status.state === 'loading' ? 'text-yellow-400' : 
                  'text-green-400'
                }`}>
                  <span className={`w-2 h-2 rounded-full mr-2 ${
                    status.state === 'error' ? 'bg-red-400' : 
                    status.state === 'loading' ? 'bg-yellow-400 animate-pulse' : 
                    'bg-green-400'
                  }`}></span>
                  {status.text}
                </span>
                <span className="text-slate-500">•</span>
                <span className="text-slate-400 font-mono">{model.split('/')[1]?.toUpperCase() || model.toUpperCase()}</span>
              </div>
              <div className="flex items-center space-x-2">
                {messages.length > 0 && (
                  <>
                    <button
                      onClick={deleteCurrentChat}
                      className="text-slate-400 hover:text-red-400 transition-colors duration-200 flex items-center space-x-1"
                      title="Delete chat (Ctrl+D)"
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                      <span>Delete</span>
                    </button>
                    <span className="text-slate-600">•</span>
                  </>
                )}
                <button
                  onClick={clearChat}
                  className="text-slate-400 hover:text-blue-400 transition-colors duration-200 flex items-center space-x-1"
                  title="Clear chat (Ctrl+L)"
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                  <span>Clear</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
      </div>

      {/* Shortcuts Modal */}
      {showShortcuts && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4" onClick={() => setShowShortcuts(false)}>
          <div className="bg-slate-800/90 backdrop-blur-md border border-slate-700/50 rounded-2xl max-w-lg w-full shadow-2xl" onClick={(e) => e.stopPropagation()}>
            <div className="p-6">
              <div className="flex items-center justify-between mb-6">
                <h2 className="text-xl font-semibold text-slate-200">Keyboard Shortcuts</h2>
                <button 
                  className="text-slate-400 hover:text-slate-200 transition-colors duration-200"
                  onClick={() => setShowShortcuts(false)}
                >
                  <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              <div className="space-y-2 max-h-[60vh] overflow-y-auto">
                <div className="grid grid-cols-2 gap-3">
                  <div className="col-span-2 text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">General</div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">Show help</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">?</kbd>
                  </div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">Close modal</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Esc</kbd>
                  </div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">Focus input</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Ctrl+K</kbd>
                  </div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">Settings</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Ctrl+,</kbd>
                  </div>
                  
                  <div className="col-span-2 text-xs font-semibold text-slate-400 uppercase tracking-wider mt-2 mb-1">Chat</div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">Send message</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Enter</kbd>
                  </div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">New line</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Shift+Enter</kbd>
                  </div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">Clear chat</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Ctrl+L</kbd>
                  </div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">Delete chat</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Ctrl+D</kbd>
                  </div>
                  
                  <div className="col-span-2 text-xs font-semibold text-slate-400 uppercase tracking-wider mt-2 mb-1">Navigation</div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">New chat</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Ctrl+N</kbd>
                  </div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">Toggle sidebar</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Ctrl+H</kbd>
                  </div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">Search chats</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Ctrl+/</kbd>
                  </div>
                  <div className="flex items-center justify-between py-2 px-3 rounded-lg bg-slate-700/30">
                    <span className="text-slate-300 text-sm">Switch chat</span>
                    <kbd className="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-xs font-mono">Ctrl+1-9</kbd>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Settings Modal */}
      {showSettings && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4" onClick={() => setShowSettings(false)}>
          <div className="bg-slate-800/80 backdrop-blur-md border border-slate-700/50 rounded-2xl max-w-md w-full shadow-2xl" onClick={(e) => e.stopPropagation()}>
            <div className="p-6">
              <div className="flex items-center justify-between mb-6">
                <h2 className="text-xl font-semibold text-slate-200">Settings</h2>
                <button 
                  className="text-slate-400 hover:text-slate-200 transition-colors duration-200"
                  onClick={() => setShowSettings(false)}
                >
                  <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              <div className="space-y-6">
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-2">OpenRouter API Key</label>
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    className="w-full p-3 bg-slate-700/50 backdrop-blur-sm border border-slate-600/50 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-400/50 focus:border-blue-400/50 transition-all duration-200 placeholder-slate-400"
                    placeholder="sk-or-v1-..."
                  />
                  <p className="text-xs text-slate-500 mt-1">Your API key is stored locally and only sent to OpenRouter. Get your key at <a href="https://openrouter.ai/keys" target="_blank" rel="noopener noreferrer" className="text-blue-400 hover:underline">openrouter.ai/keys</a></p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-2">Model</label>
                  <select
                    value={model}
                    onChange={(e) => setModel(e.target.value)}
                    className="w-full p-3 bg-slate-700/50 backdrop-blur-sm border border-slate-600/50 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-400/50 focus:border-blue-400/50 transition-all duration-200"
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
                  className="w-full px-4 py-3 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 rounded-lg font-medium transition-all duration-200 shadow-lg"
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
