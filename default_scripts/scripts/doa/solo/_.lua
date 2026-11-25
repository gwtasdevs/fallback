---@type ScriptGroup
group = {
  when = function()
    return gw.player.get_party_size() == 1
      and gw.player.get_primary_profession() == Profession.Ranger
      and gw.player.get_secondary_profession() == Profession.Assassin
  end
}
