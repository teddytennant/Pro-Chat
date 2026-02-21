--- Pro-Chat Neovim integration plugin.
--- Opens and manages a pro-chat TUI terminal split inside Neovim.

local M = {}

--- Default configuration.
local defaults = {
  -- Width of the terminal split as a fraction of the editor width.
  split_width = 0.4,
  -- Explicit path to the pro binary. When nil the plugin auto-detects.
  binary = nil,
  -- Extra arguments forwarded to the pro binary.
  extra_args = {},
  -- Keybinding prefix.
  leader_key = "<leader>c",
  -- Whether to register default keymaps on setup.
  keymaps = true,
}

--- Resolved runtime state.
local state = {
  config = {},
  buf = nil,
  win = nil,
  chan = nil,
}

---------------------------------------------------------------------------
-- Helpers
---------------------------------------------------------------------------

--- Locate the pro binary on the system.
--- Checks, in order:
---   1. Explicit config value
---   2. Cargo target/release build inside the plugin's parent repo
---   3. Cargo target/debug build inside the plugin's parent repo
---   4. $HOME/.cargo/bin/pro
---   5. Whatever is on $PATH
---@return string|nil path Absolute path to the binary, or nil.
local function find_binary()
  -- 1. Explicit config.
  if state.config.binary then
    if vim.fn.executable(state.config.binary) == 1 then
      return state.config.binary
    end
    vim.notify("[pro-chat] configured binary not found: " .. state.config.binary, vim.log.levels.WARN)
  end

  -- 2/3. Repo-local Cargo builds (release first, then debug).
  local plugin_root = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":h:h:h")
  for _, profile in ipairs({ "release", "debug" }) do
    local candidate = plugin_root .. "/target/" .. profile .. "/pro"
    if vim.fn.executable(candidate) == 1 then
      return candidate
    end
  end

  -- 4. User cargo bin.
  local cargo_bin = vim.env.HOME .. "/.cargo/bin/pro"
  if vim.fn.executable(cargo_bin) == 1 then
    return cargo_bin
  end

  -- 5. $PATH.
  local on_path = vim.fn.exepath("pro")
  if on_path ~= "" then
    return on_path
  end

  return nil
end

--- Return the current Neovim server socket path (v:servername).
---@return string
local function server_name()
  return vim.v.servername or ""
end

--- Build the full command string used to launch pro-chat.
---@return string|nil cmd Shell command, or nil when the binary cannot be found.
local function build_cmd()
  local bin = find_binary()
  if not bin then
    vim.notify("[pro-chat] could not locate the 'pro' binary", vim.log.levels.ERROR)
    return nil
  end

  local parts = { vim.fn.shellescape(bin) }

  -- Pass the Neovim socket so pro-chat can communicate back.
  local sock = server_name()
  if sock ~= "" then
    table.insert(parts, "--nvim-socket")
    table.insert(parts, vim.fn.shellescape(sock))
  end

  for _, arg in ipairs(state.config.extra_args) do
    table.insert(parts, vim.fn.shellescape(arg))
  end

  return table.concat(parts, " ")
end

--- Return true when the pro-chat window is visible.
---@return boolean
local function is_open()
  return state.win ~= nil and vim.api.nvim_win_is_valid(state.win)
end

--- Return true when the terminal buffer is still alive.
---@return boolean
local function buf_valid()
  return state.buf ~= nil and vim.api.nvim_buf_is_valid(state.buf)
end

--- Configure the terminal buffer for a clean appearance.
---@param buf integer Buffer handle.
local function configure_term_buf(buf)
  local opts = {
    number = false,
    relativenumber = false,
    signcolumn = "no",
    foldcolumn = "0",
    statuscolumn = "",
    winfixwidth = true,
  }
  -- We apply these on the window after it is created, but also store them
  -- so they can be reapplied when the buffer is reshown.
  vim.api.nvim_buf_set_var(buf, "pro_chat_winopts", opts)
end

--- Apply stored window options.
---@param win integer Window handle.
---@param buf integer Buffer handle.
local function apply_win_opts(win, buf)
  local ok, opts = pcall(vim.api.nvim_buf_get_var, buf, "pro_chat_winopts")
  if not ok then
    return
  end
  for k, v in pairs(opts) do
    vim.api.nvim_set_option_value(k, v, { win = win })
  end
end

---------------------------------------------------------------------------
-- Core API
---------------------------------------------------------------------------

