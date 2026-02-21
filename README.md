# Pro Chat

A fast terminal UI chat client for LLMs, written in Rust. Vim-style keybindings, streaming responses, tool execution, and Neovim integration.

## Features

- **Fast TUI** built with [Ratatui](https://github.com/ratatui/ratatui) and Crossterm
- **Vim-style keybindings** with Normal, Insert, Command, and Search modes
- **Streaming API responses** with real-time token display
- **Tool execution** -- the model can read, write, and edit files, search codebases, and run shell commands
- **Tool permission system** with auto-allow, ask-first, and deny policies per tool
- **Syntax-highlighted code blocks** via [syntect](https://github.com/trishume/syntect)
- **4 built-in color themes** -- Tokyo Night, Catppuccin, Gruvbox, Dracula
- **Conversation history** with persistence and browsable history overlay
- **Session resume** -- automatically restores the last conversation on startup
- **Neovim integration** with a bundled plugin for terminal splits, code review, and more
- **Markdown rendering** in the chat view
- **Code block extraction** -- list, yank, or send code blocks to Neovim
- **Project context loading** -- inject your project file tree into the system prompt
- **Git diff review** -- load `git diff` output directly into the chat
- **Multi-provider support** -- Anthropic and OpenAI
- **Model aliases** for quick switching (`sonnet`, `opus`, `haiku`, `gpt4`, `gpt4m`)
- **Export conversations** to markdown files
- **Mouse scroll support**
- **Response timing** -- shows how long each response took
- **Message timestamps** on every message
- **Tab completion** for slash commands and file paths
- **Clipboard integration** -- paste text or yank responses
- **Bell notification** when a response completes

## Install

### From source with Cargo

```bash
cargo install --path .
```

This installs the `pro` binary to `~/.cargo/bin/`.

### Development build

```bash
cargo build --release
# Binary is at target/release/pro
```

### Nix

A `shell.nix` is provided for Nix users:

```bash
nix-shell
cargo build --release
```

## Usage

```bash
pro                                     # Start interactive chat
pro -p "explain monads"                 # Send a prompt directly
pro -m gpt-4o --provider openai         # Use OpenAI
pro --nvim-socket /tmp/nvim.sock        # Connect to a Neovim instance
pro -c <conversation-id>                # Resume a specific conversation
pro --config-path                       # Print the config file path
```

## Configuration

Config file location: `~/.config/pro-chat/config.toml`

API keys can also be set via environment variables: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`.

### Full config example

```toml
# API provider: "anthropic" or "openai"
provider = "anthropic"

# Model identifier (or use aliases via /model command)
model = "claude-sonnet-4-20250514"

# API keys (or set ANTHROPIC_API_KEY / OPENAI_API_KEY env vars)
# anthropic_api_key = "sk-ant-..."
# openai_api_key = "sk-..."

# Maximum tokens for the response
max_tokens = 8192

# Sampling temperature (0.0 - 1.0)
temperature = 0.7

# System prompt
system_prompt = "You are a helpful AI assistant."

# Start in vim mode (Normal mode). When false, starts in Insert mode.
vim_mode = false

# Ring the terminal bell when a response completes
notify_on_complete = true

# Color theme: "tokyo-night", "catppuccin", "gruvbox", "dracula"
theme_name = "tokyo-night"

# Custom theme colors (overrides the named theme)
[theme]
accent = "#7aa2f7"
user_color = "#9ece6a"
assistant_color = "#bb9af7"
border_color = "#3b4261"
dim_color = "#565f89"

# Neovim integration
[neovim]
auto_connect = true
socket_path = "/tmp/nvim.sock"
send_code_blocks = true
```

## Keybindings

Pro Chat uses vim-style modal editing. Press `Esc` to enter Normal mode, `i` to return to Insert mode.

### Global

| Key | Action |
|-----|--------|
| `Ctrl+c` | Cancel streaming response, or quit |
| `Ctrl+q` | Quit |

### Normal Mode

| Key | Action |
|-----|--------|
| `i` | Enter Insert mode at cursor |
| `a` | Enter Insert mode after cursor |
| `A` | Enter Insert mode at end of line |
| `I` | Enter Insert mode at start of line |
| `o` | Enter Insert mode on a new line below |
| `:` | Enter Command mode |
| `/` | Enter Search mode |
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `Ctrl+d` | Scroll down half page |
| `Ctrl+u` | Scroll up half page |
| `G` | Scroll to bottom |
| `g` | Scroll to top |
| `h` / `Left` | Cursor left |
| `l` / `Right` | Cursor right |
| `w` | Cursor forward one word |
| `b` | Cursor back one word |
| `0` | Cursor to start of line |
| `$` | Cursor to end of line |
| `x` | Delete character at cursor |
| `d` | Clear input line |
| `p` | Paste from clipboard |
| `y` | Copy last assistant response to clipboard |
| `Ctrl+y` | List code blocks (enter visual selection mode) |
| `1`-`9` | Yank code block by number (in visual mode) |
| `Ctrl+e` | Send last code block to Neovim |
| `Ctrl+r` | Retry / regenerate last response |
| `e` | Edit last user message (when input is empty) |
| `n` | Next search match |
| `N` | Previous search match |
| `?` | Open help overlay |
| `Ctrl+h` | Open history overlay |
| `Ctrl+n` | New conversation |
| `Ctrl+l` | Clear conversation |

### Insert Mode

| Key | Action |
|-----|--------|
| `Esc` | Switch to Normal mode |
| `Enter` | Send message |
| `Shift+Enter` / `Alt+Enter` | Insert newline |
| `Backspace` / `Ctrl+h` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `Ctrl+w` | Delete word before cursor |
| `Ctrl+u` | Delete to start of line |
| `Left` | Cursor left |
| `Right` | Cursor right |
| `Home` / `Ctrl+a` | Cursor to start of line |
| `End` / `Ctrl+e` | Cursor to end of line |
| `Up` / `Ctrl+p` | Previous input history |
| `Down` / `Ctrl+n` | Next input history |
| `Tab` | Tab-complete slash commands and file paths |

### Command Mode (`:`)

| Key | Action |
|-----|--------|
| `Esc` | Cancel and return to Normal mode |
| `Enter` | Execute command |
| `Backspace` | Delete character (exits to Normal if empty) |

Available commands: `:q`, `:quit`, `:w`, `:save`, `:wq`, `:clear`, `:new`, `:help`, `:history`, `:tools`, `:set model=<m>`, `:set temp=<t>`, `:set provider=<p>`, `:set vim`, `:set tools`, `:model <m>`

### Search Mode (`/`)

| Key | Action |
|-----|--------|
| `Esc` | Cancel search |
| `Enter` | Execute search |
| `Backspace` | Delete character (exits if empty) |

After searching, use `n` / `N` in Normal mode to navigate matches.

### Overlay Navigation (Help, History, Settings)

| Key | Action |
|-----|--------|
| `Esc` / `q` | Close overlay |
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `Enter` | Select item |
| `d` | Delete entry (History overlay only) |

## Slash Commands

Type these in Insert mode and press Enter.

| Command | Alias | Description |
|---------|-------|-------------|
| `/clear` | `/c` | Clear the current conversation |
| `/new` | `/n` | Start a new conversation (saves current) |
| `/model <name>` | `/m` | Set model (supports aliases: `sonnet`, `s`, `opus`, `o`, `haiku`, `h`, `gpt4`, `gpt4m`) |
| `/models` | | List available model aliases |
| `/provider <name>` | `/p` | Set API provider (`anthropic`, `openai`) |
| `/system <prompt>` | `/s` | Set or view the system prompt |
| `/temp <value>` | `/t` | Set or view the temperature |
| `/history` | `/h` | Browse conversation history |
| `/help` | `/?` | Show help overlay |
| `/tools [on\|off]` | | Toggle tools or show tool permissions |
| `/file <path>` | `/f` | Load a file's contents into the input |
| `/context` | `/ctx` | Load project file tree into system prompt |
| `/paste` | | Paste clipboard contents as a code block |
| `/resume` | `/r` | Resume the last conversation |
| `/diff` | `/d` | Load `git diff` output into the input for review |
| `/export [path]` | | Export conversation to a markdown file |
| `/theme <name>` | | Set theme (`tokyo-night`, `catppuccin`, `gruvbox`, `dracula`) |
| `/retry` | | Remove last assistant response for regeneration |
| `/edit` | | Edit the last user message |
| `/run <cmd>` | `/!` | Run a shell command and show output |
| `/nvim [socket]` | | Connect to a Neovim instance |
| `/save` | | Save current config to disk |
| `/quit` | `/q` | Quit |

## Neovim Integration

Pro Chat ships with a Neovim plugin in the `nvim-plugin/` directory. It opens Pro Chat in a vertical terminal split and provides commands for sending code, reviewing diffs, and more.

### Setup with lazy.nvim

```lua
{
  dir = "/path/to/Pro-Chat/nvim-plugin",
  config = function()
    require("pro-chat").setup({
      -- Width of the terminal split (fraction of editor width)
      split_width = 0.4,
      -- Explicit path to the pro binary (nil = auto-detect)
      binary = nil,
      -- Extra CLI arguments forwarded to pro
      extra_args = {},
      -- Keybinding prefix
      leader_key = "<leader>c",
      -- Whether to register default keymaps
      keymaps = true,
    })
  end,
}
```

### Commands

| Command | Description |
|---------|-------------|
| `:ProChat` | Open Pro Chat in a terminal split |
| `:ProChatToggle` | Toggle the Pro Chat split open/closed |
| `:ProChatSend` | Send visual selection to Pro Chat |
| `:ProChatAsk <question>` | Ask a question about the current buffer |
| `:ProChatFile` | Send the entire current file as context |
| `:ProChatReview` | Review the current file's unstaged git diff |
| `:ProChatExplain` | Explain visual selection or whole buffer |
| `:ProChatRefactor` | Refactor visual selection |
| `:ProChatTest` | Generate tests for visual selection or whole buffer |

### Default Keymaps

When `keymaps = true` (the default), the following mappings are registered using the configured `leader_key` (default `<leader>c`):

| Mapping | Mode | Action |
|---------|------|--------|
| `<leader>cc` | Normal | Toggle Pro Chat split |
| `<leader>cs` | Visual | Send selection |
| `<leader>ca` | Normal | Ask about current buffer (prompts for input) |
| `<leader>cr` | Normal | Review current file's git changes |
| `<leader>ce` | Visual | Explain selection |
| `<leader>cf` | Visual | Refactor selection |
| `<leader>ct` | Visual | Generate tests for selection |

## Tools

When connected to the Anthropic API, Pro Chat gives the model access to these tools:

| Tool | Default Permission | Description |
|------|-------------------|-------------|
| `read_file` | Auto-allow | Read file contents |
| `write_file` | Ask first | Write content to a file |
| `edit_file` | Ask first | Replace text in a file |
| `list_files` | Auto-allow | List files in a directory |
| `search_files` | Auto-allow | Search for patterns in files |
| `execute` | Ask first | Run a shell command |

When a tool requires confirmation, a prompt appears with these options:

| Key | Action |
|-----|--------|
| `y` / `Enter` | Allow this invocation |
| `a` | Always allow this tool type |
| `n` / `Esc` | Deny this invocation |
| `d` | Always deny this tool type |

## Themes

Set the theme with `/theme <name>` or in `config.toml` with `theme_name`.

| Theme | Description |
|-------|-------------|
| `tokyo-night` | Cool blues and purples (default) |
| `catppuccin` | Soft pastels on dark background |
| `gruvbox` | Warm retro colors |
| `dracula` | High-contrast purple and cyan |

## License

MIT
