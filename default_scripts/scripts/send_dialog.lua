---@type Script
script = {
  enabled = false,
  name = "Send Dialog",
  triggers = {
    { type = "key", keys = {Key.E} }
  },
  run = function()
    gw.interact.send_first_dialog()
  end,
}
