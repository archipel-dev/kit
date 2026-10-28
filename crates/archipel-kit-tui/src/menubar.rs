//! [`MenuBarControl`] and [`MenuPopup`]: a [`model::MenuBar`] as a row of titles with
//! drop-down menus (TUI-GUI-035..040), and a [`model::MenuEntry`] list as a popup menu.
//!
//! The titles are padded by one cell on each side with the mnemonic letter underlined; the
//! open title has the `hover` fill. A click opens its menu below the title (the menu's top
//! edge is the row under the bar); while a menu is open, hovering another title switches to
//! it; `Left` and `Right` move between the titles (when no submenu is open), `Esc` closes
//! the menu and a second `Esc` collapses the bar. `Alt` plus a title's mnemonic opens it,
//! `F10` focuses the first title.
//!
//! The menus are [`PopupStyle::Island`] popups: a rounded box on the popup surface, a
//! 2-cell icon column (the check mark of a toggle too), the label with its underlined
//! mnemonic, the shortcut right aligned in the secondary color and a 2-cell submenu arrow
//! column. Submenus open to the right after 250 ms of hover, at once on `Right` or `Enter`.

use crate::chrome::{self, Placed, Tokens};
use crate::components::{RowIcon, fill, put, text_width};
use crate::hit::Id;
use crate::input::{Event, KeyCode, KeyEventKind, KeyModifiers};
use crate::pointer::PointerKind;
use crate::popup::{
    Chosen, LevelSpec, Placement, PopupHost, PopupOptions, PopupSource, PopupState, PopupStyle,
    RowSpec,
};
use crate::runtime::Runtime;
use archipel_kit_model::{
    IconKey, MenuBar, MenuEntry, MenuEvent, MenuItem, Notation, Outline, OutlineEntry,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

fn row_icon(icon: &IconKey) -> Option<RowIcon> {
    Some(match icon {
        IconKey::Glyph(glyph) => RowIcon::Glyph(*glyph),
        IconKey::File(name) => match crate::Language::from_file_name(name) {
            Some(language) => RowIcon::Language(language),
            None => RowIcon::Glyph(crate::Glyph::File),
        },
        IconKey::Folder { open } => RowIcon::Folder(*open),
        _ => return None,
    })
}

/// The rows of a menu: the items of `entries`, separators dropped.
fn items(entries: &[MenuEntry]) -> Vec<&MenuItem> {
    entries
        .iter()
        .filter_map(|e| match e {
            MenuEntry::Item(item) => Some(item),
            _ => None,
        })
        .collect()
}

/// A [`PopupSource`] over model menu entries.
#[derive(Debug, Clone)]
pub struct MenuSource {
    entries: Vec<MenuEntry>,
    notation: Notation,
    chosen: Option<String>,
}

impl MenuSource {
    /// A source for `entries`; shortcuts are written in `notation`.
    pub fn new(entries: Vec<MenuEntry>, notation: Notation) -> Self {
        Self {
            entries,
            notation,
            chosen: None,
        }
    }

    /// Replaces the entries (the model changed); open levels stay when their rows still
    /// exist.
    pub fn set_entries(&mut self, entries: Vec<MenuEntry>) {
        self.entries = entries;
    }

    /// The entries at `path` (indices of rows, separators not counted).
    fn at<'a>(&'a self, path: &[usize]) -> Option<&'a [MenuEntry]> {
        let mut entries: &[MenuEntry] = &self.entries;
        for &index in path {
            let item = items(entries).into_iter().nth(index)?;
            entries = item.submenu.as_deref()?;
        }
        Some(entries)
    }

    /// The id of the chosen row, once the popup finished with a choice.
    pub fn take_chosen(&mut self) -> Option<String> {
        self.chosen.take()
    }
}

