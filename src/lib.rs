use hudhook::hooks::dx9::ImguiDx9Hooks;
mod gw;
mod ui;
mod utils;

#[macro_use]
extern crate macro_rules_attribute;

hudhook::hudhook!(ImguiDx9Hooks, ui::RenderLoop::default());
