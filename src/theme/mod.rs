use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

// ── Theme kind (built-in catalog) ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeKind {
    TokyoNight,
    CatppuccinMocha,
    Gruvbox,
    Nord,
    Dracula,
    SolarizedDark,
    RosePine,
}

impl ThemeKind {
    pub fn label(&self) -> &'static str {
        match self {
            ThemeKind::TokyoNight => "Tokyo Night",
            ThemeKind::CatppuccinMocha => "Catppuccin Mocha",
            ThemeKind::Gruvbox => "Gruvbox",
            ThemeKind::Nord => "Nord",
            ThemeKind::Dracula => "Dracula",
            ThemeKind::SolarizedDark => "Solarized Dark",
            ThemeKind::RosePine => "Rosé Pine",
        }
    }

    pub fn cycle_next(&self) -> Self {
        match self {
            ThemeKind::TokyoNight => ThemeKind::CatppuccinMocha,
            ThemeKind::CatppuccinMocha => ThemeKind::Gruvbox,
            ThemeKind::Gruvbox => ThemeKind::Nord,
            ThemeKind::Nord => ThemeKind::Dracula,
            ThemeKind::Dracula => ThemeKind::SolarizedDark,
            ThemeKind::SolarizedDark => ThemeKind::RosePine,
            ThemeKind::RosePine => ThemeKind::TokyoNight,
        }
    }
}

// ── Theme struct ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,

    // Chrome
    pub bg: Color,
    pub fg: Color,
    pub border: Style,
    pub border_focused: Style,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,

    // Tabs
    pub tab_active: Style,
    pub tab_inactive: Style,
    pub tab_bar_bg: Color,

    // Editor
    pub editor_bg: Color,
    pub editor_fg: Color,
    pub editor_line_number: Style,
    pub editor_cursor_line: Style,

    // SQL syntax (placeholder for tree-sitter integration)
    pub sql_keyword: Style,
    pub sql_string: Style,
    pub sql_number: Style,
    pub sql_comment: Style,
    pub sql_function: Style,
    pub sql_operator: Style,
    pub sql_identifier: Style,

    // Results
    pub result_header: Style,
    pub result_cell: Style,
    pub result_cell_alt: Style,
    pub result_selected: Style,
    pub result_null: Style,

    // Type badges (column type indicators)
    pub type_integer: Style,
    pub type_float: Style,
    pub type_string: Style,
    pub type_temporal: Style,
    pub type_boolean: Style,
    pub type_binary: Style,
    pub type_nested: Style,

    // Command palette
    pub palette_bg: Color,
    pub palette_border: Style,
    pub palette_input: Style,
    pub palette_item: Style,
    pub palette_item_selected: Style,
    pub palette_match_char: Style,

    // Schema browser
    pub tree_node: Style,
    pub tree_node_selected: Style,
    pub tree_icon: Style,

    // Semantic
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub info: Style,
    pub accent: Color,
}