impl PopupSource for MenuSource {
    fn level(&mut self, path: &[usize]) -> Option<LevelSpec> {
        let entries = self.at(path)?;
        let mut rows: Vec<RowSpec> = Vec::new();
        let mut separator: Option<Option<String>> = None;
        for entry in entries {
            match entry {
                MenuEntry::Separator { title } => separator = Some(title.clone()),
                MenuEntry::Item(item) => rows.push(RowSpec {
                    text: item.label.clone(),
                    icon: item.icon.as_ref().and_then(row_icon),
                    shortcut: item.shortcut.as_ref().map(|s| s.label(self.notation)),
                    checked: item.checked,
                    selectable: item.enabled,
                    separator: separator.take(),
                    sub: item.submenu.is_some(),
                    mnemonic: item
                        .mnemonic
                        .and_then(|at| item.label.chars().nth(at))
                        .map(|c| c.to_ascii_lowercase()),
                    underline: item.mnemonic,
                }),
            }
        }
        Some(LevelSpec {
            rows,
            title: None,
            speed_search: false,
            auto_select: false,
            mnemonics: true,
            default_index: None,
        })
    }

    fn choose(&mut self, path: &[usize], row: usize, _final_choice: bool) -> Chosen {
        let Some(entries) = self.at(path) else {
            return Chosen::Stay;
        };
        let Some(item) = items(entries).into_iter().nth(row) else {
            return Chosen::Stay;
        };
        if !item.enabled {
            return Chosen::Stay;
        }
        if item.submenu.is_some() {
            return Chosen::Opened;
        }
        self.chosen = Some(item.id.clone());
        Chosen::Final
    }
}

/// A popup menu of model entries: a context menu, a header overflow, a tab list.
pub type MenuPopup = PopupHost<MenuSource>;

/// Builds a [`MenuPopup`] for `entries` at `placement`, in the GUI's popup look.
pub fn menu_popup(entries: Vec<MenuEntry>, notation: Notation, placement: Placement) -> MenuPopup {
    PopupHost::from_parts(
        MenuSource::new(entries, notation),
        placement,
        PopupOptions {
            show_submenu_on_hover: true,
            cancel_on_click_outside: true,
            style: PopupStyle::Island,
        },
    )
}

impl PopupHost<MenuSource> {
    /// The id of the chosen row.
    pub fn take_choice(&mut self) -> Option<String> {
        self.source.take_chosen()
    }
}

/// The titles of a [`MenuBar`] and the menu that is open.
pub struct MenuBarControl {
    /// The identity; titles use child ids.
    pub id: Id,
    open: Option<usize>,
    expanded: bool,
    popup: Option<MenuPopup>,
    placed: Vec<(Id, usize, Rect)>,
    headers: Vec<Placed>,
    last_props: MenuBar,
    notation: Notation,
}

