# Markers
Each file contains a definition of a marker group. A file named `_.lua` refers to the folder, and its settings extend to all nested files and folders.

Each marker group can contain:

* `enabled` - set to false to disable all markers in this group.
* `name` - the name to display in the Fallback settings window. If omitted, the file/folder name is used.
* `when` - a function that can return `false` to indicate that all markers in this group should not be displayed.
* `markers` - a list of marker definitions.
* `overrides` - a list of setting overrides for agent markers.

## Markers
Each marker definition displays something. there are 3 types:

* Agent - a marker that follows the position of an agent.
* Circle - a static marker in the shape of a circle.
* Line - a static marker in the shape of a line or polygon.

Each marker contains these fields:

* `type` - can be `agent`, `circle`, or `line`.
* `name` - the name to display in the Fallback settings window.
* `when` - a function that can return `false` to indicate that this marker should not be displayed.

### Agent Markers
* `channel` - a unique identifier for this specific marker, so that its setting can be changed by [Overrides](#overrides), including overrides that are in more deeply nested groups than the marker itself.
* `properties.shape` - can be `Shape.Circle`, `Shape.Square`, `Shape.Teardrop`, or `Shape.Triangle`.
* `properties.size` - how big the marker should be.
* `properties.border_color` - the color of the shape's border.
* `properties.border_opacity` - how transparent the border is.
* `properties.border_thickness` - how thick the border is.
* `properties.fill_color` - the color of the shape's interior.
* `properties.fill_opacity` - how transparent the interior is.
* `properties.draw_priority` - a number that determines if this marker should be drawn on top of others, a marker with higher `draw_priority` is drawn on top. Can be negative, and if omitted it defaults to 0.

### Circle Markers
* `x` and `y` - the center of the circle.
* `radius` - the radius of the circle.
* `draw_on_terrain` - if true, this marker is also drawn in-game as well as on the compass.

### Line Markers
* `points` - a list of coordinates. If the first and last entry are the same, a closed polygon is formed and the `fill_color` and `fill_opacity` settings will apply.
* `draw_on_terrain` - if true, this marker is also drawn in-game as well as on the compass.

## Overrides
Rather than drawing a new marker, an override modifies the settings of existing markers. This applies only to Agent markers. Only settings that are specified are overridden, allowing multiple overrides to stack on top of each other.

* `channel` - the unique identifier of the marker to override.
* `name` - the name to display in the Fallback settings window.
* `priority` - a number that determines if this override should take precedence over other overrides (in case multiple overrides affect the same setting), an override with a higher `priority` will take preference. Can be negative, and if omitted it defaults to 0.
* `properties` - the same as for Agent markers, contains the settings to override. This can also be a function that takes an `agent_id` as argument and returns the properties object.
* `when` - a function that can return `false` to indicate that this override should not be applied.
