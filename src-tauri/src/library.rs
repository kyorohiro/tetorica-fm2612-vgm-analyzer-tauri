use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

const RECENT_LIMIT: usize = 20;
const FILE_LIMIT: usize = 10000;
const BYTE_LIMIT: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    id: String,
    kind: String,
    label: String,
    paths: Vec<PathBuf>,
    pinned: bool,
}
#[derive(Default)]
pub struct Library {
    entries: Vec<Entry>,
    files: HashMap<String, PathBuf>,
    next: u64,
    storage: PathBuf,
}
pub type LibraryState = Mutex<Library>;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    token: String,
    name: String,
    size: u64,
}
#[derive(Serialize)]
pub struct Selection {
    tracks: Vec<Track>,
    entries: Vec<Entry>,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub(crate) fn allowed(window: &WebviewWindow) -> Result<(), String> {
    let url = window.url().map_err(err)?;
    let local = url.scheme() == "tauri"
        || (matches!(url.scheme(), "http" | "https") && url.host_str() == Some("tauri.localhost"));
    let development = cfg!(debug_assertions)
        && window
            .app_handle()
            .config()
            .build
            .dev_url
            .as_ref()
            .is_some_and(|dev| dev.origin() == url.origin());
    if window.label() == "main" && (local || development) {
        Ok(())
    } else {
        Err("Desktop library is available only in the local Analyzer window".into())
    }
}
fn trim(entries: &mut Vec<Entry>) {
    let mut count = 0;
    entries.retain(|entry| {
        if entry.pinned {
            return true;
        }
        count += 1;
        count <= RECENT_LIMIT
    });
}
fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "vgm" | "vgz" | "s98"))
}
fn collect(paths: &[PathBuf], folder: bool) -> Result<Vec<PathBuf>, String> {
    let mut result = Vec::new();
    if folder {
        let root = paths.first().ok_or("Missing folder path")?;
        if !root.is_dir() {
            return Err(format!("Folder is unavailable: {}", root.display()));
        }
        let mut pending = vec![root.clone()];
        let mut visited = 0;
        while let Some(dir) = pending.pop() {
            visited += 1;
            if visited > FILE_LIMIT {
                return Err("Too many subfolders. Choose a smaller folder.".into());
            }
            for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
                let entry = entry.map_err(err)?;
                let kind = entry.file_type().map_err(err)?;
                // Do not follow directory symlinks; avoid cycles and leaving the chosen tree.
                if kind.is_dir() {
                    pending.push(entry.path());
                } else if kind.is_file() && supported(&entry.path()) {
                    result.push(entry.path());
                }
                if result.len() > FILE_LIMIT {
                    return Err("Too many tracks. Choose a smaller folder.".into());
                }
            }
        }
    } else {
        for path in paths {
            if !path.is_file() {
                return Err(format!("File is unavailable: {}", path.display()));
            }
            if supported(path) {
                result.push(path.clone());
            }
        }
    }
    result.sort();
    result.dedup();
    if result.is_empty() {
        return Err("No VGM, VGZ or S98 files found.".into());
    }
    if result.len() > FILE_LIMIT {
        return Err("Too many tracks. Choose fewer files.".into());
    }
    Ok(result)
}
impl Library {
    pub fn load(storage: PathBuf) -> Result<Self, String> {
        let mut entries: Vec<Entry> = match fs::read(&storage) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| format!("Cannot read recent library: {e}"))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(err(e)),
        };
        entries.retain(|e| !e.paths.is_empty() && matches!(e.kind.as_str(), "files" | "folder"));
        trim(&mut entries);
        Ok(Self {
            entries,
            storage,
            ..Self::default()
        })
    }
    fn save(&mut self, entries: Vec<Entry>) -> Result<(), String> {
        let parent = self.storage.parent().ok_or("Missing settings directory")?;
        fs::create_dir_all(parent).map_err(err)?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(err)?;
        tmp.write_all(&serde_json::to_vec_pretty(&entries).map_err(err)?)
            .map_err(err)?;
        tmp.as_file().sync_all().map_err(err)?;
        tmp.persist(&self.storage).map_err(err)?;
        self.entries = entries;
        Ok(())
    }
    fn select(&mut self, paths: Vec<PathBuf>, folder: bool) -> Result<Selection, String> {
        let files = collect(&paths, folder)?;
        let mut tracks = Vec::new();
        let mut grants = HashMap::new();
        for path in files {
            let size = fs::metadata(&path).map_err(err)?.len();
            if size > BYTE_LIMIT {
                return Err(format!("File exceeds 256 MiB: {}", path.display()));
            }
            // Check readability before remembering the selection; actual bytes are loaded on demand.
            fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            self.next += 1;
            let token = self.next.to_string();
            let name = if folder {
                path.strip_prefix(&paths[0])
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned()
            } else {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            };
            tracks.push(Track {
                token: token.clone(),
                name,
                size,
            });
            grants.insert(token, path);
        }
        let kind = if folder { "folder" } else { "files" };
        let existing = self
            .entries
            .iter()
            .find(|e| e.kind == kind && e.paths == paths);
        let entry = Entry {
            id: existing.map(|e| e.id.clone()).unwrap_or_else(|| {
                format!(
                    "{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos()
                )
            }),
            pinned: existing.is_some_and(|e| e.pinned),
            label: if paths.len() == 1 {
                paths[0]
                    .file_name()
                    .unwrap_or(paths[0].as_os_str())
                    .to_string_lossy()
                    .into_owned()
            } else {
                format!(
                    "{} (+{} files)",
                    paths[0].file_name().unwrap_or_default().to_string_lossy(),
                    paths.len() - 1
                )
            },
            kind: kind.into(),
            paths,
        };
        let mut entries = self.entries.clone();
        entries.retain(|e| e.id != entry.id);
        entries.insert(0, entry);
        trim(&mut entries);
        self.save(entries)?;
        self.files = grants;
        Ok(Selection {
            tracks,
            entries: self.entries.clone(),
        })
    }
}

