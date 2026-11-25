---@type Script
script = {
  name = "Open Xunlai Chest",
  triggers = {
    { type = "key", keys = {Key.F8} },
    { type = "map", outpost = true },
  },
  run = function()
    gw.interact.open_xunlai_chest()
  end,
}
