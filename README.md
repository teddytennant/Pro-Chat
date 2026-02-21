# Pro Chat

Fast TUI chat CLI with vim keybindings, streaming AI responses, and Neovim integration.

## Install

```bash
cargo install --path .
```

## Usage

```bash
pro                          # Start interactive chat
pro -p "explain monads"      # Send a prompt directly
pro -m gpt-4o --provider openai  # Use OpenAI
pro --nvim-socket /tmp/nvim  # Connect to Neovim
```

## Configuration

Config file: `~/.config/pro-chat/config.toml`

```toml
provider = "anthropic"
model = "claude-sonnet-4-20250514"
anthropic_api_key = "sk-ant-..."
temperature = 0.7
vim_mode = false

[theme]
accent = "#7aa2f7"

[neovim]
auto_connect = true
```

Or set env vars: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`

## Keybindings

Vim-style modes: Normal (`Esc`), Insert (`i`), Command (`:`)

| Key | Mode | Action |
|-----|------|--------|
| `i/a/A/I/o` | Normal | Enter insert mode |
| `j/k` | Normal | Scroll messages |
| `Ctrl+d/u` | Normal | Half-page scroll |
| `G/gg` | Normal | Bottom/top |
| `y` | Normal | Copy last response |
| `?` | Normal | Help |
| `Enter` | Insert | Send message |
| `Shift+Enter` | Insert | New line |
| `Ctrl+c` | Any | Cancel stream / quit |

## Slash Commands

`/clear` `/new` `/model <m>` `/provider <p>` `/system <prompt>` `/temp <t>` `/history` `/nvim` `/save` `/quit`

## License

MIT
