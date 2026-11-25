---@type MarkerGroup
marker_group = {
  markers = {
    {
      type = "agent",
      channel = "agent",
      name = "Agent",
      properties = {
        shape = Shape.Teardrop,
        size = 50,
        border_opacity = 0.25,
      },
      when = function(agent_id)
        return gw.agent.get_agent(agent_id) --[[@as boolean]]
      end,
    },
    {
      type = "agent",
      channel = "gadget",
      name = "Gadget",
      properties = {
        shape = Shape.Square,
        size = 40,
        border_opacity = 0,
        fill_color = 0x0000C8,
      },
      when = function(agent_id)
        return gw.agent.get_gadget(agent_id) --[[@as boolean]]
      end,
    },
  },
  overrides = {
    {
      channel = "agent",
      name = "Target",
      properties = {
        border_opacity = 1,
        border_thickness = 1.5,
      },
      when = function(agent_id)
        return gw.agent.get_target_agent_id() == agent_id
      end
    },
    {
      channel = "agent",
      name = "Player",
      priority = 100,
      properties = {
        fill_color = 0xFF8000,
        draw_priority = 100,
      },
      when = function(agent_id)
        return gw.agent.get_player_agent_id() == agent_id
      end,
    },
    {
      channel = "agent",
      name = "Ally",
      properties = {
        fill_color = 0x99FF99,
      },
      when = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        return agent and agent.is_ally --[[@as boolean]]
      end,
    },
    {
      channel = "agent",
      name = "Minion",
      properties = {
        size = 30,
      },
      when = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        return agent and agent.is_allied_minion --[[@as boolean]]
      end,
    },
    {
      channel = "agent",
      name = "Enemy",
      properties = {
        fill_color = 0xF00000,
      },
      when = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        return agent and agent.is_enemy and not agent.is_dead --[[@as boolean]]
      end,
    },
    {
      channel = "agent",
      name = "Dead enemy",
      properties = {
        fill_color = 0x320000,
      },
      when = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        return agent and agent.is_enemy and agent.is_dead --[[@as boolean]]
      end,
    },
  },
}
