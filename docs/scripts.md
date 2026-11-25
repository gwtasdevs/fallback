# Scripts
Each folder represents a group, and each individual file is a script. A file called `_.lua` can be created to change settings for the group:

* `enabled` - set to false to disable all scripts nested inside this folder.
* `name` - the name to display in the Fallback settings window. If omitted, the folder name is used.
* `when` - a function that can return `false` to indicate that all scripts nested inside should not be run.

A script should at a minimum contains a `run` function, which contains the code to execute when triggered. It may also contain:

* `enabled` - set to false to disable the script.
* `name` - the name to display in the Fallback settings window. If omitted, the file name is used.
* `throttle` - throttle the script to once per the specified number of milliseconds.
* `triggers` - if no trigger is specified, then the script can only be run by clicking on it in the Fallback settings window.
* `when` - if specified, the script will only run when this function returns `true`.

## Throttle
A script can set `throttle` to ensure it is not run more than once in the specified number of milliseconds. If the `run` function returns `false`, this indicates that it was not successful, and throttling will be ignored.

## Trigger
Scripts can be triggered by a few different mechanisms:

* Hotkey pressed
* Map changed
* Skill used
* Every tick. This is useful for scripts that need to run continuously.