impl Theme {
    pub fn builtin(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::TokyoNight => Self::tokyo_night(),
            ThemeKind::CatppuccinMocha => Self::catppuccin_mocha(),
            ThemeKind::Gruvbox => Self::gruvbox(),
            ThemeKind::Nord => Self::nord(),
            ThemeKind::Dracula => Self::dracula(),
            ThemeKind::SolarizedDark => Self::solarized_dark(),
            ThemeKind::RosePine => Self::rose_pine(),
        }
    }

    // ── Tokyo Night ───────────────────────────────────────────

    fn tokyo_night() -> Self {
        let bg = Color::Rgb(26, 27, 38);
        let fg = Color::Rgb(169, 177, 214);
        let blue = Color::Rgb(122, 162, 247);
        let cyan = Color::Rgb(125, 207, 255);
        let green = Color::Rgb(158, 206, 106);
        let yellow = Color::Rgb(224, 175, 104);
        let magenta = Color::Rgb(187, 154, 247);
        let red = Color::Rgb(247, 118, 142);
        let orange = Color::Rgb(255, 158, 100);
        let comment = Color::Rgb(86, 95, 137);
        let surface = Color::Rgb(36, 40, 59);
        let surface2 = Color::Rgb(41, 46, 66);

        Self {
            name: "Tokyo Night".into(),
            bg,
            fg,
            border: Style::default().fg(Color::Rgb(59, 66, 97)),
            border_focused: Style::default().fg(blue),
            status_bar_bg: surface,
            status_bar_fg: fg,
            tab_active: Style::default()
                .fg(bg)
                .bg(blue)
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(comment).bg(surface),
            tab_bar_bg: surface,
            editor_bg: bg,
            editor_fg: fg,
            editor_line_number: Style::default().fg(comment),
            editor_cursor_line: Style::default().bg(surface2),
            sql_keyword: Style::default().fg(magenta).add_modifier(Modifier::BOLD),
            sql_string: Style::default().fg(green),
            sql_number: Style::default().fg(orange),
            sql_comment: Style::default().fg(comment).add_modifier(Modifier::ITALIC),
            sql_function: Style::default().fg(blue),
            sql_operator: Style::default().fg(cyan),
            sql_identifier: Style::default().fg(fg),
            result_header: Style::default().fg(blue).add_modifier(Modifier::BOLD),
            result_cell: Style::default().fg(fg),
            result_cell_alt: Style::default().fg(fg).bg(surface),
            result_selected: Style::default().fg(bg).bg(blue),
            result_null: Style::default().fg(comment).add_modifier(Modifier::DIM),
            type_integer: Style::default().fg(cyan),
            type_float: Style::default().fg(green),
            type_string: Style::default().fg(yellow),
            type_temporal: Style::default().fg(magenta),
            type_boolean: Style::default().fg(blue),
            type_binary: Style::default().fg(red),
            type_nested: Style::default().fg(orange),
            palette_bg: surface,
            palette_border: Style::default().fg(blue),
            palette_input: Style::default().fg(fg),
            palette_item: Style::default().fg(fg),
            palette_item_selected: Style::default().fg(bg).bg(blue),
            palette_match_char: Style::default().fg(yellow).add_modifier(Modifier::BOLD),
            tree_node: Style::default().fg(fg),
            tree_node_selected: Style::default().fg(bg).bg(blue),
            tree_icon: Style::default().fg(cyan),
            success: Style::default().fg(green),
            warning: Style::default().fg(yellow),
            error: Style::default().fg(red),
            info: Style::default().fg(blue),
            accent: blue,
        }
    }

    // ── Catppuccin Mocha ──────────────────────────────────────

    fn catppuccin_mocha() -> Self {
        let base = Color::Rgb(30, 30, 46);
        let text = Color::Rgb(205, 214, 244);
        let blue = Color::Rgb(137, 180, 250);
        let green = Color::Rgb(166, 227, 161);
        let yellow = Color::Rgb(249, 226, 175);
        let mauve = Color::Rgb(203, 166, 247);
        let red = Color::Rgb(243, 139, 168);
        let peach = Color::Rgb(250, 179, 135);
        let teal = Color::Rgb(148, 226, 213);
        let overlay0 = Color::Rgb(108, 112, 134);
        let surface0 = Color::Rgb(49, 50, 68);
        let surface1 = Color::Rgb(69, 71, 90);

        Self {
            name: "Catppuccin Mocha".into(),
            bg: base,
            fg: text,
            border: Style::default().fg(surface1),
            border_focused: Style::default().fg(mauve),
            status_bar_bg: surface0,
            status_bar_fg: text,
            tab_active: Style::default()
                .fg(base)
                .bg(mauve)
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(overlay0).bg(surface0),
            tab_bar_bg: surface0,
            editor_bg: base,
            editor_fg: text,
            editor_line_number: Style::default().fg(overlay0),
            editor_cursor_line: Style::default().bg(surface0),
            sql_keyword: Style::default().fg(mauve).add_modifier(Modifier::BOLD),
            sql_string: Style::default().fg(green),
            sql_number: Style::default().fg(peach),
            sql_comment: Style::default().fg(overlay0).add_modifier(Modifier::ITALIC),
            sql_function: Style::default().fg(blue),
            sql_operator: Style::default().fg(teal),
            sql_identifier: Style::default().fg(text),
            result_header: Style::default().fg(mauve).add_modifier(Modifier::BOLD),
            result_cell: Style::default().fg(text),
            result_cell_alt: Style::default().fg(text).bg(surface0),
            result_selected: Style::default().fg(base).bg(mauve),
            result_null: Style::default().fg(overlay0).add_modifier(Modifier::DIM),
            type_integer: Style::default().fg(teal),
            type_float: Style::default().fg(green),
            type_string: Style::default().fg(yellow),
            type_temporal: Style::default().fg(mauve),
            type_boolean: Style::default().fg(blue),
            type_binary: Style::default().fg(red),
            type_nested: Style::default().fg(peach),
            palette_bg: surface0,
            palette_border: Style::default().fg(mauve),
            palette_input: Style::default().fg(text),
            palette_item: Style::default().fg(text),
            palette_item_selected: Style::default().fg(base).bg(mauve),
            palette_match_char: Style::default().fg(yellow).add_modifier(Modifier::BOLD),
            tree_node: Style::default().fg(text),
            tree_node_selected: Style::default().fg(base).bg(mauve),
            tree_icon: Style::default().fg(teal),
            success: Style::default().fg(green),
            warning: Style::default().fg(yellow),
            error: Style::default().fg(red),
            info: Style::default().fg(blue),
            accent: mauve,
        }
    }

    // ── Gruvbox ───────────────────────────────────────────────

    fn gruvbox() -> Self {
        let bg = Color::Rgb(40, 40, 40);
        let fg = Color::Rgb(235, 219, 178);
        let yellow = Color::Rgb(250, 189, 47);
        let green = Color::Rgb(184, 187, 38);
        let blue = Color::Rgb(131, 165, 152);
        let aqua = Color::Rgb(142, 192, 124);
        let red = Color::Rgb(251, 73, 52);
        let orange = Color::Rgb(254, 128, 25);
        let purple = Color::Rgb(211, 134, 155);
        let gray = Color::Rgb(146, 131, 116);
        let bg1 = Color::Rgb(60, 56, 54);
        let bg2 = Color::Rgb(80, 73, 69);

        Self {
            name: "Gruvbox".into(),
            bg,
            fg,
            border: Style::default().fg(bg2),
            border_focused: Style::default().fg(yellow),
            status_bar_bg: bg1,
            status_bar_fg: fg,
            tab_active: Style::default()
                .fg(bg)
                .bg(yellow)
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(gray).bg(bg1),
            tab_bar_bg: bg1,
            editor_bg: bg,
            editor_fg: fg,
            editor_line_number: Style::default().fg(gray),
            editor_cursor_line: Style::default().bg(bg1),
            sql_keyword: Style::default().fg(orange).add_modifier(Modifier::BOLD),
            sql_string: Style::default().fg(green),
            sql_number: Style::default().fg(purple),
            sql_comment: Style::default().fg(gray).add_modifier(Modifier::ITALIC),
            sql_function: Style::default().fg(aqua),
            sql_operator: Style::default().fg(fg),
            sql_identifier: Style::default().fg(fg),
            result_header: Style::default().fg(yellow).add_modifier(Modifier::BOLD),
            result_cell: Style::default().fg(fg),
            result_cell_alt: Style::default().fg(fg).bg(bg1),
            result_selected: Style::default().fg(bg).bg(yellow),
            result_null: Style::default().fg(gray).add_modifier(Modifier::DIM),
            type_integer: Style::default().fg(blue),
            type_float: Style::default().fg(aqua),
            type_string: Style::default().fg(yellow),
            type_temporal: Style::default().fg(purple),
            type_boolean: Style::default().fg(orange),
            type_binary: Style::default().fg(red),
            type_nested: Style::default().fg(green),
            palette_bg: bg1,
            palette_border: Style::default().fg(yellow),
            palette_input: Style::default().fg(fg),
            palette_item: Style::default().fg(fg),
            palette_item_selected: Style::default().fg(bg).bg(yellow),
            palette_match_char: Style::default().fg(orange).add_modifier(Modifier::BOLD),
            tree_node: Style::default().fg(fg),
            tree_node_selected: Style::default().fg(bg).bg(yellow),
            tree_icon: Style::default().fg(aqua),
            success: Style::default().fg(green),
            warning: Style::default().fg(yellow),
            error: Style::default().fg(red),
            info: Style::default().fg(blue),
            accent: yellow,
        }
    }

    // ── Nord ──────────────────────────────────────────────────

    fn nord() -> Self {
        let bg = Color::Rgb(46, 52, 64);
        let fg = Color::Rgb(216, 222, 233);
        let frost0 = Color::Rgb(143, 188, 187);
        let frost1 = Color::Rgb(136, 192, 208);
        let frost2 = Color::Rgb(129, 161, 193);
        let frost3 = Color::Rgb(94, 129, 172);
        let green = Color::Rgb(163, 190, 140);
        let yellow = Color::Rgb(235, 203, 139);
        let red = Color::Rgb(191, 97, 106);
        let orange = Color::Rgb(208, 135, 112);
        let purple = Color::Rgb(180, 142, 173);
        let comment = Color::Rgb(76, 86, 106);
        let surface = Color::Rgb(59, 66, 82);

        Self {
            name: "Nord".into(),
            bg,
            fg,
            border: Style::default().fg(comment),
            border_focused: Style::default().fg(frost1),
            status_bar_bg: surface,
            status_bar_fg: fg,
            tab_active: Style::default()
                .fg(bg)
                .bg(frost1)
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(comment).bg(surface),
            tab_bar_bg: surface,
            editor_bg: bg,
            editor_fg: fg,
            editor_line_number: Style::default().fg(comment),
            editor_cursor_line: Style::default().bg(surface),
            sql_keyword: Style::default().fg(frost3).add_modifier(Modifier::BOLD),
            sql_string: Style::default().fg(green),
            sql_number: Style::default().fg(purple),
            sql_comment: Style::default().fg(comment).add_modifier(Modifier::ITALIC),
            sql_function: Style::default().fg(frost1),
            sql_operator: Style::default().fg(frost0),
            sql_identifier: Style::default().fg(fg),
            result_header: Style::default().fg(frost1).add_modifier(Modifier::BOLD),
            result_cell: Style::default().fg(fg),
            result_cell_alt: Style::default().fg(fg).bg(surface),
            result_selected: Style::default().fg(bg).bg(frost1),
            result_null: Style::default().fg(comment).add_modifier(Modifier::DIM),
            type_integer: Style::default().fg(frost0),
            type_float: Style::default().fg(green),
            type_string: Style::default().fg(yellow),
            type_temporal: Style::default().fg(purple),
            type_boolean: Style::default().fg(frost2),
            type_binary: Style::default().fg(red),
            type_nested: Style::default().fg(orange),
            palette_bg: surface,
            palette_border: Style::default().fg(frost1),
            palette_input: Style::default().fg(fg),
            palette_item: Style::default().fg(fg),
            palette_item_selected: Style::default().fg(bg).bg(frost1),
            palette_match_char: Style::default().fg(yellow).add_modifier(Modifier::BOLD),
            tree_node: Style::default().fg(fg),
            tree_node_selected: Style::default().fg(bg).bg(frost1),
            tree_icon: Style::default().fg(frost0),
            success: Style::default().fg(green),
            warning: Style::default().fg(yellow),
            error: Style::default().fg(red),
            info: Style::default().fg(frost1),
            accent: frost1,
        }
    }

    // ── Dracula ───────────────────────────────────────────────

    fn dracula() -> Self {
        let bg = Color::Rgb(40, 42, 54);
        let fg = Color::Rgb(248, 248, 242);
        let purple = Color::Rgb(189, 147, 249);
        let green = Color::Rgb(80, 250, 123);
        let pink = Color::Rgb(255, 121, 198);
        let cyan = Color::Rgb(139, 233, 253);
        let yellow = Color::Rgb(241, 250, 140);
        let orange = Color::Rgb(255, 184, 108);
        let red = Color::Rgb(255, 85, 85);
        let comment = Color::Rgb(98, 114, 164);
        let current = Color::Rgb(68, 71, 90);

        Self {
            name: "Dracula".into(),
            bg,
            fg,
            border: Style::default().fg(comment),
            border_focused: Style::default().fg(purple),
            status_bar_bg: current,
            status_bar_fg: fg,
            tab_active: Style::default()
                .fg(bg)
                .bg(purple)
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(comment).bg(current),
            tab_bar_bg: current,
            editor_bg: bg,
            editor_fg: fg,
            editor_line_number: Style::default().fg(comment),
            editor_cursor_line: Style::default().bg(current),
            sql_keyword: Style::default().fg(pink).add_modifier(Modifier::BOLD),
            sql_string: Style::default().fg(yellow),
            sql_number: Style::default().fg(purple),
            sql_comment: Style::default().fg(comment).add_modifier(Modifier::ITALIC),
            sql_function: Style::default().fg(green),
            sql_operator: Style::default().fg(pink),
            sql_identifier: Style::default().fg(fg),
            result_header: Style::default().fg(purple).add_modifier(Modifier::BOLD),
            result_cell: Style::default().fg(fg),
            result_cell_alt: Style::default().fg(fg).bg(current),
            result_selected: Style::default().fg(bg).bg(purple),
            result_null: Style::default().fg(comment).add_modifier(Modifier::DIM),
            type_integer: Style::default().fg(cyan),
            type_float: Style::default().fg(green),
            type_string: Style::default().fg(yellow),
            type_temporal: Style::default().fg(pink),
            type_boolean: Style::default().fg(purple),
            type_binary: Style::default().fg(red),
            type_nested: Style::default().fg(orange),
            palette_bg: current,
            palette_border: Style::default().fg(purple),
            palette_input: Style::default().fg(fg),
            palette_item: Style::default().fg(fg),
            palette_item_selected: Style::default().fg(bg).bg(purple),
            palette_match_char: Style::default().fg(cyan).add_modifier(Modifier::BOLD),
            tree_node: Style::default().fg(fg),
            tree_node_selected: Style::default().fg(bg).bg(purple),
            tree_icon: Style::default().fg(cyan),
            success: Style::default().fg(green),
            warning: Style::default().fg(yellow),
            error: Style::default().fg(red),
            info: Style::default().fg(cyan),
            accent: purple,
        }
    }

    // ── Solarized Dark ────────────────────────────────────────

    fn solarized_dark() -> Self {
        let base03 = Color::Rgb(0, 43, 54);
        let base02 = Color::Rgb(7, 54, 66);
        let base01 = Color::Rgb(88, 110, 117);
        let base0 = Color::Rgb(131, 148, 150);
        let blue = Color::Rgb(38, 139, 210);
        let cyan = Color::Rgb(42, 161, 152);
        let green = Color::Rgb(133, 153, 0);
        let yellow = Color::Rgb(181, 137, 0);
        let orange = Color::Rgb(203, 75, 22);
        let red = Color::Rgb(220, 50, 47);
        let magenta = Color::Rgb(211, 54, 130);
        let violet = Color::Rgb(108, 113, 196);

        Self {
            name: "Solarized Dark".into(),
            bg: base03,
            fg: base0,
            border: Style::default().fg(base01),
            border_focused: Style::default().fg(blue),
            status_bar_bg: base02,
            status_bar_fg: base0,
            tab_active: Style::default()
                .fg(base03)
                .bg(blue)
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(base01).bg(base02),
            tab_bar_bg: base02,
            editor_bg: base03,
            editor_fg: base0,
            editor_line_number: Style::default().fg(base01),
            editor_cursor_line: Style::default().bg(base02),
            sql_keyword: Style::default().fg(green).add_modifier(Modifier::BOLD),
            sql_string: Style::default().fg(cyan),
            sql_number: Style::default().fg(magenta),
            sql_comment: Style::default().fg(base01).add_modifier(Modifier::ITALIC),
            sql_function: Style::default().fg(blue),
            sql_operator: Style::default().fg(orange),
            sql_identifier: Style::default().fg(base0),
            result_header: Style::default().fg(blue).add_modifier(Modifier::BOLD),
            result_cell: Style::default().fg(base0),
            result_cell_alt: Style::default().fg(base0).bg(base02),
            result_selected: Style::default().fg(base03).bg(blue),
            result_null: Style::default().fg(base01).add_modifier(Modifier::DIM),
            type_integer: Style::default().fg(cyan),
            type_float: Style::default().fg(green),
            type_string: Style::default().fg(yellow),
            type_temporal: Style::default().fg(violet),
            type_boolean: Style::default().fg(blue),
            type_binary: Style::default().fg(red),
            type_nested: Style::default().fg(orange),
            palette_bg: base02,
            palette_border: Style::default().fg(blue),
            palette_input: Style::default().fg(base0),
            palette_item: Style::default().fg(base0),
            palette_item_selected: Style::default().fg(base03).bg(blue),
            palette_match_char: Style::default().fg(yellow).add_modifier(Modifier::BOLD),
            tree_node: Style::default().fg(base0),
            tree_node_selected: Style::default().fg(base03).bg(blue),
            tree_icon: Style::default().fg(cyan),
            success: Style::default().fg(green),
            warning: Style::default().fg(yellow),
            error: Style::default().fg(red),
            info: Style::default().fg(blue),
            accent: blue,
        }
    }

    // ── Rosé Pine ─────────────────────────────────────────────

    fn rose_pine() -> Self {
        let base = Color::Rgb(25, 23, 36);
        let text = Color::Rgb(224, 222, 244);
        let rose = Color::Rgb(235, 188, 186);
        let gold = Color::Rgb(246, 193, 119);
        let pine = Color::Rgb(49, 116, 143);
        let foam = Color::Rgb(156, 207, 216);
        let iris = Color::Rgb(196, 167, 231);
        let love = Color::Rgb(235, 111, 146);
        let subtle = Color::Rgb(110, 106, 134);
        let surface = Color::Rgb(42, 39, 63);
        let overlay = Color::Rgb(57, 53, 82);

        Self {
            name: "Rosé Pine".into(),
            bg: base,
            fg: text,
            border: Style::default().fg(subtle),
            border_focused: Style::default().fg(iris),
            status_bar_bg: surface,
            status_bar_fg: text,
            tab_active: Style::default()
                .fg(base)
                .bg(iris)
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(subtle).bg(surface),
            tab_bar_bg: surface,
            editor_bg: base,
            editor_fg: text,
            editor_line_number: Style::default().fg(subtle),
            editor_cursor_line: Style::default().bg(surface),
            sql_keyword: Style::default().fg(iris).add_modifier(Modifier::BOLD),
            sql_string: Style::default().fg(gold),
            sql_number: Style::default().fg(rose),
            sql_comment: Style::default().fg(subtle).add_modifier(Modifier::ITALIC),
            sql_function: Style::default().fg(foam),
            sql_operator: Style::default().fg(rose),
            sql_identifier: Style::default().fg(text),
            result_header: Style::default().fg(iris).add_modifier(Modifier::BOLD),
            result_cell: Style::default().fg(text),
            result_cell_alt: Style::default().fg(text).bg(surface),
            result_selected: Style::default().fg(base).bg(iris),
            result_null: Style::default().fg(subtle).add_modifier(Modifier::DIM),
            type_integer: Style::default().fg(foam),
            type_float: Style::default().fg(pine),
            type_string: Style::default().fg(gold),
            type_temporal: Style::default().fg(iris),
            type_boolean: Style::default().fg(rose),
            type_binary: Style::default().fg(love),
            type_nested: Style::default().fg(gold),
            palette_bg: surface,
            palette_border: Style::default().fg(iris),
            palette_input: Style::default().fg(text),
            palette_item: Style::default().fg(text),
            palette_item_selected: Style::default().fg(base).bg(iris),
            palette_match_char: Style::default().fg(gold).add_modifier(Modifier::BOLD),
            tree_node: Style::default().fg(text),
            tree_node_selected: Style::default().fg(base).bg(iris),
            tree_icon: Style::default().fg(foam),
            success: Style::default().fg(foam),
            warning: Style::default().fg(gold),
            error: Style::default().fg(love),
            info: Style::default().fg(iris),
            accent: iris,
        }
    }
}
