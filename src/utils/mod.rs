use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use anyhow::{Result, anyhow};
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

pub mod parallel;

#[derive(Debug)]
pub struct MultiError {
  pub errors: Vec<anyhow::Error>,
}

impl MultiError {
  pub fn new(errors: Vec<anyhow::Error>) -> Self {
    let errors = errors
      .into_iter()
      .flat_map(|err| match err.downcast::<MultiError>() {
        Ok(multi_err) => multi_err.errors,
        Err(err) => vec![err],
      })
      .collect::<Vec<_>>();

    Self { errors }
  }
}

impl std::error::Error for MultiError {}

impl std::fmt::Display for MultiError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "Multiple errors occurred (count: {}):", self.errors.len())?;
    for (i, err) in self.errors.iter().enumerate() {
      // Indent the sub-errors for readability
      writeln!(f, "  [{}] {}", i + 1, err)?;
    }
    Ok(())
  }
}

pub static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
  if let Some(config_path) = dirs::document_dir().map(|path| path.join("GWFallback")) {
    if std::fs::create_dir_all(&config_path).is_ok() {
      config_path
    } else {
      panic!("Could not create path to config path {}.", config_path.display());
    }
  } else {
    panic!("Could not find documents dir.");
  }
});

pub static IS_VSCODE_INSTALLED: LazyLock<bool> = LazyLock::new(|| {
  let result = Command::new("cmd")
    .args(["/C", "code --version"])
    .creation_flags(CREATE_NO_WINDOW.0)
    .output()
    .is_ok();

  log::debug!("VsCode installation status initialized");
  result
});

/// Write to a temporary file first, then renames, to prevent file corruption if the host process closes mid-write.
/// This will overwrite the file if it already exists.
pub fn write_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
  let temp_filename = format!(
    "{}.{}.tmp",
    path
      .as_ref()
      .file_name()
      .ok_or(anyhow!("Cannot write to file with no name."))?
      .to_string_lossy(),
    &uuid::Uuid::new_v4().to_string()[0..8],
  );

  let temp_path = path.as_ref().with_file_name(temp_filename);
  std::fs::write(&temp_path, contents)?;
  std::fs::rename(&temp_path, path)?;
  Ok(())
}

pub fn open_in_file_explorer(path: &Path) -> bool {
  Command::new("explorer").arg(path).spawn().is_ok()
}

pub fn open_in_vscode(root_path: &Path, path: &Path) -> bool {
  if *IS_VSCODE_INSTALLED {
    Command::new("cmd")
      .args(["/C", "code", &root_path.to_string_lossy(), &path.to_string_lossy()])
      .creation_flags(CREATE_NO_WINDOW.0)
      .spawn()
      .is_ok()
  } else {
    false
  }
}
