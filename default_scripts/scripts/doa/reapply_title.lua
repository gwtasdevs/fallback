---@type Script
script = {
  name = "Re-apply title",
  triggers = {
    { type = "skill", skill_id = Skill.ShadowForm, finished = true },
  },
  run = function()
    gw.player.reapply_title()
  end,
}
