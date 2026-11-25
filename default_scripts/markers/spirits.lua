local spirit_colors = {
  [2926] = 0x00FFFF, -- Winnowing
  [2927] = 0x00FF00, -- Edge of Extinction
  [2930] = 0xFF00FF, -- Symbiosis
  [2933] = 0xFEFFFF, -- Frozen Soil
  [2937] = 0x0000FF, -- Quickening Zephyr
}

---@type MarkerGroup
marker_group = {
  markers = {
    {
      type = "agent",
      channel = "spirit_range",
      name = "Spirit range",
      properties = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        if not agent then
          return {}
        end

        return {
          shape = Shape.Circle,
          size = 3000,
          fill_opacity = 0.2,
          border_opacity = 0,
          fill_color = spirit_colors[agent.model_id]
        }
      end,
      when = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        if not agent or agent.is_dead then
          return false
        end
        return spirit_colors[agent.model_id] --[[@as boolean]]
      end,
    },
  },
  overrides = {
    {
      channel = "agent",
      name = "Spirit agent",
      properties = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        if not agent then
          return {}
        end
        if agent.is_dead then
          return {
            fill_opacity = 0,
            border_opacity = 0,
          }
        end

        return {
          fill_color = spirit_colors[agent.model_id],
          draw_priority = -1,
        }
      end,
      when = function(agent_id)
        local agent = gw.agent.get_agent(agent_id)
        return agent and spirit_colors[agent.model_id] --[[@as boolean]]
      end,
    },
  },
}