#[tauri::command]
pub fn library_list(
    window: WebviewWindow,
    state: State<'_, LibraryState>,
) -> Result<Vec<Entry>, String> {
    allowed(&window)?;
    Ok(state.lock().map_err(err)?.entries.clone())
}
#[tauri::command]
pub async fn library_choose(
    window: WebviewWindow,
    app: tauri::AppHandle,
    folder: bool,
) -> Result<Option<Selection>, String> {
    allowed(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let picker = app.dialog().file().set_parent(&window);
        let chosen = if folder {
            picker.blocking_pick_folder().map(|p| vec![p])
        } else {
            picker
                .add_filter("VGM / VGZ / S98", &["vgm", "vgz", "s98"])
                .blocking_pick_files()
        };
        let Some(chosen) = chosen else {
            return Ok(None);
        };
        let mut paths = chosen
            .into_iter()
            .map(|p| {
                p.into_path()
                    .map_err(err)
                    .and_then(|p| fs::canonicalize(p).map_err(err))
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        paths.dedup();
        app.state::<LibraryState>()
            .lock()
            .map_err(err)?
            .select(paths, folder)
            .map(Some)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
pub async fn library_open(
    window: WebviewWindow,
    app: tauri::AppHandle,
    id: String,
) -> Result<Selection, String> {
    allowed(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<LibraryState>();
        let mut library = state.lock().map_err(err)?;
        let entry = library
            .entries
            .iter()
            .find(|e| e.id == id)
            .cloned()
            .ok_or("Recent item not found")?;
        library.select(entry.paths, entry.kind == "folder")
    })
    .await
    .map_err(err)?
}
#[tauri::command]
pub fn library_pin(
    window: WebviewWindow,
    state: State<'_, LibraryState>,
    id: String,
    pinned: bool,
) -> Result<Vec<Entry>, String> {
    allowed(&window)?;
    let mut library = state.lock().map_err(err)?;
    let mut entries = library.entries.clone();
    entries
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or("Recent item not found")?
        .pinned = pinned;
    trim(&mut entries);
    library.save(entries)?;
    Ok(library.entries.clone())
}
#[tauri::command]
pub fn library_remove(
    window: WebviewWindow,
    state: State<'_, LibraryState>,
    id: String,
) -> Result<Vec<Entry>, String> {
    allowed(&window)?;
    let mut library = state.lock().map_err(err)?;
    let entries = library
        .entries
        .iter()
        .filter(|e| e.id != id)
        .cloned()
        .collect();
    library.save(entries)?;
    Ok(library.entries.clone())
}
#[tauri::command]
pub async fn library_read(
    window: WebviewWindow,
    app: tauri::AppHandle,
    token: String,
) -> Result<tauri::ipc::Response, String> {
    allowed(&window)?;
    let path = app
        .state::<LibraryState>()
        .lock()
        .map_err(err)?
        .files
        .get(&token)
        .cloned()
        .ok_or("File selection expired; reopen it from the menu")?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut bytes = Vec::new();
        fs::File::open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .take(BYTE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        if bytes.len() as u64 > BYTE_LIMIT {
            return Err("File exceeds 256 MiB".into());
        }
        Ok(tauri::ipc::Response::new(bytes))
    })
    .await
    .map_err(err)?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(i: usize, pinned: bool) -> Entry {
        Entry {
            id: i.to_string(),
            kind: "files".into(),
            label: i.to_string(),
            paths: vec![PathBuf::from(format!("{i}.vgm"))],
            pinned,
        }
    }
    #[test]
    fn all_pins_survive_trimming_and_restart() {
        let dir = tempfile::tempdir().unwrap();
        let storage = dir.path().join("recent.json");
        let mut library = Library::load(storage.clone()).unwrap();
        let mut entries: Vec<_> = (0..60).map(|i| entry(i, i % 2 == 0)).collect();
        trim(&mut entries);
        assert_eq!(entries.iter().filter(|e| e.pinned).count(), 30);
        assert_eq!(entries.iter().filter(|e| !e.pinned).count(), 20);
        library.save(entries.clone()).unwrap();
        assert_eq!(Library::load(storage).unwrap().entries, entries);
    }
    #[test]
    fn reopening_preserves_pin_and_folder_is_rescanned() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        fs::create_dir(&music).unwrap();
        fs::write(music.join("01.vgm"), b"Vgm ").unwrap();
        let mut library = Library::load(dir.path().join("recent.json")).unwrap();
        let first = library.select(vec![music.clone()], true).unwrap();
        library.entries[0].pinned = true;
        fs::write(music.join("02.VGZ"), b"data").unwrap();
        fs::write(music.join("ignore.txt"), b"data").unwrap();
        let second = library.select(vec![music.clone()], true).unwrap();
        assert_eq!(second.tracks.len(), 2);
        assert_eq!(second.entries.len(), 1);
        assert_eq!(first.entries[0].id, second.entries[0].id);
        assert!(second.entries[0].pinned);
        fs::remove_dir_all(music).unwrap();
        assert!(library
            .select(second.entries[0].paths.clone(), true)
            .is_err());
        assert_eq!(library.entries, second.entries);
    }
    #[test]
    fn corrupt_history_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recent.json");
        fs::write(&path, b"broken").unwrap();
        assert!(Library::load(path.clone()).is_err());
        assert_eq!(fs::read(path).unwrap(), b"broken");
    }
}
