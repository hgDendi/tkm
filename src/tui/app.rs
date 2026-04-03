use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use secrecy::{ExposeSecret, SecretString};

use crate::core::registry::Registry;
use crate::core::token::{BackendType, TokenMeta};
use crate::storage::encrypted_file::EncryptedFileBackend;
use crate::storage::keychain::KeychainBackend;
use crate::storage::StorageBackend;
use crate::tui::events;

/// Which screen is active
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Unlock,
    List,
    Detail(usize),
    Add,
    Confirm(ConfirmAction),
    Search,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    Delete(usize),
}

/// Clipboard status message with auto-clear
pub struct ClipStatus {
    pub message: String,
    pub expires: Instant,
}

/// Main TUI application state
pub struct App {
    pub screen: Screen,
    pub should_quit: bool,

    // Data
    pub registry: Registry,
    pub entries: Vec<TokenMeta>,

    // Backends
    pub keychain: KeychainBackend,
    pub file_backend: EncryptedFileBackend,
    pub file_unlocked: bool,

    // List state
    pub table_state: TableState,

    // Unlock screen
    pub password_input: String,
    pub unlock_error: Option<String>,

    // Add screen
    pub add_service: String,
    pub add_key: String,
    pub add_value: String,
    pub add_backend: BackendType,
    pub add_label: String,
    pub add_field_idx: usize, // which field is focused (0=service,1=key,2=value,3=backend,4=label)

    // Detail screen
    pub reveal_secret: bool,
    pub secret_cache: Option<String>, // temporarily cached secret for display

    // Search
    pub search_query: String,
    pub filtered_indices: Vec<usize>,

    // Status
    pub clip_status: Option<ClipStatus>,
    pub status_message: Option<String>,
}

fn tkm_dir() -> std::path::PathBuf {
    dirs::home_dir().expect("home dir").join(".tkm")
}

fn registry_path() -> std::path::PathBuf {
    tkm_dir().join("registry.toml")
}

impl App {
    pub fn new() -> Result<Self> {
        let registry = Registry::load(&registry_path())?;
        let entries: Vec<TokenMeta> = registry.list().to_vec();
        let file_backend = EncryptedFileBackend::new(&tkm_dir());
        let needs_unlock = file_backend.vault_exists();

        let mut table_state = TableState::default();
        if !entries.is_empty() {
            table_state.select(Some(0));
        }

        let initial_screen = if needs_unlock {
            Screen::Unlock
        } else {
            Screen::List
        };

        Ok(Self {
            screen: initial_screen,
            should_quit: false,
            registry,
            entries,
            keychain: KeychainBackend::new(),
            file_backend,
            file_unlocked: !needs_unlock,
            table_state,
            password_input: String::new(),
            unlock_error: None,
            add_service: String::new(),
            add_key: "token".to_string(),
            add_value: String::new(),
            add_backend: BackendType::EncryptedFile,
            add_label: String::new(),
            add_field_idx: 0,
            reveal_secret: false,
            secret_cache: None,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            clip_status: None,
            status_message: None,
        })
    }

    pub fn reload_entries(&mut self) -> Result<()> {
        self.registry = Registry::load(&registry_path())?;
        self.entries = self.registry.list().to_vec();
        if self.entries.is_empty() {
            self.table_state.select(None);
        } else if let Some(i) = self.table_state.selected() {
            if i >= self.entries.len() {
                self.table_state.select(Some(self.entries.len() - 1));
            }
        }
        Ok(())
    }

    /// Get the list of entries to display (filtered or all)
    pub fn visible_entries(&self) -> Vec<(usize, &TokenMeta)> {
        if self.screen == Screen::Search && !self.search_query.is_empty() {
            self.filtered_indices
                .iter()
                .map(|&i| (i, &self.entries[i]))
                .collect()
        } else {
            self.entries.iter().enumerate().collect()
        }
    }

    pub fn selected_entry(&self) -> Option<&TokenMeta> {
        let visible = self.visible_entries();
        self.table_state
            .selected()
            .and_then(|i| visible.get(i))
            .map(|(_, meta)| *meta)
    }

