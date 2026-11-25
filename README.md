# Fallback!
A tool for Guild Wars speedclear runners.

## Project Aims
* Flexibility. Although this won't always be possible, ideally Fallback should not break when Guild Wars gets an update.
* Stability. No crashing, no thrashing, no overwriting user settings on a whim or causing disconnects. Everything should work smoothly.
* Focussed. Fallback is made for speedclears & TAS records. Other features e.g. for vanquishing etc. are outside of the scope of this project.

## Installation
1. Download `fallback.dll` from the latest release on the [Releases](https://github.com/gwtasdevs/fallback/releases) page.
2. Inject this into the game e.g. via [GWLauncher](https://github.com/gwdevhub/gwlauncher), [Daybreak](https://github.com/gwdevhub/Daybreak), etc.

## User Settings
Settings are stored in `C:\Users\<your name>\Documents\GWFallback\`, which contains:

* `main.yml` for the basic settings. This should not need to edited manually.
* The folder `logs/` for debug logs.
* The lua script folders `markers/`, and `scripts/`.

### Lua Scripts
Most settings are written via lua scripts. To edit these, it is recommended to use [VSCode](https://code.visualstudio.com/) with the [Lua](https://marketplace.visualstudio.com/items?itemName=sumneko.lua) extension, although a plain text editor will work in a pinch.
Each Fallback module has its own settings folder. If none of these folders exist when Fallback starts, it will create some default ones which can be good to read as examples. The file `_meta.lua` is generated each time Fallback runs, and includes documentation for all the different settings which should also show up by hovering over things or triggering intellisense in vscode.

All lua scripts have access to an api under the global variable `gw` which can be used to interact with the game. All functions are documented in the generated `_meta.lua`, and can be seen via intellisense in vscode.

#### Markers
All settings related to custom lines, colors, and agents markers that display on the compass. See the [markers](docs/markers.md) documentation for more details.

#### Scripts
Contains custom scripts to do any sort of automated task, including hotkeys. See the [scripts](docs/scripts.md) documentation for more details.
