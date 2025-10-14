use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Keymap {
    pub quit: String,
    pub toggle_sidebar: String,
    pub open_page_switcher: String,
    pub create_new_page: String,
    pub delete_current_page: String,
    pub toggle_favorite: String,
    pub open_logbook: String,
    pub export: String,
    pub attach: String,
    pub open_attachment: String,
    pub attachments_select_up: String,
    pub attachments_select_down: String,
    pub sidebar_select_up: String,
    pub sidebar_select_down: String,
    pub sidebar_activate: String,
    pub move_up: String,
    pub move_down: String,
    pub cursor_up: String,
    pub cursor_down: String,
    pub expand: String,
    pub collapse: String,
    pub start_editing: String,
    pub create_sibling: String,
    pub initiate_delete: String,
    pub task_overview: String,
    pub clear_tag_filter: String,
    pub paste: String,
    pub rename_page: String,
    pub help: String,
    pub create_quote_block: String,
    pub create_code_block: String,
    pub toggle_task: String,
    pub search: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub keymap: Keymap,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keymap: Keymap {
                quit: "q".to_string(),
                toggle_sidebar: "ctrl-b".to_string(),
                open_page_switcher: "ctrl-p".to_string(),
                create_new_page: "ctrl-n".to_string(),
                delete_current_page: "ctrl-d".to_string(),
                toggle_favorite: "ctrl-f".to_string(),
                open_logbook: "ctrl-l".to_string(),
                export: "ctrl-e".to_string(),
                attach: "ctrl-a".to_string(),
                open_attachment: "ctrl-o".to_string(),
                attachments_select_up: "[".to_string(),
                attachments_select_down: "]".to_string(),
                sidebar_select_up: "pageup".to_string(),
                sidebar_select_down: "pagedown".to_string(),
                sidebar_activate: "alt-enter".to_string(),
                move_up: "alt-up".to_string(),
                move_down: "alt-down".to_string(),
                cursor_up: "up".to_string(),
                cursor_down: "down".to_string(),
                expand: "right".to_string(),
                collapse: "left".to_string(),
                start_editing: "enter".to_string(),
                create_sibling: "n".to_string(),
                initiate_delete: "d".to_string(),
                task_overview: "ctrl-shift-t".to_string(),
                clear_tag_filter: "ctrl-t".to_string(),
                paste: "ctrl-v".to_string(),
                rename_page: "ctrl-r".to_string(),
                help: "h".to_string(),
                create_quote_block: "ctrl-q".to_string(),
                create_code_block: "ctrl-c".to_string(),
                toggle_task: "x".to_string(),
                search: "/".to_string(),
            },
        }
    }
}

pub fn load_config(path: &PathBuf) -> Config {
    if !path.exists() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        fs::write(path, toml).expect("Failed to write default config");
        return config;
    }

    let content = fs::read_to_string(path).expect("Failed to read config file");
    toml::from_str(&content).expect("Failed to parse config file")
}
