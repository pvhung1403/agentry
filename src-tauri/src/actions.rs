use crate::model::{Item, Removal};
use crate::scanner::{is_symlink, join_rel};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// Changes things on disk. Every change is reversible: files go to the Trash, config edits are backed up.
/// Errors that the UI translates start with a code: `tool-not-found:<tool>`, `not-json-object`, `manual`.
pub struct Actions {
    pub home: PathBuf,
    pub backup_root: PathBuf,
}

impl Actions {
    pub fn new(home: PathBuf, backup_root: PathBuf) -> Self {
        Self { home, backup_root }
    }

    pub fn remove(&self, item: &Item) -> Result<(), String> {
        match &item.removal {
            Removal::Trash { path } => move_to_trash(path),
            Removal::Cli { argv, cwd } => self.run(argv, cwd.as_deref()),
            Removal::JsonKey { file, key_path } => self.remove_json_key(file, key_path),
            Removal::Manual { .. } => Err("manual".into()),
        }
    }

    pub fn set_enabled(&self, item: &Item, enabled: bool) -> Result<(), String> {
        let Some(toggle) = &item.toggle else { return Ok(()) };
        self.run(if enabled { &toggle.enable } else { &toggle.disable }, None)
    }

    // MARK: JSON config edits

    pub fn remove_json_key(&self, file: &Path, key_path: &[String]) -> Result<(), String> {
        let text = fs::read_to_string(file).map_err(|e| e.to_string())?;
        let mut root: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        if !root.is_object() {
            return Err("not-json-object".into());
        }
        self.backup(file)?;
        remove_path(&mut root, key_path);
        let mut output = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
        output.push('\n');
        let temp = file.with_extension("agentry-tmp");
        fs::write(&temp, output).map_err(|e| e.to_string())?;
        fs::rename(&temp, file).map_err(|e| e.to_string())
    }

    pub fn backup(&self, file: &Path) -> Result<PathBuf, String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let folder = self.backup_root.join(format!("{}-{:09}", now.as_secs(), now.subsec_nanos()));
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let relative = file.strip_prefix(&self.home).unwrap_or(file);
        let flat: Vec<String> = relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
        let copy = folder.join(flat.join("__"));
        fs::copy(file, &copy).map_err(|e| e.to_string())?;
        Ok(copy)
    }

    // MARK: running agent CLIs

    /// Apps opened from the Dock, Start menu or app launcher don't get the shell's PATH,
    /// so also look in the usual install folders.
    fn search_dirs(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = [".local/bin", ".bun/bin", ".npm-global/bin", ".volta/bin", ".cargo/bin"]
            .iter()
            .map(|r| join_rel(&self.home, r))
            .collect();
        if cfg!(windows) {
            if let Some(appdata) = std::env::var_os("APPDATA") {
                dirs.push(PathBuf::from(appdata).join("npm"));
            }
        } else {
            dirs.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin", "/snap/bin"].map(PathBuf::from));
        }
        if let Some(path) = std::env::var_os("PATH") {
            dirs.extend(std::env::split_paths(&path));
        }
        dirs
    }

    pub fn resolve(&self, tool: &str) -> Option<PathBuf> {
        let suffixes: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ".bat"] } else { &[""] };
        self.search_dirs()
            .into_iter()
            .flat_map(|dir| suffixes.iter().map(move |s| dir.join(format!("{tool}{s}"))))
            .find(|p| is_executable(p))
    }

    pub fn run(&self, argv: &[String], cwd: Option<&Path>) -> Result<(), String> {
        let Some((tool, args)) = argv.split_first() else { return Ok(()) };
        let executable = self.resolve(tool).ok_or_else(|| format!("tool-not-found:{tool}"))?;
        let mut command = Command::new(executable);
        command.args(args).stdin(Stdio::null());
        if let Ok(path) = std::env::join_paths(self.search_dirs()) {
            command.env("PATH", path);
        }
        if let Some(dir) = cwd {
            command.current_dir(dir);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flash
        }
        let output = command.output().map_err(|e| e.to_string())?;
        if output.status.success() {
            return Ok(());
        }
        let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        let text = text.trim();
        Err(if text.is_empty() { format!("`{}` failed", argv.join(" ")) } else { text.to_string() })
    }
}

/// Removes the key at `path`, keeping the order of the other keys.
pub fn remove_path(value: &mut Value, path: &[String]) {
    let Some((last, parents)) = path.split_last() else { return };
    let mut node = value;
    for key in parents {
        node = match node.get_mut(key.as_str()) {
            Some(child) => child,
            None => return,
        };
    }
    if let Some(map) = node.as_object_mut() {
        map.shift_remove(last);
    }
}

fn move_to_trash(path: &Path) -> Result<(), String> {
    // A symlink is removed by itself: some trash backends resolve links and would trash the shared target.
    if is_symlink(path) {
        return fs::remove_file(path).or_else(|_| fs::remove_dir(path)).map_err(|e| e.to_string());
    }
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        let mut context = trash::TrashContext::default();
        // NSFileManager avoids the Finder automation permission prompt.
        context.set_delete_method(DeleteMethod::NsFileManager);
        context.delete(path).map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        trash::delete(path).map_err(|e| e.to_string())
    }
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else { return false };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.is_file() && meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        meta.is_file()
    }
}
