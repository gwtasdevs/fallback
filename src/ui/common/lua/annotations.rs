use anyhow::Result;
use indoc::indoc;

use crate::utils::{CONFIG_PATH, write_file};

pub fn write_annotations() -> Result<()> {
  // TODO: maybe the annotations file should be created at build time, and only copied to the scripts path here.
  let annotations_path = CONFIG_PATH.join("_meta.lua");
  write_file(&annotations_path, get_annotations()?)
}

fn get_annotations() -> Result<String> {
  let mut annotations = String::from(
    indoc!("
      ---@meta

      ---@class Script
      ---@field enabled boolean? -- Default: true.
      ---@field name string? -- Name of the script. Default: the filename.
      ---@field throttle number? -- The number of milliseconds after the script was last run before it can run again. Use 0 to mean only run once per instance. Omit to not throttle at all.
      ---@field triggers Trigger[]? -- A list of triggers for this script.
      ---@field run fun(): boolean? -- The function to be run. Return false to indicate that the script was not successful, and should not count for throttling purposes.
      ---@field when (fun(): boolean)? -- A function that determines when this script should be run.
      script = nil

      ---@alias Trigger KeyTrigger|MapTrigger|SkillTrigger|TickTrigger

      --- Triggers when a key is pressed.
      ---@class KeyTrigger
      ---@field type \"key\"
      ---@field keys Key[]
      ---@field mode KeyMode? -- Press: the script will run when the key is pressed. Hold: the script will run every frame whilst the key is held. Default: Press.
      ---@field alt boolean? -- Default: false.
      ---@field ctrl boolean? -- Default: false.
      ---@field shift boolean? -- Default: false.
      ---@field win boolean? -- Default: false.

      --- Triggers when a map is loaded.
      ---@class MapTrigger
      ---@field type \"map\"
      ---@field map_id number? -- Default: any map.
      ---@field outpost boolean? -- If true, then trigger when entering an outpost. If false, then trigger when entering an explorable. Omit to trigger on both.

      --- Triggers when a skill is cast.
      ---@class SkillTrigger
      ---@field type \"skill\"
      ---@field skill_id number? -- Default: any skill.
      ---@field finished boolean? -- If true then trigger after the skill has finished casting. If false then trigger as soon as the skill has started casting. Default: true.

      --- Triggers on every frame.
      ---@class TickTrigger
      ---@field type \"tick\"

      ---@class ScriptGroup
      ---@field enabled boolean? -- Default: true.
      ---@field name string? -- Name of the group. Default: the folder name.
      ---@field when (fun(): boolean)? -- A function that determines when any scripts within the group should be allowed to run.
      group = nil

      ---@class MarkerGroup
      ---@field enabled boolean? -- Default: true.
      ---@field name string? -- Default: filename of the group.
      ---@field when (fun(): boolean)? -- A function that determines when any markers within the group should be rendered.
      ---@field markers Marker[]?
      ---@field overrides AgentMarkerOverride[]?
      marker_group = nil

      ---@class AgentMarkerProperties
      ---@field shape Shape? -- Default: Shape.Teardrop.
      ---@field size number? -- Default: 50.
      ---@field border_color number? -- Default: 0x000000.
      ---@field border_opacity number? -- Default: 1.
      ---@field border_thickness number? -- Default: 1.
      ---@field fill_color number? -- Default: 0x000000.
      ---@field fill_opacity number? -- Default: 1.
      ---@field draw_priority integer? -- Markers with higher draw priority are drawn on top of markers with lower draw priority. Default: 0.

      --- Overrides properties for the specified channel. When an agent marker is drawn with that channel, it will inherit properties from all overrides that match.
      ---@class AgentMarkerOverride
      ---@field channel string -- Identifies which marker to apply the override to.
      ---@field name string
      ---@field priority integer? -- When two overrides affect the same property, the one with the higher priority take precedence. Default: 0.
      ---@field properties AgentMarkerProperties|(fun(agent_id: integer): AgentMarkerProperties)?
      ---@field when (fun(agent_id: integer): boolean)?

      ---@alias Marker AgentMarker|CircleMarker|LineMarker

      --- Displayed per agent, at the agent's location.
      ---@class AgentMarker
      ---@field type \"agent\"
      ---@field name string
      ---@field channel string? -- An identifier that overrides can target. If omitted, then overrides cannot apply to this marker.
      ---@field properties AgentMarkerProperties|(fun(agent_id: integer): AgentMarkerProperties)?
      ---@field when (fun(agent_id: integer): boolean)?

      ---@class CircleMarker
      ---@field type \"circle\"
      ---@field name string
      ---@field x number
      ---@field y number
      ---@field radius number
      ---@field border_color number? -- Default: 0xFFFFFF.
      ---@field border_opacity number? -- Default: 1.
      ---@field border_thickness number? -- Default: 1.
      ---@field fill_color number? -- Default: 0xFFFFFF.
      ---@field fill_opacity number? -- Default: 1.
      ---@field draw_on_terrain boolean? -- Default: false.
      ---@field draw_priority integer? -- Markers with higher draw priority are drawn on top of markers with lower draw priority. Default: 0.
      ---@field when (fun(): boolean)?

      ---@class LineMarker
      ---@field type \"line\"
      ---@field name string
      ---@field points { x: number, y: number }[]
      ---@field border_color number? -- Default: 0xFFFFFF.
      ---@field border_opacity number? -- Default: 1.
      ---@field border_thickness number? -- Default: 1.
      ---@field fill_color number? -- Default: 0xFFFFFF.
      ---@field fill_opacity number? -- Default: 1.
      ---@field draw_on_terrain boolean? -- Default: false.
      ---@field draw_priority integer? -- Markers with higher draw priority are drawn on top of markers with lower draw priority. Default: 0.
      ---@field when (fun(): boolean)?

    ")
  );
  for annotation_getter in inventory::iter::<super::lua_macros::LuaAnnotationGetter> {
    annotations.push_str(&annotation_getter.0());
  }
  for annotation in inventory::iter::<super::lua_macros::LuaAnnotation> {
    annotations.push_str(annotation.0);
  }

  Ok(annotations)
}
