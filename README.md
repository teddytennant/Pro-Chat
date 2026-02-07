# Pro-Chat

> A personal web development project exploring a lightweight AI chat UI with full keyboard navigation.
>
> ⚠️ **Note**: This was a learning project. For AI development workflows, I'd recommend using CLI tools like **Claude Code**, **Codex CLI**, or **OpenCode** instead, which provide better integration with your development environment.

## Features

- 🎨 **Clean & Minimal Design** - Dark theme with Tailwind CSS
- ⌨️ **Full Keyboard Navigation** - Every action accessible via keyboard
- 🚀 **Lightweight** - Built with React and Vite
- 🔒 **Privacy-Focused** - API key stored locally, never sent anywhere except OpenAI
- ⚡ **Fast & Responsive** - Optimized for speed and efficiency

## Getting Started

1. Install dependencies: `npm install`
2. Start the development server: `npm run dev`
3. Open http://localhost:5173 in your browser
4. Press `Ctrl+,` (or `Cmd+,` on Mac) to open settings
5. Enter your OpenAI API key
6. Start chatting!

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Enter` | Send message |
| `Shift+Enter` | New line in message |
| `Ctrl+L` | Clear chat |
| `Ctrl+K` | Focus input |
| `Ctrl+,` | Open settings |
| `?` | Show keyboard shortcuts help |
| `Esc` | Close modals / Focus input |
| `Tab` | Navigate between elements |
| `↑` / `↓` | Scroll chat (when chat is focused) |

## Tech Stack

- React 19
- Vite
- Tailwind CSS
- OpenAI API

## Design Philosophy

Pro-Chat is designed for developers and power users who prefer keyboard-driven workflows. Every feature is accessible without touching the mouse, and the UI stays out of your way while you focus on the conversation.

## Browser Support

Works in all modern browsers that support ES6+ JavaScript and CSS Grid/Flexbox.

## License

MIT License - see LICENSE file for details