--- Open the pro-chat terminal in a vertical split on the right side.
function M.open()
  if is_open() then
    -- Already visible -- just focus it.
    vim.api.nvim_set_current_win(state.win)
    vim.cmd("startinsert")
    return
  end

  -- If the buffer is alive but the window was closed, re-show it.
  if buf_valid() then
    local width = math.floor(vim.o.columns * state.config.split_width)
    vim.cmd("botright vertical " .. width .. "split")
    state.win = vim.api.nvim_get_current_win()
    vim.api.nvim_win_set_buf(state.win, state.buf)
    apply_win_opts(state.win, state.buf)
    vim.cmd("startinsert")
    return
  end

  -- Launch a fresh terminal.
  local cmd = build_cmd()
  if not cmd then
    return
  end

  local width = math.floor(vim.o.columns * state.config.split_width)
  vim.cmd("botright vertical " .. width .. "split")
  state.win = vim.api.nvim_get_current_win()
  state.buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_win_set_buf(state.win, state.buf)

  state.chan = vim.fn.termopen(cmd, {
    on_exit = function(_, exit_code, _)
      -- Clean up state when the process exits.
      state.chan = nil
      if buf_valid() then
        vim.api.nvim_buf_delete(state.buf, { force = true })
      end
      state.buf = nil
      if is_open() then
        vim.api.nvim_win_close(state.win, true)
      end
      state.win = nil
      if exit_code ~= 0 then
        vim.notify("[pro-chat] process exited with code " .. exit_code, vim.log.levels.WARN)
      end
    end,
  })

  configure_term_buf(state.buf)
  apply_win_opts(state.win, state.buf)
  vim.cmd("startinsert")
end

--- Close the pro-chat window (the terminal keeps running in the background).
function M.close()
  if is_open() then
    vim.api.nvim_win_close(state.win, true)
    state.win = nil
  end
end

--- Toggle the pro-chat split open/closed.
function M.toggle()
  if is_open() then
    M.close()
  else
    M.open()
  end
end

--- Send raw text into the running pro-chat terminal.
---@param text string The text to feed into the terminal.
function M.send_raw(text)
  if not state.chan then
    vim.notify("[pro-chat] terminal is not running -- open it first with :ProChat", vim.log.levels.WARN)
    return
  end
  vim.api.nvim_chan_send(state.chan, text)
end

--- Send the current visual selection as context to pro-chat.
--- The text is sent verbatim followed by a newline.
function M.send_selection()
  -- Yank the visual selection into register z.
  local saved = vim.fn.getreg("z")
  vim.cmd('noautocmd normal! "zy')
  local text = vim.fn.getreg("z")
  vim.fn.setreg("z", saved)

  if text == "" then
    vim.notify("[pro-chat] empty selection", vim.log.levels.INFO)
    return
  end

  -- Make sure the terminal is open.
  if not state.chan then
    M.open()
  end

  M.send_raw(text .. "\n")
end

--- Send a question about the current buffer to pro-chat.
--- The question is prefixed with the buffer file name so the LLM has context.
---@param question string The question to ask.
function M.ask(question)
  if not question or question == "" then
    vim.notify("[pro-chat] no question provided", vim.log.levels.WARN)
    return
  end

  local bufname = vim.api.nvim_buf_get_name(0)
  if bufname == "" then
    bufname = "[unsaved buffer]"
  end

  local filetype = vim.bo.filetype or ""
  local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
  local content = table.concat(lines, "\n")

  local prompt = string.format(
    "File: %s (filetype: %s)\n\n```%s\n%s\n```\n\nQuestion: %s\n",
    bufname,
    filetype,
    filetype,
    content,
    question
  )

  if not state.chan then
    M.open()
  end

  M.send_raw(prompt)
end

--- Send the entire current file as context to pro-chat.
function M.send_file()
  local bufname = vim.api.nvim_buf_get_name(0)
  if bufname == "" then
    bufname = "[unsaved buffer]"
  end

  local filetype = vim.bo.filetype or ""
  local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
  local content = table.concat(lines, "\n")

  local prompt = string.format(
    "Here is the full file %s (filetype: %s):\n\n```%s\n%s\n```\n",
    bufname,
    filetype,
    filetype,
    content
  )

  if not state.chan then
    M.open()
  end

  M.send_raw(prompt)
end

---------------------------------------------------------------------------
-- Setup
---------------------------------------------------------------------------

--- Initialise the plugin.  Intended to be called from a lazy.nvim config:
---
---   require("pro-chat").setup({ split_width = 0.35 })
---
---@param opts table|nil User configuration (merged over defaults).
function M.setup(opts)
  state.config = vim.tbl_deep_extend("force", {}, defaults, opts or {})

  if state.config.keymaps then
    local leader = state.config.leader_key
    -- Use the last character of the leader sequence as the sub-key prefix.
    -- Default: <leader>cc -> toggle, <leader>cs -> send, <leader>ca -> ask.
    vim.keymap.set("n", leader .. "c", M.toggle, { desc = "Pro-Chat: toggle" })
    vim.keymap.set("v", leader .. "s", M.send_selection, { desc = "Pro-Chat: send selection" })
    vim.keymap.set("n", leader .. "a", function()
      vim.ui.input({ prompt = "Ask pro-chat: " }, function(input)
        if input then
          M.ask(input)
        end
      end)
    end, { desc = "Pro-Chat: ask about buffer" })
  end
end

return M
