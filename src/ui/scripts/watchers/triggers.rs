use std::sync::Arc;

use arc_swap::ArcSwap;
use hudhook::imgui;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, error::TryRecvError};

use crate::{
  gw::{
    Gw,
    models::{map::InstanceType, skill::SkillId},
  },
  ui::{
    common::{KeyMode, KeyState},
    scripts::{
      ScriptTask,
      models::{ScriptGroup, Trigger},
      watchers::ui::UiMessage,
    },
  },
};

async fn handle_hook<F: Fn(&Trigger) -> bool>(
  group: &ScriptGroup,
  ui_messages_sender: &UnboundedSender<Vec<UiMessage>>,
  set: &mut tokio::task::JoinSet<()>,
  trigger_matches: &F,
) -> Vec<ScriptTask> {
  let mut new_tasks = vec![];

  if !group.enabled {
    return new_tasks;
  }

  if let Some(when) = group.when.as_ref() {
    match when.call_async::<bool>(()).await {
      Ok(result) => {
        if !result {
          return new_tasks;
        }
      }
      Err(err) => {
        log::error!(
          "Error executing the `when` function of script group '{:?}': {}.",
          group.path,
          err
        );
        return new_tasks;
      }
    }
  }

  let now = std::time::Instant::now();

  for script in group.child_scripts.iter() {
    if !script.enabled {
      continue;
    }

    for script_trigger in script.triggers.iter() {
      if trigger_matches(script_trigger) {
        if let Some(throttle) = script.throttle
          && let Some(last_trigger) = script.last_triggered
        {
          let throttle = std::time::Duration::from_millis(throttle.into());
          if let Some(expiry) = last_trigger.checked_add(throttle)
            && now < expiry
          {
            continue;
          }
        }

        if let Some(when) = script.when.as_ref() {
          match when.call_async::<bool>(()).await {
            Ok(result) => {
              if !result {
                continue;
              }
            }
            Err(err) => {
              log::error!(
                "Error executing the `when` function of script '{:?}': {}.",
                script.path,
                err
              );
              continue;
            }
          }
        }

        if let Some(task) = script.run(ui_messages_sender, set) {
          new_tasks.push(task);
        }

        // break here to stop the script executing multiple times if more than 1 trigger matches
        // TODO: add a bool option to allow a script to have multiple running instances?
        break;
      }
    }
  }

  for child_group in group.child_groups.iter() {
    new_tasks.extend(Box::pin(handle_hook(child_group, ui_messages_sender, set, trigger_matches)).await);
  }

  new_tasks
}

fn collect_key_state(key_state_receiver: &mut UnboundedReceiver<KeyState>) -> Result<KeyState, TryRecvError> {
  match key_state_receiver.try_recv() {
    Ok(key_state) => {
      let mut keys_down = key_state.keys_down;
      let mut keys_pressed = key_state.keys_pressed;

      loop {
        match key_state_receiver.try_recv() {
          Ok(key_state) => {
            keys_down = key_state.keys_down;
            keys_pressed.extend(key_state.keys_pressed);
          }
          Err(TryRecvError::Empty) => break,
          Err(TryRecvError::Disconnected) => {
            log::warn!("Key press hook disconnected.");
            return Err(TryRecvError::Disconnected);
          }
        }
      }

      Ok(KeyState {
        keys_down,
        keys_pressed,
      })
    }
    Err(TryRecvError::Empty) => Err(TryRecvError::Empty),
    Err(TryRecvError::Disconnected) => Err(TryRecvError::Disconnected),
  }
}