    pub fn selected_real_index(&self) -> Option<usize> {
        let visible = self.visible_entries();
        self.table_state
            .selected()
            .and_then(|i| visible.get(i))
            .map(|(real_idx, _)| *real_idx)
    }

    /// Handle a key event, returning true if we need to redraw
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Global quit
        if events::is_quit(&key) {
            self.should_quit = true;
            return true;
        }

        match self.screen.clone() {
            Screen::Unlock => self.handle_unlock_key(key),
            Screen::List => self.handle_list_key(key),
            Screen::Detail(idx) => self.handle_detail_key(key, idx),
            Screen::Add => self.handle_add_key(key),
            Screen::Confirm(action) => self.handle_confirm_key(key, action),
            Screen::Search => self.handle_search_key(key),
        }
    }

    fn handle_unlock_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                let pw = SecretString::from(self.password_input.clone());
                self.password_input.clear();
                match self.file_backend.unlock(&pw) {
                    Ok(()) => {
                        self.file_unlocked = true;
                        self.unlock_error = None;
                        self.screen = Screen::List;
                    }
                    Err(e) => {
                        self.unlock_error = Some(format!("Wrong password: {e}"));
                    }
                }
                true
            }
            KeyCode::Char(c) => {
                self.password_input.push(c);
                true
            }
            KeyCode::Backspace => {
                self.password_input.pop();
                true
            }
            KeyCode::Esc => {
                self.should_quit = true;
                true
            }
            _ => false,
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> bool {
        let visible_len = self.visible_entries().len();
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if visible_len > 0 {
                    let i = self.table_state.selected().unwrap_or(0);
                    self.table_state.select(Some((i + 1) % visible_len));
                }
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if visible_len > 0 {
                    let i = self.table_state.selected().unwrap_or(0);
                    let prev = if i == 0 { visible_len - 1 } else { i - 1 };
                    self.table_state.select(Some(prev));
                }
                true
            }
            KeyCode::Enter => {
                if let Some(idx) = self.selected_real_index() {
                    self.reveal_secret = false;
                    self.secret_cache = None;
                    self.screen = Screen::Detail(idx);
                }
                true
            }
            KeyCode::Char('a') => {
                self.add_service.clear();
                self.add_key = "token".to_string();
                self.add_value.clear();
                self.add_backend = BackendType::EncryptedFile;
                self.add_label.clear();
                self.add_field_idx = 0;
                self.screen = Screen::Add;
                true
            }
            KeyCode::Char('d') => {
                if let Some(idx) = self.selected_real_index() {
                    self.screen = Screen::Confirm(ConfirmAction::Delete(idx));
                }
                true
            }
            KeyCode::Char('c') => {
                self.copy_selected_token();
                true
            }
            KeyCode::Char('/') => {
                self.search_query.clear();
                self.filtered_indices = (0..self.entries.len()).collect();
                self.screen = Screen::Search;
                if !self.entries.is_empty() {
                    self.table_state.select(Some(0));
                }
                true
            }
            _ => false,
        }
    }

    fn handle_detail_key(&mut self, key: KeyEvent, _idx: usize) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.reveal_secret = false;
                self.secret_cache = None;
                self.screen = Screen::List;
                true
            }
            KeyCode::Char('v') => {
                if self.reveal_secret {
                    self.reveal_secret = false;
                    self.secret_cache = None;
                } else {
                    self.load_secret_for_detail();
                    self.reveal_secret = true;
                }
                true
            }
            KeyCode::Char('c') => {
                self.copy_selected_detail_token();
                true
            }
            _ => false,
        }
    }

    fn handle_add_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.screen = Screen::List;
                true
            }
            KeyCode::Tab => {
                self.add_field_idx = (self.add_field_idx + 1) % 5;
                true
            }
            KeyCode::BackTab => {
                self.add_field_idx = if self.add_field_idx == 0 { 4 } else { self.add_field_idx - 1 };
                true
            }
            KeyCode::Enter => {
                if self.add_field_idx == 3 {
                    // Toggle backend
                    self.add_backend = match self.add_backend {
                        BackendType::Keychain => BackendType::EncryptedFile,
                        BackendType::EncryptedFile => BackendType::Keychain,
                    };
                    return true;
                }

                // Submit if all required fields are filled
                if key.modifiers.contains(KeyModifiers::CONTROL) || self.add_field_idx == 4 {
                    return self.submit_add();
                }

                // Otherwise move to next field
                self.add_field_idx = (self.add_field_idx + 1) % 5;
                true
            }
            KeyCode::Char(c) => {
                self.current_add_field_mut().push(c);
                true
            }
            KeyCode::Backspace => {
                self.current_add_field_mut().pop();
                true
            }
            _ => false,
        }
    }

    fn current_add_field_mut(&mut self) -> &mut String {
        match self.add_field_idx {
            0 => &mut self.add_service,
            1 => &mut self.add_key,
            2 => &mut self.add_value,
            3 => &mut self.add_label, // backend toggle handled separately
            4 => &mut self.add_label,
            _ => unreachable!(),
        }
    }

    fn submit_add(&mut self) -> bool {
        if self.add_service.is_empty() {
            self.status_message = Some("Service name is required".into());
            return true;
        }
        if self.add_value.is_empty() {
            self.status_message = Some("Value is required".into());
            return true;
        }

        let value = SecretString::from(self.add_value.clone());
        let result = match self.add_backend {
            BackendType::Keychain => {
                self.keychain.set(&self.add_service, &self.add_key, &value)
            }
            BackendType::EncryptedFile => {
                self.file_backend.set(&self.add_service, &self.add_key, &value)
            }
        };

        match result {
            Ok(()) => {
                let mut meta = TokenMeta::new(&self.add_service, &self.add_key, self.add_backend.clone());
                if !self.add_label.is_empty() {
                    meta.label = Some(self.add_label.clone());
                }
                self.registry.upsert(meta);
                let _ = self.registry.save();
                let _ = self.reload_entries();
                self.status_message = Some(format!("Added {}:{}", self.add_service, self.add_key));
                self.screen = Screen::List;
            }
            Err(e) => {
                self.status_message = Some(format!("Error: {e}"));
            }
        }

        // Clear sensitive input
        self.add_value.clear();
        true
    }

    fn handle_confirm_key(&mut self, key: KeyEvent, action: ConfirmAction) -> bool {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                match action {
                    ConfirmAction::Delete(idx) => {
                        self.delete_entry(idx);
                    }
                }
                self.screen = Screen::List;
                true
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.screen = Screen::List;
                true
            }
            _ => false,
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.search_query.clear();
                self.screen = Screen::List;
                // Reset table state for full list
                if !self.entries.is_empty() {
                    self.table_state.select(Some(0));
                }
                true
            }
            KeyCode::Enter => {
                // Stay on filtered list, switch to list mode
                self.screen = Screen::List;
                true
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.update_search_filter();
                true
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_search_filter();
                true
            }
            KeyCode::Down => {
                let visible_len = self.filtered_indices.len();
                if visible_len > 0 {
                    let i = self.table_state.selected().unwrap_or(0);
                    self.table_state.select(Some((i + 1) % visible_len));
                }
                true
            }
            KeyCode::Up => {
                let visible_len = self.filtered_indices.len();
                if visible_len > 0 {
                    let i = self.table_state.selected().unwrap_or(0);
                    let prev = if i == 0 { visible_len - 1 } else { i - 1 };
                    self.table_state.select(Some(prev));
                }
                true
            }
            _ => false,
        }
    }

    fn update_search_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered_indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.service.to_lowercase().contains(&query)
                    || e.key.to_lowercase().contains(&query)
                    || e.label
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&query))
            })
            .map(|(i, _)| i)
            .collect();

        if self.filtered_indices.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }

    fn delete_entry(&mut self, idx: usize) {
        if idx >= self.entries.len() {
            return;
        }

        let entry = &self.entries[idx];
        let service = entry.service.clone();
        let key = entry.key.clone();
        let backend = entry.backend.clone();

        let result = match backend {
            BackendType::Keychain => self.keychain.delete(&service, &key),
            BackendType::EncryptedFile => self.file_backend.delete(&service, &key),
        };

        match result {
            Ok(()) => {
                self.registry.remove(&service, &key);
                let _ = self.registry.save();
                let _ = self.reload_entries();
                self.status_message = Some(format!("Deleted {service}:{key}"));
            }
            Err(e) => {
                self.status_message = Some(format!("Delete failed: {e}"));
            }
        }
    }

    fn load_secret_for_detail(&mut self) {
        if let Some(entry) = self.selected_entry().cloned() {
            let result = match entry.backend {
                BackendType::Keychain => self.keychain.get(&entry.service, &entry.key),
                BackendType::EncryptedFile => self.file_backend.get(&entry.service, &entry.key),
            };
            match result {
                Ok(secret) => {
                    self.secret_cache = Some(secret.expose_secret().to_string());
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to read secret: {e}"));
                }
            }
        }
    }

    fn copy_selected_token(&mut self) {
        if let Some(entry) = self.selected_entry().cloned() {
            let result = match entry.backend {
                BackendType::Keychain => self.keychain.get(&entry.service, &entry.key),
                BackendType::EncryptedFile => self.file_backend.get(&entry.service, &entry.key),
            };
            match result {
                Ok(secret) => {
                    self.copy_to_clipboard(secret.expose_secret(), &entry.service);
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to read: {e}"));
                }
            }
        }
    }

    fn copy_selected_detail_token(&mut self) {
        // Use cached secret if available, otherwise load it
        let value = if let Some(ref cached) = self.secret_cache {
            cached.clone()
        } else {
            if let Some(entry) = self.selected_entry().cloned() {
                let result = match entry.backend {
                    BackendType::Keychain => self.keychain.get(&entry.service, &entry.key),
                    BackendType::EncryptedFile => self.file_backend.get(&entry.service, &entry.key),
                };
                match result {
                    Ok(secret) => secret.expose_secret().to_string(),
                    Err(e) => {
                        self.status_message = Some(format!("Failed to read: {e}"));
                        return;
                    }
                }
            } else {
                return;
            }
        };
        if let Screen::Detail(idx) = self.screen {
            let svc = self.entries.get(idx)
                .map(|e| e.service.clone())
                .unwrap_or_else(|| "token".to_string());
            self.copy_to_clipboard(&value, &svc);
        }
    }

    fn copy_to_clipboard(&mut self, value: &str, label: &str) {
        match arboard::Clipboard::new() {
            Ok(mut cb) => {
                if cb.set_text(value.to_string()).is_ok() {
                    self.clip_status = Some(ClipStatus {
                        message: format!("Copied {label} to clipboard"),
                        expires: Instant::now() + Duration::from_secs(30),
                    });
                    // Schedule clipboard clear
                    let _ = std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_secs(30));
                        if let Ok(mut cb) = arboard::Clipboard::new() {
                            let _ = cb.set_text(String::new());
                        }
                    });
                } else {
                    self.status_message = Some("Failed to copy to clipboard".into());
                }
            }
            Err(_) => {
                self.status_message = Some("Clipboard not available".into());
            }
        }
    }

    /// Check and clear expired status messages
    pub fn tick(&mut self) {
        if let Some(ref clip) = self.clip_status {
            if Instant::now() > clip.expires {
                self.clip_status = None;
            }
        }
    }
}

/// Entry point for TUI mode
pub fn run_tui() -> Result<()> {
    use crate::tui::screens;

    let mut app = App::new()?;

    // If vault doesn't exist, prompt to init first
    if !app.file_backend.vault_exists() {
        eprintln!("tkm is not initialized. Run `tkm init` first.");
        return Ok(());
    }

    let mut terminal = ratatui::init();

    loop {
        // Draw
        terminal.draw(|frame| {
            match &app.screen {
                Screen::Unlock => screens::unlock::render(frame, &app),
                Screen::List | Screen::Search => screens::list::render(frame, &mut app),
                Screen::Detail(idx) => screens::detail::render(frame, &app, *idx),
                Screen::Add => screens::add::render(frame, &app),
                Screen::Confirm(action) => screens::confirm::render(frame, &app, &action.clone()),
            }
        })?;

        if app.should_quit {
            break;
        }

        // Handle input
        if let Some(key) = events::poll_key(Duration::from_millis(100)) {
            app.handle_key(key);
            // Clear status message on any key (except when it was just set)
            // status_message is a one-shot, clip_status has its own expiry
        }

        app.tick();
    }

    ratatui::restore();
    Ok(())
}
