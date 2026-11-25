---@type Script
script = {
  name = "Enter r2",
  triggers = {
    { type = "key", keys = { Key.Delete } },
  },
  run = function()
    if gw.map.get_instance_type() ~= InstanceType.Explorable
      or gw.player.distance_from(-18321, 13638) > 3000
    then
      return;
    end

    gw.player.move_to(-17059, 15102)
    gw.util.wait(100)
    gw.player.move_to(-18631, 13192)
    gw.util.wait(40)
    gw.player.move_to(-19019, 13188)
    gw.player.move_to(-19491, 12803)
    gw.player.start_moving(-19426, 11939)
  end,
}
