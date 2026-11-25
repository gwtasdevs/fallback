---@type Script
script = {
  enabled = false,
  name = "Auto Rock Candy",
  throttle = 5000,
  triggers = {
    { type = "tick" },
  },
  run = function()
    if not gw.player.has_effect(2971) then
      gw.util.log("Popping blue rock candy")
      gw.player.use_item(31151)
    end
    if not gw.player.has_effect(2972) then
      gw.util.log("Popping green rock candy")
      gw.player.use_item(31152)
    end
    if not gw.player.has_effect(2973) then
      gw.util.log("Popping red rock candy")
      gw.player.use_item(31153)
    end
  end,
  when = function()
    return gw.map.get_instance_type() == InstanceType.Explorable
      and not gw.player.has_skill(Skill.IAmUnstoppable)
      and gw.player.get_agent().is_crippled
      and (
        not gw.player.has_effect(2971)
        or not gw.player.has_effect(2972)
        or not gw.player.has_effect(2973)
      )
  end
}
