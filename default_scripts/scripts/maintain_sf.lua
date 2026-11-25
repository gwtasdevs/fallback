---@type Script
script = {
  enabled = false,
  name = "Maintain Shadow Form",
  throttle = 5000,
  triggers = {
    { type = "tick" },
  },
  run = function()
    gw.player.use_skill(Skill.ShadowForm)
  end,
  when = function()
    return gw.map.get_instance_type() == InstanceType.Explorable
      and gw.player.get_skill_cooldown(Skill.ShadowForm) == 0
      and gw.player.get_effect_remaining_duration(Skill.ShadowForm) < 900
  end
}
