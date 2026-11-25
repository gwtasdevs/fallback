---@type Script
script = {
  name = "Dhuum glitch",
  triggers = {
    { type = "key", keys = {Key.E} },
  },
  run = function()
    gw.player.move_to(0, 0)
  end,
  when = function()
    return gw.player.distance_from(0, 0) < 1000
  end,
}
