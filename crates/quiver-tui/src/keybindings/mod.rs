use serde::{Deserialize, Serialize};

/// Editor input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyMode {
    /// Standard insert mode — keystrokes produce characters.
    Normal,
    /// Vim normal mode — keystrokes are motions/commands.
    VimNormal,
    /// Vim insert mode — keystrokes produce characters.
    VimInsert,
    /// Vim visual mode — keystrokes extend selection.
    VimVisual,
    /// Emacs mode — Ctrl/Alt combos for movement/editing.
    Emacs,
}

impl KeyMode {
    pub fn label(&self) -> &'static str {
        match self {
            KeyMode::Normal => "INSERT",
            KeyMode::VimNormal => "NORMAL",
            KeyMode::VimInsert => "INSERT",
            KeyMode::VimVisual => "VISUAL",
            KeyMode::Emacs => "EMACS",
        }
    }

    /// Auto-detect preferred mode from environment.
    /// Checks $EDITOR / $VISUAL and presence of config files.
    pub fn detect_from_env() -> Self {
        if let Ok(editor) = std::env::var("EDITOR").or_else(|_| std::env::var("VISUAL")) {
            let e = editor.to_lowercase();
            if e.contains("vim") || e.contains("nvim") {
                return KeyMode::VimNormal;
            }
            if e.contains("emacs") {
                return KeyMode::Emacs;
            }
        }

        // Check for config files
        if let Some(home) = dirs::home_dir() {
            if home.join(".vimrc").exists()
                || home.join(".config/nvim/init.vim").exists()
                || home.join(".config/nvim/init.lua").exists()
            {
                return KeyMode::VimNormal;
            }
            if home.join(".emacs").exists()
                || home.join(".emacs.d").exists()
                || home.join(".doom.d").exists()
            {
                return KeyMode::Emacs;
            }
        }

        KeyMode::Normal
    }
}
