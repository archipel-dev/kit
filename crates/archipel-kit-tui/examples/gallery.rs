//! The TUI gallery: the interactive controls on one page.
//!
//! `cargo run -p archipel-kit-tui --example gallery`
//!
//! Tab and Shift+Tab move focus, Space and Enter press, arrows change a radio group or the
//! segmented button, `t` (outside a text field) switches the theme, Ctrl+K then Q quits. The
//! mouse works too. Use a Nerd Font in the terminal (JetBrainsMono Nerd Font).

#![allow(missing_docs)]

use archipel_kit::ui::Status;
use archipel_kit_tui::components::{Pane, PaneKind, ProgressBar};
use archipel_kit_tui::controls::{
    BannerControl, ButtonControl, ButtonState, CheckBoxControl, CheckState, FieldEvent,
    OnOffControl, RadioGroupControl, SegmentedControl, TextFieldControl, TextFieldState,
};
use archipel_kit_tui::input::{Event, KeyCode};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::{Glyph, Id, Look, Runtime, Theme, runtime};

const OK: Id = Id::new("g.ok");
const CHECK: Id = Id::new("g.check");
const RADIO: Id = Id::new("g.radio");
const SWITCH: Id = Id::new("g.switch");
const SEG: Id = Id::new("g.seg");
const NAME: Id = Id::new("g.name");
const BANNER: Id = Id::new("g.banner");

struct Gallery {
    button: ButtonState,
    clicks: u32,
    check: CheckState,
    radio: usize,
    switch: bool,
    seg: usize,
    name: TextFieldState,
    theme: usize,
}

const RADIOS: [&str; 3] = ["Spaces", "Tabs", "Mixed"];
const SEGS: [&str; 3] = ["Tree", "List", "Graph"];

impl archipel_kit_tui::App for Gallery {
    fn event(&mut self, rt: &mut Runtime, event: &Event) {
        if ButtonControl::new(OK, "Apply")
            .primary()
            .handle(rt, &mut self.button, event)
        {
            self.clicks += 1;
        }
        CheckBoxControl::new(CHECK, "Format on save").handle(rt, &mut self.check, event);
        RadioGroupControl::new(RADIO, &RADIOS).handle(rt, &mut self.radio, event);
        if OnOffControl::new(SWITCH, "Soft wrap").handle(rt, event) {
            self.switch = !self.switch;
        }
        SegmentedControl::new(SEG, &SEGS).handle(rt, &mut self.seg, event);
        let r = TextFieldControl::new(NAME)
            .label("Name")
            .handle(rt, &mut self.name, event);
        if let Event::Key(k) = event {
            match k.code {
                KeyCode::Tab => rt.focus.next(),
                KeyCode::BackTab => rt.focus.previous(),
                KeyCode::Char('t') if r == FieldEvent::Ignored && !rt.focused(NAME) => {
                    self.theme = (self.theme + 1) % Theme::all().len();
                    rt.look = Look::new((Theme::all()[self.theme].constructor)());
                }
                _ => {}
            }
        }
        if rt.is_action("Exit") {
            rt.quit = true;
        }
        BannerControl::new(BANNER, Status::Info, "Press Tab to move focus", &["Got it"])
            .handle(rt, event);
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        Pane::new(&rt.look).kind(PaneKind::Editor).render(area, buf);
        if area.height < 18 || area.width < 50 {
            return;
        }
        ButtonControl::new(OK, "Apply")
            .primary()
            .icon(Glyph::Success)
            .show(rt, &self.button, 2, 1, buf);
        CheckBoxControl::new(CHECK, "Format on save").show(rt, self.check, 2, 3, buf);
        RadioGroupControl::new(RADIO, &RADIOS).show(rt, self.radio, 2, 5, buf);
        OnOffControl::new(SWITCH, "Soft wrap").show(rt, self.switch, 2, 9, buf);
        SegmentedControl::new(SEG, &SEGS).show(rt, self.seg, 2, 11, buf);
        TextFieldControl::new(NAME)
            .label("Name")
            .placeholder("project name")
            .show(rt, &self.name, Rect::new(2, 13, 40, 1), buf);
        ProgressBar::new(&rt.look, 0.6).render(Rect::new(2, 15, 40, 1), buf);
        BannerControl::new(BANNER, Status::Info, "Press Tab to move focus", &["Got it"]).show(
            rt,
            Rect::new(0, area.bottom() - 1, area.width, 1),
            buf,
        );
    }
}

fn main() -> std::io::Result<()> {
    let mut app = Gallery {
        button: ButtonState::default(),
        clicks: 0,
        check: CheckState::Checked,
        radio: 0,
        switch: false,
        seg: 0,
        name: TextFieldState::new(),
        theme: 0,
    };
    let look = Look::new((Theme::all()[0].constructor)());
    runtime::run_app_with_keymap(look, &mut app, runtime::RunOptions::default(), |os| {
        archipel_kit_tui::input::terminal_keymap(os)
    })
}
