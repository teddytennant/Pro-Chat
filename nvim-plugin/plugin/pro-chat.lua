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

-- :ProChatReview  -- review the current file's git diff
vim.api.nvim_create_user_command("ProChatReview", function()
  pro_chat.review()
end, { desc = "Review the current file's git changes in pro-chat" })

-- :ProChatExplain  -- explain visual selection or whole buffer
vim.api.nvim_create_user_command("ProChatExplain", function(cmd)
  pro_chat.explain(cmd.range > 0)
end, { range = true, desc = "Explain code in pro-chat" })

-- :ProChatRefactor  -- refactor visual selection
vim.api.nvim_create_user_command("ProChatRefactor", function(cmd)
  pro_chat.refactor(cmd.range > 0)
end, { range = true, desc = "Refactor code via pro-chat" })

-- :ProChatTest  -- generate tests for visual selection or whole buffer
vim.api.nvim_create_user_command("ProChatTest", function(cmd)
  pro_chat.test(cmd.range > 0)
end, { range = true, desc = "Generate tests via pro-chat" })
