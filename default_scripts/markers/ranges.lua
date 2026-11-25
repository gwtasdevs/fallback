local border_opacity = 0.3

---@type MarkerGroup
marker_group = {
  markers = {
    {
      type = "agent",
      name = "HoS range",
      properties = {
        shape = Shape.Circle,
        size = 360,
        fill_opacity = 0,
        border_opacity = border_opacity,
      },
      when = function(agent_id)
        return gw.agent.get_player_agent_id() == agent_id
      end,
    },
    {
      type = "agent",
      name = "Chain aggro range",
      properties = {
        shape = Shape.Circle,
        size = 700,
        fill_opacity = 0,
        border_opacity = border_opacity,
      },
      when = function(agent_id)
        return gw.agent.get_player_agent_id() == agent_id or gw.agent.get_target_agent_id() == agent_id
      end,
    },
    {
      type = "agent",
      name = "Aggro range",
      properties = {
        shape = Shape.Circle,
        size = 1012,
        fill_opacity = 0,
        border_opacity = border_opacity,
      },
      when = function(agent_id)
        return gw.agent.get_player_agent_id() == agent_id
      end,
    },
    {
      type = "agent",
      name = "Spell range",
      properties = {
        shape = Shape.Circle,
        size = 1248,
        fill_opacity = 0,
        border_opacity = border_opacity,
      },
      when = function(agent_id)
        return gw.agent.get_player_agent_id() == agent_id
      end,
    },
    {
      type = "agent",
      name = "Spirit range",
      properties = {
        shape = Shape.Circle,
        size = 3000,
        fill_opacity = 0,
        border_opacity = border_opacity,
      },
      when = function(agent_id)
        return gw.agent.get_player_agent_id() == agent_id
      end,
    },
    {
      type = "agent",
      name = "Compass range",
      properties = {
        shape = Shape.Circle,
        size = 5020,
        fill_opacity = 0,
        border_opacity = border_opacity,
      },
      when = function(agent_id)
        return gw.agent.get_player_agent_id() == agent_id
      end,
    },
  },
}
