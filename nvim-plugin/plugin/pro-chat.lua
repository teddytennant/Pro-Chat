--- Auto-loaded entry point for the pro-chat Neovim plugin.
--- Registers user commands so they are available immediately on startup.
--- The heavy lifting lives in lua/pro-chat/init.lua.

if vim.g.loaded_pro_chat then
  return
end
vim.g.loaded_pro_chat = true

local pro_chat = require("pro-chat")

-- :ProChat  -- open pro-chat in a terminal split
vim.api.nvim_create_user_command("ProChat", function()
  pro_chat.open()
end, { desc = "Open pro-chat in a terminal split" })

-- :ProChatToggle  -- toggle the pro-chat split
vim.api.nvim_create_user_command("ProChatToggle", function()
  pro_chat.toggle()
end, { desc = "Toggle the pro-chat terminal split" })

-- :ProChatSend  -- send visual selection to pro-chat
vim.api.nvim_create_user_command("ProChatSend", function()
  pro_chat.send_selection()
end, { range = true, desc = "Send visual selection to pro-chat" })

-- :ProChatAsk <question>  -- ask a question about the current buffer
vim.api.nvim_create_user_command("ProChatAsk", function(cmd)
  pro_chat.ask(cmd.args)
end, { nargs = "+", desc = "Ask pro-chat a question about the current buffer" })

-- :ProChatFile  -- send the entire current file as context
vim.api.nvim_create_user_command("ProChatFile", function()
  pro_chat.send_file()
end, { desc = "Send the current file to pro-chat as context" })