impl MenuBarControl {
    /// A collapsed bar.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            open: None,
            expanded: false,
            popup: None,
            placed: Vec::new(),
            headers: Vec::new(),
            last_props: MenuBar::default(),
            notation: Notation::Text,
        }
    }

    /// Whether the titles are shown (the hamburger was clicked or `F10` pressed).
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Whether a menu is open.
    pub fn is_open(&self) -> bool {
        self.popup.as_ref().is_some_and(|p| p.is_open())
    }

    /// The index of the open header.
    pub fn open_header(&self) -> Option<usize> {
        self.open.filter(|_| self.is_open())
    }

    /// Shows the titles without opening a menu (the hamburger).
    pub fn expand(&mut self) {
        self.expanded = true;
    }

    /// Hides the titles and closes the menu.
    pub fn collapse(&mut self) {
        self.expanded = false;
        self.open = None;
        self.popup = None;
    }

    /// Opens the menu of header `index` (shows the titles first).
    pub fn open_menu(&mut self, rt: &Runtime, props: &MenuBar, index: usize, by_keyboard: bool) {
        let Some(header) = props.headers.get(index) else {
            return;
        };
        self.expanded = true;
        self.open = Some(index);
        let notation = chrome::notation(rt);
        self.notation = notation;
        let anchor = self
            .placed
            .iter()
            .find(|(_, i, _)| *i == index)
            .map(|(_, _, r)| *r)
            .unwrap_or_default();
        let mut popup = menu_popup(header.entries.clone(), notation, Placement::Below(anchor));
        popup.chain.forget_close();
        popup.open(rt, by_keyboard);
        self.popup = Some(popup);
    }

    /// Opens the menu whose mnemonic is `letter` (`Alt+letter`); true when there is one.
    pub fn open_by_mnemonic(&mut self, rt: &Runtime, props: &MenuBar, letter: char) -> bool {
        let letter = letter.to_ascii_lowercase();
        match props
            .headers
            .iter()
            .position(|h| h.mnemonic_char() == Some(letter))
        {
            Some(index) => {
                self.open_menu(rt, props, index, true);
                true
            }
            None => false,
        }
    }

    /// Where header `index` was drawn.
    pub fn header_rect(&self, index: usize) -> Option<Rect> {
        self.placed
            .iter()
            .find(|(_, i, _)| *i == index)
            .map(|(_, _, r)| *r)
    }

    /// The outline of the titles as drawn.
    pub fn outline(&self, buf: &Buffer) -> Outline {
        chrome::outline_of(&self.headers, buf, |_| true)
    }

    /// Draws the titles from column `x` in `row` and, when a menu is open, its popup.
    /// Returns the column after the last title.
    pub fn draw(
        &mut self,
        rt: &mut Runtime,
        props: &MenuBar,
        row: Rect,
        x: u16,
        buf: &mut Buffer,
    ) -> u16 {
        self.last_props = props.clone();
        self.placed.clear();
        self.headers.clear();
        let look = rt.look.clone();
        let c = &look.theme.colors;
        let t = Tokens::new(&look, c.frame);
        let mut cx = x;
        for (index, header) in props.headers.iter().enumerate() {
            let w = text_width(&header.label) + 2;
            let rect = Rect::new(cx, row.y, w, 1).intersection(row);
            if rect.width < w {
                break;
            }
            let id = chrome::child(self.id, &header.id);
            let open = self.open_header() == Some(index);
            let hovered = rt.hovered() == Some(id);
            let bg = if open || hovered {
                t.of(&look, c.hover)
            } else {
                t.bg
            };
            fill(
                buf,
                rect,
                rect.x,
                rect.y,
                rect.width,
                Style::default().bg(bg),
            );
            let style = Style::default().fg(t.of(&look, c.text)).bg(bg);
            chrome::put_mnemonic(
                buf,
                rect,
                rect.x + 1,
                rect.y,
                &header.label,
                header.mnemonic,
                style,
            );
            rt.clickable(id, rect);
            self.placed.push((id, index, rect));
            let mut entry = OutlineEntry::new("menu", header.id.clone(), header.label.clone());
            entry.expanded = Some(open);
            self.headers.push(Placed {
                key: header.id.clone(),
                id,
                rect: Rect::new(rect.x + 1, rect.y, text_width(&header.label), 1),
                entry,
            });
            cx += w;
        }
        cx
    }

    /// Draws the open popup above everything; call after the rest of the screen.
    pub fn draw_popup(&mut self, rt: &mut Runtime, buf: &mut Buffer) {
        if let Some(popup) = &mut self.popup {
            popup.draw(rt, buf);
        }
    }

    /// The next time the popup needs a tick (submenu delay).
    pub fn deadline(&self) -> Option<f64> {
        self.popup.as_ref().and_then(|p| p.deadline())
    }

    /// Runs the popup's timers.
    pub fn tick(&mut self, rt: &Runtime) {
        if let Some(popup) = &mut self.popup {
            popup.tick(rt);
        }
    }

    /// Applies `event`. While a menu is open it takes every key and mouse event.
    pub fn handle(&mut self, rt: &mut Runtime, props: &MenuBar, event: &Event) -> Vec<MenuEvent> {
        let mut out = Vec::new();
        // Hover switching and clicks on the titles.
        if let Event::Mouse(_) = event
            && let Some(p) = rt.pointer().copied()
        {
            // Hit test by rectangle: an open menu blocks the hit map below it.
            let at = ratatui::layout::Position::new(p.x, p.y);
            let over = self
                .placed
                .iter()
                .find(|(_, _, rect)| rect.contains(at))
                .map(|(_, i, _)| *i);
            if let Some(index) = over {
                let hover = matches!(p.kind, PointerKind::Move | PointerKind::Drag(_));
                let press = p.kind == PointerKind::Down(crate::input::MouseButton::Left);
                if press {
                    if self.open_header() == Some(index) {
                        // A click on the open title closes its menu.
                        self.popup = None;
                        self.open = None;
                    } else {
                        self.open_menu(rt, props, index, false);
                    }
                    return out;
                }
                if hover && self.is_open() && self.open_header() != Some(index) {
                    self.open_menu(rt, props, index, false);
                    return out;
                }
            }
        }
        if !self.is_open() {
            if let Event::Key(key) = event
                && self.expanded
                && key.kind != KeyEventKind::Release
                && key.code == KeyCode::Esc
            {
                self.collapse();
                out.push(MenuEvent::Closed);
            }
            return out;
        }
        // Left and Right move between the titles unless a submenu is involved.
        if let Event::Key(key) = event
            && key.kind != KeyEventKind::Release
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            && let Some(index) = self.open
        {
            let count = props.headers.len();
            let popup = self.popup.as_ref();
            let depth = popup.map_or(0, |p| p.chain.depth());
            let selected_sub = popup.and_then(|p| {
                let row = p.chain.selected(depth.saturating_sub(1))?;
                let level_path: Vec<usize> = (0..depth.saturating_sub(1))
                    .filter_map(|d| p.chain.open_row(d))
                    .collect();
                let entries = p.source.entries_at(&level_path)?;
                Some(items(entries).get(row)?.submenu.is_some())
            });
            let target = match key.code {
                KeyCode::Left if depth <= 1 && count > 0 => Some((index + count - 1) % count),
                KeyCode::Right if depth <= 1 && selected_sub != Some(true) && count > 0 => {
                    Some((index + 1) % count)
                }
                _ => None,
            };
            if let Some(target) = target {
                self.open_menu(rt, props, target, true);
                return out;
            }
        }
        if let Some(popup) = &mut self.popup {
            match popup.event(rt, event) {
                PopupState::Chosen => {
                    if let Some(id) = popup.take_choice() {
                        out.push(MenuEvent::Chosen { id });
                    }
                    self.popup = None;
                    self.open = None;
                    // A choice closes all menus and collapses the bar.
                    self.expanded = false;
                }
                PopupState::Cancelled => {
                    self.popup = None;
                    self.open = None;
                    if matches!(event, Event::Mouse(_)) {
                        // A press outside collapses the bar, unless it hit a title.
                        let on_title = rt.pointer().is_some_and(|p| {
                            let at = ratatui::layout::Position::new(p.x, p.y);
                            self.placed.iter().any(|(_, _, rect)| rect.contains(at))
                        });
                        if !on_title {
                            self.expanded = false;
                            out.push(MenuEvent::Closed);
                        }
                    }
                }
                PopupState::Open => {}
            }
        }
        out
    }

    /// The notation the open popup uses.
    pub fn notation(&self) -> Notation {
        self.notation
    }
}

impl MenuSource {
    fn entries_at(&self, path: &[usize]) -> Option<&[MenuEntry]> {
        self.at(path)
    }
}

#[allow(dead_code)]
fn _unused(_: Modifier) {
    let _ = put;
}
