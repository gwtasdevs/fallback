---@type MarkerGroup
marker_group = {
  overrides = {
    {
      channel = "agent",
      name = "Soul Tormentor",
      priority = 1,
      properties = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        if not agent then
          return {}
        end

        return {
          draw_priority = agent.weapon_type == WeaponType.None and 1 or 0,
          shape = agent.weapon_type == WeaponType.None and Shape.Teardrop or Shape.Square,
          fill_color = agent.weapon_type == WeaponType.None and agent.is_in_combat and 0xFFFF3B or 0xFFDC00,
        }
      end,
      when = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        return agent and agent.name == "Soul Tormentor" --[[@as boolean]]
      end,
    },
    {
      channel = "agent",
      name = "Water Tormentor",
      properties = {
        fill_color = 0x0,
      },
      when = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        return agent and agent.name == "Water Tormentor" --[[@as boolean]]
      end,
    },
    {
      channel = "agent",
      name = "Stygian Fiend",
      properties = {
        fill_color = 0x0,
      },
      when = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        return agent and agent.name == "Stygian Fiend" --[[@as boolean]]
      end,
    },
  },
}
