---@type Script
script = {
  name = "Start gloom",
  triggers = {
    { type = "map", map_id = Map.DomainOfAnguish, outpost = false },
  },
  run = function()
    gw.player.start_moving(16024, 2077)
    gw.player.use_item(22269) -- Cupcake

    local npc = gw.agent.wait_until_found(function(agent) return agent.name == "Voice of Whispers" end)
    if npc == nil then
      return
    end

    gw.interact.change_target(npc.agent_id)

    gw.player.move_to(16024, 2077)
    gw.interact.interact_with_npc(npc.agent_id)
    gw.interact.send_dialog(8581377)
    gw.player.start_moving(14061, 4912)

    gw.util.wait_until(function()
      local souls = gw.agent.count(function(agent) return agent.name == "Soul Tormentor" end)
      return souls >= 2
    end)

    local weaponless_souls = gw.agent.count(function(agent)
      return agent.name == "Soul Tormentor" and agent.weapon_type == WeaponType.None
    end)

    if weaponless_souls == 0 then
      gw.map.travel(Map.DomainOfAnguish)
      return
    end

    gw.util.log(weaponless_souls + " weaponless souls detected")

    -- fb.consumables.enable()
  end,
  when = function()
    -- only if starting in gloom
    return gw.player.distance_from(16034, 1244) <= 500
  end,
}