pub async fn watch_script_triggers(
  gw_api: Arc<Gw>,
  mut key_state_receiver: UnboundedReceiver<KeyState>,
  root_script: Arc<ArcSwap<ScriptGroup>>,
  tasks: Arc<ArcSwap<Vec<ScriptTask>>>,
  ui_messages_sender: UnboundedSender<Vec<UiMessage>>,
  _lua: Arc<mlua::Lua>,
) {
  log::debug!("Watching for script triggers.");

  let mut previous_key_state = KeyState::empty();

  let on_tick = Arc::new(tokio::sync::Notify::new());
  let notify = Arc::clone(&on_tick);
  let on_tick_hook_id = gw_api.thread.register_on_tick_hook(move || notify.notify_one());

  let (tx, mut on_map_changed) = tokio::sync::watch::channel(None);
  let gw_api_ref = gw_api.clone();
  let on_map_changed_ui_hook_id = gw_api.map.register_map_loaded_hook(move |_| {
    let _ = tx.send(gw_api_ref.map.get_map_id());
  });
  log::debug!("Hooked GW map_loaded_ui_message.");

  let (tx, mut on_skill_activated) = tokio::sync::watch::channel(SkillId::None);
  let gw_api_ref = gw_api.clone();
  let on_skill_activated_hook_id = gw_api.agent.register_skill_activated_hook(move |message| {
    let message = unsafe { message.read_unaligned() };
    let player_id = gw_api_ref.agent.get_player_agent_id();
    if message.agent_id as usize == player_id {
      let _ = tx.send(message.skill_id);
    }
  });
  log::debug!("Hooked GW on_skill_activated.");

  let (tx, mut on_skill_finished) = tokio::sync::watch::channel(SkillId::None);
  let gw_api_ref = gw_api.clone();
  let on_skill_finished_hook_id = gw_api.agent.register_skill_finished_hook(move |message| {
    let message = unsafe { message.read_unaligned() };
    let player_id = gw_api_ref.agent.get_player_agent_id();
    if message.agent_id as usize == player_id {
      let _ = tx.send(message.skill_id);
    }
  });
  log::debug!("Hooked GW on_skill_finished.");

  scopeguard::defer! {
    gw_api.thread.deregister_on_tick_hook(&on_tick_hook_id);
    gw_api.ui.deregister_send_ui_message_hook(&on_map_changed_ui_hook_id);
    gw_api.ui.deregister_send_ui_message_hook(&on_skill_activated_hook_id);
    gw_api.ui.deregister_send_ui_message_hook(&on_skill_finished_hook_id);
  }

  let mut set = tokio::task::JoinSet::new();

  loop {
    // wait for game loop tick
    on_tick.notified().await;

    let mut new_tasks: Vec<ScriptTask> = vec![];

    let script = root_script.load();

    // TODO: prevent await of when funcs in Tick and Key(mode: Hold) triggers.

    // run scripts that trigger on tick
    new_tasks.extend(
      handle_hook(&script, &ui_messages_sender, &mut set, &|trigger| {
        *trigger == Trigger::Tick
      })
      .await,
    );

    // run scripts that trigger on key press
    match collect_key_state(&mut key_state_receiver) {
      Ok(key_state) => {
        new_tasks.extend(
          handle_hook(&script, &ui_messages_sender, &mut set, &|trigger| match trigger {
            Trigger::Key {
              keys,
              mode,
              alt,
              ctrl,
              shift,
              win,
            } => {
              let alt_pressed = key_state.keys_down.contains(&imgui::Key::LeftAlt)
                || key_state.keys_down.contains(&imgui::Key::RightAlt);
              let ctrl_pressed = key_state.keys_down.contains(&imgui::Key::LeftCtrl)
                || key_state.keys_down.contains(&imgui::Key::RightCtrl);
              let shift_pressed = key_state.keys_down.contains(&imgui::Key::LeftShift)
                || key_state.keys_down.contains(&imgui::Key::RightShift);
              let win_pressed = key_state.keys_down.contains(&imgui::Key::LeftSuper)
                || key_state.keys_down.contains(&imgui::Key::RightSuper);

              if *alt != alt_pressed || *ctrl != ctrl_pressed || *shift != shift_pressed || *win != win_pressed {
                return false;
              }

              if !keys.iter().all(|&key| key_state.keys_down.contains(&key.into())) {
                return false;
              }

              match mode {
                KeyMode::Hold => true,
                KeyMode::Press => keys.iter().any(|&key| key_state.keys_pressed.contains(&key.into())),
              }
            }
            _ => false,
          })
          .await,
        );
        previous_key_state = key_state;
      }
      Err(TryRecvError::Empty) => {
        // if keys are still held, process hold key triggers
        new_tasks.extend(
          handle_hook(&script, &ui_messages_sender, &mut set, &|trigger| match trigger {
            Trigger::Key {
              keys,
              mode,
              alt,
              ctrl,
              shift,
              win,
            } => match mode {
              KeyMode::Hold => {
                let alt_pressed = previous_key_state.keys_down.contains(&imgui::Key::LeftAlt)
                  || previous_key_state.keys_down.contains(&imgui::Key::RightAlt);
                let ctrl_pressed = previous_key_state.keys_down.contains(&imgui::Key::LeftCtrl)
                  || previous_key_state.keys_down.contains(&imgui::Key::RightCtrl);
                let shift_pressed = previous_key_state.keys_down.contains(&imgui::Key::LeftShift)
                  || previous_key_state.keys_down.contains(&imgui::Key::RightShift);
                let win_pressed = previous_key_state.keys_down.contains(&imgui::Key::LeftSuper)
                  || previous_key_state.keys_down.contains(&imgui::Key::RightSuper);

                if *alt != alt_pressed || *ctrl != ctrl_pressed || *shift != shift_pressed || *win != win_pressed {
                  return false;
                }

                keys
                  .iter()
                  .all(|&key| previous_key_state.keys_down.contains(&key.into()))
              }
              KeyMode::Press => false,
            },
            _ => false,
          })
          .await,
        );
      }
      Err(TryRecvError::Disconnected) => {
        log::error!("Key press hook disconnected.");
        break;
      }
    }

    match on_map_changed.has_changed() {
      Ok(true) => {
        // TODO: reset script throttling

        let current_map_id = *on_map_changed.borrow_and_update();
        if let Some(current_map_id) = current_map_id {
          let mut instance_type;
          loop {
            instance_type = gw_api.map.get_instance_type();
            if instance_type != InstanceType::Loading {
              break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
          }
          tokio::time::sleep(std::time::Duration::from_millis(500)).await;
          log::debug!("Map changed: {:?} : {:?}", current_map_id, instance_type);
          new_tasks.extend(
            handle_hook(&script, &ui_messages_sender, &mut set, &|trigger| match trigger {
              Trigger::Map { map_id, outpost } => {
                if let Some(map_id) = map_id
                  && *map_id != current_map_id
                {
                  return false;
                }

                match outpost {
                  Some(true) if instance_type == InstanceType::Outpost => true,
                  Some(false) if instance_type == InstanceType::Explorable => true,
                  _ => false,
                }
              }
              _ => false,
            })
            .await,
          );
        }
      }
      Ok(false) => {}
      Err(_) => {
        log::warn!("GW map changed hook disconnected.");
        break;
      }
    }

    match on_skill_activated.has_changed() {
      Ok(true) => {
        let actual_skill_id = *on_skill_activated.borrow_and_update();
        new_tasks.extend(
          handle_hook(&script, &ui_messages_sender, &mut set, &|trigger| match trigger {
            Trigger::Skill {
              skill_id,
              finished: false,
            } => {
              if let Some(skill_id) = skill_id
                && *skill_id != actual_skill_id
              {
                return false;
              }

              true
            }
            _ => false,
          })
          .await,
        );
      }
      Ok(false) => {}
      Err(_) => {
        log::warn!("GW skill activated hook disconnected.");
        break;
      }
    }

    match on_skill_finished.has_changed() {
      Ok(true) => {
        let actual_skill_id = *on_skill_finished.borrow_and_update();
        new_tasks.extend(
          handle_hook(&script, &ui_messages_sender, &mut set, &|trigger| match trigger {
            Trigger::Skill {
              skill_id,
              finished: true,
            } => {
              if let Some(skill_id) = skill_id
                && *skill_id != actual_skill_id
              {
                return false;
              }

              true
            }
            _ => false,
          })
          .await,
        );
      }
      Ok(false) => {}
      Err(_) => {
        log::warn!("GW skill finished hook disconnected.");
        break;
      }
    }

    if !new_tasks.is_empty() {
      tasks.rcu(|tasks| {
        let mut updated_tasks = tasks
          .iter()
          .filter(|task| !task.abort_handle.is_finished())
          .cloned()
          .collect::<Vec<_>>();
        updated_tasks.extend(new_tasks.iter().cloned());

        Arc::new(updated_tasks)
      });
    }
  }

  set.shutdown().await;
  log::debug!("Stopped watching for script triggers.");
}
