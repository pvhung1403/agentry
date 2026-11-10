// No console window next to the app in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod claude;
mod frontmatter;
mod model;
mod scanner;
mod specs;
#[cfg(test)]
mod tests;
mod usage;

use actions::Actions;
use model::{Component, Item, Kind};
use scanner::Scanner;
use usage::Usage;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

/// The last scan. Commands take item ids, so the UI can only act on things the scanner found.
struct AppState {
    home: PathBuf,
    backups: PathBuf,
    items: Mutex<Vec<Item>>,
    usage: Mutex<Usage>,
}

impl AppState {
    fn actions(&self) -> Actions {
        Actions::new(self.home.clone(), self.backups.clone())
    }

    fn find(&self, ids: &[String]) -> Vec<Item> {
        self.items
            .lock()
            .map(|items| items.iter().filter(|i| ids.contains(&i.id)).cloned().collect())
            .unwrap_or_default()
    }

    fn path_of(&self, id: &str) -> Result<PathBuf, String> {
        self.find(&[id.to_string()]).into_iter().next().and_then(|i| i.path).ok_or_else(|| "Item not found".to_string())
    }
}

#[derive(Serialize)]
struct Failure {
    name: String,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanResult {
    items: Vec<Item>,
    /// Oldest Claude Code history entry: "last used" can't see further back.
    history_since: Option<String>,
}

/// Scans every agent, then marks Claude Code items with when they were last used.
fn scan_home(home: PathBuf) -> (Vec<Item>, Usage) {
    let scanner = Scanner::new(home);
    let mut items = scanner.scan_all();
    let usage = Usage::read(&scanner.rel(".claude/projects"));
    usage.apply(&mut items);
    (items, usage)
}

#[tauri::command]
async fn scan(state: State<'_, AppState>) -> Result<ScanResult, String> {
    let home = state.home.clone();
    let (items, usage) = tauri::async_runtime::spawn_blocking(move || scan_home(home))
        .await
        .map_err(|e| e.to_string())?;
    eprintln!("agentry: scanned {} items", items.len());
    *state.items.lock().map_err(|e| e.to_string())? = items.clone();
    let history_since = usage.since.clone();
    *state.usage.lock().map_err(|e| e.to_string())? = usage;
    Ok(ScanResult { items, history_since })
}

/// What a plugin is made of, most expensive first, with when each part was last used.
#[tauri::command]
fn components(id: String, state: State<'_, AppState>) -> Vec<Component> {
    let Some(item) = state.find(&[id]).into_iter().next() else { return vec![] };
    let (Kind::Plugin, Some(root)) = (item.kind, item.path.clone()) else { return vec![] };
    let parts = Scanner::new(state.home.clone()).plugin_components(&root);
    state.usage.lock().map(|usage| usage.components(&item, parts)).unwrap_or_default()
}

#[tauri::command]
async fn remove(ids: Vec<String>, state: State<'_, AppState>) -> Result<Vec<Failure>, String> {
    let targets = state.find(&ids);
    let actions = state.actions();
    tauri::async_runtime::spawn_blocking(move || {
        targets
            .iter()
            .filter_map(|item| actions.remove(item).err().map(|message| Failure { name: item.name.clone(), message }))
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_enabled(id: String, enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    let item = state.find(&[id]).into_iter().next().ok_or("Item not found")?;
    let actions = state.actions();
    tauri::async_runtime::spawn_blocking(move || actions.set_enabled(&item, enabled))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn reveal(id: String, state: State<'_, AppState>) -> Result<(), String> {
    tauri_plugin_opener::reveal_item_in_dir(state.path_of(&id)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_item(id: String, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = state.path_of(&id)?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

/// First part of the file behind an item: SKILL.md / MEMORY.md for folders, the file itself otherwise.
#[tauri::command]
fn preview(id: String, state: State<'_, AppState>) -> Option<String> {
    let item = state.find(&[id]).into_iter().next()?;
    let mut file = item.path?;
    if file.is_dir() {
        file = file.join(if item.kind == Kind::Memory { "MEMORY.md" } else { "SKILL.md" });
    }
    if !["md", "mdc", "toml", "txt"].contains(&scanner::extension(&file).as_str()) {
        return None;
    }
    Some(scanner::head(&file).chars().take(6000).collect())
}

fn main() {
    // `agentry --scan-json` prints the scan, for bug reports and scripting (macOS / Linux terminals).
    if std::env::args().any(|a| a == "--scan-json") {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
        let (items, usage) = scan_home(home);
        let result = ScanResult { items, history_since: usage.since };
        println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let home = app.path().home_dir()?;
            let backups = app.path().app_data_dir()?.join("Backups");
            app.manage(AppState { home, backups, items: Mutex::new(Vec::new()), usage: Mutex::new(Usage::default()) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![scan, remove, set_enabled, reveal, open_item, preview, components])
        .run(tauri::generate_context!())
        .expect("error while running Agentry");
}
