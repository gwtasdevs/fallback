---@type Script
script = {
  name = "Enter r2",
  triggers = {
    { type = "key", keys = { Key.Delete } },
  },
  run = function()
    if gw.map.get_instance_type() ~= InstanceType.Explorable
      or gw.player.distance_from(-13856, 10872) > 1500
    then
      return;
    end

    gw.player.move_to(-13886, 11340)
    gw.util.wait(50)
    gw.player.move_to(-14240, 10934)
    gw.player.move_to(-14277, 10945)
    gw.player.move_to(-14703, 11062)
    gw.player.start_moving(-14974, 11298)
  end,
}
