//! The runtime: the [`App`] trait, the event loop ([`run`]), and the [`Runtime`] that carries
//! what every component needs: the look, the keymap, the clock, the hit map, focus and
//! clipboard.
//!
//! The loop is the usual one for an immediate mode UI: wait for input (but no longer than
//! the app's next [deadline](App::deadline), so timers such as the tooltip delay fire on
//! time), update the clock, tell the app, draw. Time is a plain `f64` of seconds in
//! [`Runtime::now`]; the delays are the GUI kit's ([`archipel_kit::tokens::motion`]).
//! Tests drive the same code with synthetic events and a manual clock
//! ([`crate::testing::Harness`]).

use crate::Look;
use crate::clipboard::Clipboard;
use crate::focus::FocusManager;
use crate::hit::{HitMap, Id};
use crate::input::{self, Caps, DoubleShift, Event, KeyEvent};
use crate::pointer::{Pointer, PointerTracker};
use crate::tooltip::TooltipState;
use archipel_kit::action_system::{KeyResolution, Keymap, PendingStroke, SECOND_KEYSTROKE_TIMEOUT};
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::io::{self, Write};
use std::time::{Duration, Instant};

/// A terminal application.
pub trait App {
    /// Handles one input event (after [`Runtime::preprocess`]: the pointer and the key
    /// resolution are available on `rt`).
    fn event(&mut self, rt: &mut Runtime, event: &Event);

    /// Draws the whole screen into `buf`. Register hit rectangles and focus ids through
    /// `rt` while drawing.
    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer);

    /// Called every loop turn after the clock advanced: run timers here.
    fn tick(&mut self, rt: &mut Runtime) {
        let _ = rt;
    }

    /// The time (in [`Runtime::now`] seconds) the app next needs a [`tick`](App::tick),
    /// if any. The loop wakes up then even without input.
    fn deadline(&self, rt: &Runtime) -> Option<f64> {
        let _ = rt;
        None
    }
}

/// The shape of the terminal cursor ([`Runtime::cursor`]), sent with DECSCUSR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CursorShape {
    /// A vertical bar: insert mode.
    Bar,
    /// A block: overwrite mode.
    Block,
    /// An underline.
    Underline,
}

/// Where the terminal cursor goes this frame, and its shape. A component that edits text
/// sets it while drawing ([`Runtime::set_cursor`]); the last one wins. The loop shows the
/// real terminal cursor there, and hides it when nothing set it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorRequest {
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// Shape.
    pub shape: CursorShape,
}

/// What every component needs to draw and to handle input.
pub struct Runtime {
    /// The theme as terminal colors.
    pub look: Look,
    /// The keymap shared with the GUI, plus the terminal fallbacks
    /// ([`input::terminal_keymap`]).
    pub keymap: Keymap,
    /// What the terminal can send.
    pub caps: Caps,
    /// The clock, in seconds. Advanced by the loop (or the test harness).
    pub now: f64,
    /// This frame's hit map.
    pub hits: HitMap,
    /// Keyboard focus.
    pub focus: FocusManager,
    /// The clipboard.
    pub clipboard: Clipboard,
    /// Tooltips (show after 500 ms of hover or focus).
    pub tooltip: TooltipState,
    /// The description of the hovered action, for the status bar (set while drawing).
    pub hint: Option<String>,
    /// Set to true to leave the loop.
    pub quit: bool,
    /// The screen area of the last frame.
    pub screen: Rect,
    /// Whether the terminal window has focus (notification timers pause when it has not).
    pub window_focused: bool,
    /// Where the terminal cursor goes this frame (set while drawing, cleared by
    /// [`begin_frame`](Self::begin_frame)).
    pub cursor: Option<CursorRequest>,
    /// The window title (OSC 2): the loop sends it when it changes.
    pub title: Option<String>,
    /// Whether the terminal draws curly underlines (SGR 4:3, kitty, WezTerm, Ghostty, Alacritty,
    /// foot, iTerm2, Windows Terminal, VTE). Cells with an underline color are then drawn as
    /// undercurl after each frame; without it they keep the plain underline. Detected by
    /// [`detect_undercurl`]; false in tests.
    pub undercurl: bool,
    tracker: PointerTracker,
    pointer: Option<Pointer>,
    pointer_pos: Option<(u16, u16)>,
    hover_id: Option<Id>,
    pending: Option<PendingStroke>,
    resolution: Option<KeyResolution>,
    chord_second: bool,
    double_shift: DoubleShift,
    double_shift_fired: bool,
}

impl Runtime {
    /// A runtime for `look` with the terminal keymap for the current operating system.
    pub fn new(look: Look) -> Self {
        let caps = Caps::legacy();
        Self::with(look, input::terminal_keymap(caps.os), caps)
    }

    /// A runtime with an explicit keymap and capabilities.
    pub fn with(look: Look, keymap: Keymap, caps: Caps) -> Self {
        Self {
            look,
            keymap,
            caps,
            now: 0.0,
            hits: HitMap::new(),
            focus: FocusManager::new(),
            clipboard: Clipboard::new(),
            tooltip: TooltipState::default(),
            hint: None,
            quit: false,
            screen: Rect::default(),
            window_focused: true,
            cursor: None,
            title: None,
            undercurl: false,
            tracker: PointerTracker::default(),
            pointer: None,
            pointer_pos: None,
            hover_id: None,
            pending: None,
            resolution: None,
            chord_second: false,
            double_shift: DoubleShift::default(),
            double_shift_fired: false,
        }
    }

    /// Starts a frame of `area`: forgets the hit map, the focus order and the tooltip offers.
    pub fn begin_frame(&mut self, area: Rect) {
        self.screen = area;
        self.hits.clear();
        self.focus.begin_frame();
        self.tooltip.begin_frame();
        self.hint = None;
        self.cursor = None;
    }

    /// Asks for the terminal cursor at (`x`, `y`) in `shape`.
    pub fn set_cursor(&mut self, x: u16, y: u16, shape: CursorShape) {
        self.cursor = Some(CursorRequest { x, y, shape });
    }

    /// Ends a frame.
    pub fn end_frame(&mut self) {
        self.focus.end_frame();
        self.hover_id = self.pointer_pos.and_then(|(x, y)| self.hits.hit(x, y));
    }

    /// Registers a focusable, clickable widget: its `rect` in the hit map and `id` in the
    /// focus order.
    pub fn widget(&mut self, id: Id, rect: Rect) {
        self.hits.add(rect, id);
        self.focus.register(id);
    }

    /// Registers a clickable but not focusable widget (a toolbar button, a tab).
    pub fn clickable(&mut self, id: Id, rect: Rect) {
        self.hits.add(rect, id);
    }

    /// Whether `id` has keyboard focus.
    pub fn focused(&self, id: Id) -> bool {
        self.focus.is_focused(id)
    }

    /// Prepares `event` before the app sees it: resolves the keymap (two-stroke chords
    /// included), the pointer, window focus, double-Shift, paste and resize.
    pub fn preprocess(&mut self, event: &Event) {
        self.pointer = None;
        self.resolution = None;
        self.chord_second = false;
        self.double_shift_fired = self.double_shift.feed(event, self.now);
        match event {
            Event::Key(key) => {
                self.tooltip.dismiss();
                self.resolve(key);
            }
            Event::Mouse(mouse) => {
                let pointer = self.tracker.update(mouse, &self.hits, self.now);
                if matches!(pointer.kind, crate::pointer::PointerKind::Down(_)) {
                    self.tooltip.dismiss();
                }
                // Focus follows a press on a focusable widget.
                if pointer.kind
                    == crate::pointer::PointerKind::Down(crate::input::MouseButton::Left)
                    && let Some(id) = pointer.target
                    && self.focus.contains(id)
                {
                    self.focus.focus(id);
                }
                self.pointer_pos = Some((pointer.x, pointer.y));
                self.hover_id = pointer.target;
                self.tooltip
                    .update(self.now, pointer.target, self.focus.current());
                self.pointer = Some(pointer);
            }
            Event::Resize(w, h) => self.screen = Rect::new(0, 0, *w, *h),
            Event::Paste(text) => self.clipboard.remember(text.clone()),
            Event::FocusGained => self.window_focused = true,
            Event::FocusLost => {
                self.window_focused = false;
                self.pending = None;
            }
            Event::KeyUp(_) => {}
        }
    }

    fn resolve(&mut self, key: &KeyEvent) {
        if let Some(stroke) = input::key_stroke(key) {
            let fresh = self
                .pending
                .filter(|p| self.now - p.at <= SECOND_KEYSTROKE_TIMEOUT.as_secs_f64());
            let resolution = self.keymap.feed(&mut self.pending, stroke, self.now);
            self.chord_second = match (&fresh, &resolution) {
                (Some(first), KeyResolution::Matched(ids)) => {
                    *ids == self.keymap.action_ids_for(&first.first, Some(&stroke))
                }
                _ => false,
            };
            self.resolution = Some(resolution);
        }
    }

    /// Whether the current key event is the second stroke of a chord that matched: its
    /// [`actions`](Self::actions) belong to the chord, and the key must not also act as the
    /// plain key it is (the `Up` of `Ctrl+K, Up`).
    pub fn chord_completed(&self) -> bool {
        self.chord_second
    }

    /// The pointer of the current mouse event.
    pub fn pointer(&self) -> Option<&Pointer> {
        self.pointer.as_ref()
    }

    /// Where the pointer last was.
    pub fn pointer_pos(&self) -> Option<(u16, u16)> {
        self.pointer_pos
    }

    /// The widget under the pointer (as of the last frame's hit map).
    pub fn hovered(&self) -> Option<Id> {
        self.hover_id
    }

    /// The ids of the actions the current key event triggers in the keymap.
    pub fn actions(&self) -> &[String] {
        match &self.resolution {
            Some(KeyResolution::Matched(ids)) => ids,
            _ => &[],
        }
    }

    /// Whether the current key event triggers the action `id`.
    pub fn is_action(&self, id: &str) -> bool {
        self.actions().iter().any(|a| a == id)
    }

    /// Whether the current key event is the first stroke of a chord: show the key hints.
    pub fn awaiting_second_stroke(&self) -> bool {
        self.pending.is_some()
    }

    /// The first stroke of the chord in progress.
    pub fn pending_stroke(&self) -> Option<&PendingStroke> {
        self.pending.as_ref()
    }

    /// Whether the second stroke of a chord is still awaited and has not timed out.
    pub fn pending_active(&self) -> bool {
        self.pending
            .is_some_and(|p| self.now - p.at <= SECOND_KEYSTROKE_TIMEOUT.as_secs_f64())
    }

    /// Whether the current event completed a double tap of Shift.
    pub fn double_shift(&self) -> bool {
        self.double_shift_fired
    }

    /// Advances the timers that live on the runtime: forgets a chord whose second stroke
    /// took too long.
    pub fn tick(&mut self) {
        if self.pending.is_some() && !self.pending_active() {
            self.pending = None;
        }
        let (hovered, focused) = (self.hovered(), self.focus.current());
        self.tooltip.update(self.now, hovered, focused);
    }

    /// The text of the shortcut of `action_id` that this terminal can send.
    pub fn shortcut_text(&self, action_id: &str) -> Option<String> {
        input::display_shortcut(&self.keymap, action_id, &self.caps)
    }
}

/// The OSC 2 sequence that sets the window title to `title` (control characters dropped).
pub fn title_sequence(title: &str) -> String {
    let clean: String = title.chars().filter(|c| !c.is_control()).collect();
    format!("\x1b]2;{clean}\x07")
}

/// Options of [`run`].
#[derive(Debug, Clone, Copy)]
pub struct RunOptions {
    /// The longest the loop waits for input when the app has no deadline.
    pub idle_wait: Duration,
    /// Ask the terminal to report modifier keys on their own (needed for double-Shift); the
    /// kitty protocol only.
    pub report_modifier_keys: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            idle_wait: Duration::from_millis(250),
            report_modifier_keys: false,
        }
    }
}

/// The type of the terminal [`setup`] creates.
pub type CrosstermTerminal = Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>;

/// Puts the terminal in app mode: raw mode, the alternate screen, mouse capture (clicks,
/// drags, wheel and hover), bracketed paste, focus reporting, and the kitty keyboard
/// protocol when the terminal has it. Call [`restore`] when done (or use [`run_app`]).
///
/// # Errors
///
/// When the terminal cannot be configured.
pub fn setup(options: RunOptions) -> io::Result<(CrosstermTerminal, Caps)> {
    use ratatui::crossterm::event::{
        EnableBracketedPaste, EnableFocusChange, EnableMouseCapture, KeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    };
    use ratatui::crossterm::execute;
    use ratatui::crossterm::terminal::supports_keyboard_enhancement;
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    let terminal = ratatui::init();
    let mut out = io::stdout();
    // Save the window title; `restore` puts it back after the app changed it (OSC 2).
    let _ = out.write_all(b"\x1b[22;2t");
    execute!(
        out,
        EnableMouseCapture,
        EnableBracketedPaste,
        EnableFocusChange
    )?;
    if kitty {
        let mut flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS;
        if options.report_modifier_keys {
            flags |= KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        }
        execute!(out, PushKeyboardEnhancementFlags(flags))?;
    }
    // The panic hook of `ratatui::init` restores raw mode and the screen; add the rest.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_modes();
        previous(info);
    }));
    let caps = Caps {
        kitty,
        os: archipel_kit::action_system::Os::current(),
        truecolor: detect_truecolor(),
        super_reported: false,
        curly: detect_undercurl(),
    };
    Ok((terminal, caps))
}

fn restore_modes() {
    use ratatui::crossterm::cursor::{SetCursorStyle, Show};
    use ratatui::crossterm::event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, PopKeyboardEnhancementFlags,
    };
    use ratatui::crossterm::execute;
    let mut out = io::stdout();
    let _ = out.write_all(b"\x1b[23;2t");
    let _ = execute!(
        out,
        SetCursorStyle::DefaultUserShape,
        Show,
        PopKeyboardEnhancementFlags,
        DisableFocusChange,
        DisableBracketedPaste,
        DisableMouseCapture
    );
}

/// Undoes [`setup`].
pub fn restore() {
    restore_modes();
    ratatui::restore();
}

/// Sets the terminal up, runs `app` with a runtime for `look`, and restores the terminal,
/// also when `app` fails.
///
/// # Errors
///
/// When the terminal fails.
pub fn run_app(look: Look, app: &mut impl App, options: RunOptions) -> io::Result<()> {
    let (mut terminal, caps) = setup(options)?;
    let mut rt = Runtime::with(look, input::terminal_keymap(caps.os), caps);
    rt.undercurl = detect_undercurl();
    let result = run(&mut terminal, &mut rt, app, options);
    restore();
    result
}

/// Whether the terminal shows 24-bit colors: `COLORTERM` is `truecolor` or `24bit`
/// (`ARCHIPEL_TRUECOLOR=1` or `=0` overrides).
pub fn detect_truecolor() -> bool {
    if let Ok(value) = std::env::var("ARCHIPEL_TRUECOLOR") {
        return value != "0" && !value.is_empty();
    }
    std::env::var("COLORTERM")
        .map(|v| {
            let v = v.to_ascii_lowercase();
            v == "truecolor" || v == "24bit"
        })
        .unwrap_or(false)
}

/// Whether the terminal draws curly underlines (SGR `4:3` with the color in `58`): kitty,
/// WezTerm, Ghostty, Alacritty, foot, Contour, iTerm2, Windows Terminal and VTE 0.52 or newer
/// (GNOME Terminal). Judged from the environment; `ARCHIPEL_UNDERCURL=1` or `=0` overrides.
/// Inside tmux it is off unless overridden (tmux needs `terminal-overrides` for it).
pub fn detect_undercurl() -> bool {
    use std::env::var;
    if let Ok(value) = var("ARCHIPEL_UNDERCURL") {
        return value != "0" && !value.is_empty();
    }
    if var("TMUX").is_ok() {
        return false;
    }
    let term = var("TERM").unwrap_or_default().to_lowercase();
    let program = var("TERM_PROGRAM").unwrap_or_default().to_lowercase();
    let known = [
        "kitty",
        "wezterm",
        "ghostty",
        "alacritty",
        "foot",
        "contour",
    ];
    if known
        .iter()
        .any(|k| term.contains(k) || program.contains(k))
    {
        return true;
    }
    if program.contains("iterm") || var("WT_SESSION").is_ok() || var("KITTY_WINDOW_ID").is_ok() {
        return true;
    }
    var("VTE_VERSION")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .is_some_and(|v| v >= 5200)
}

/// The escape sequences that draw the cells of `buf` that have an underline color as
/// curly underlines: for each such cell, move there, set colors, `4:3` and the underline
/// color, and print the symbol again. The plain underline ratatui drew first stays as the
/// fallback for terminals without support.
pub fn undercurl_sequences(buf: &Buffer) -> String {
    use ratatui::style::{Color, Modifier};
    use std::fmt::Write as _;
    let mut out = String::new();
    let area = buf.area;
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &buf[(x, y)];
            if !cell.modifier.contains(Modifier::UNDERLINED) || cell.symbol().is_empty() {
                continue;
            }
            let Color::Rgb(ur, ug, ub) = cell.underline_color else {
                continue;
            };
            let _ = write!(out, "\x1b[{};{}H\x1b[0m", y + 1, x + 1);
            if let Color::Rgb(r, g, b) = cell.fg {
                let _ = write!(out, "\x1b[38;2;{r};{g};{b}m");
            }
            if let Color::Rgb(r, g, b) = cell.bg {
                let _ = write!(out, "\x1b[48;2;{r};{g};{b}m");
            }
            if cell.modifier.contains(Modifier::BOLD) {
                out.push_str("\x1b[1m");
            }
            if cell.modifier.contains(Modifier::ITALIC) {
                out.push_str("\x1b[3m");
            }
            let _ = write!(
                out,
                "\x1b[4:3m\x1b[58:2::{ur}:{ug}:{ub}m{}\x1b[0m",
                cell.symbol()
            );
        }
    }
    out
}

/// Like [`run_app`], with a keymap built for the detected operating system.
///
/// # Errors
///
/// When the terminal fails.
pub fn run_app_with_keymap(
    look: Look,
    app: &mut impl App,
    options: RunOptions,
    keymap: impl FnOnce(archipel_kit::action_system::Os) -> Keymap,
) -> io::Result<()> {
    let (mut terminal, caps) = setup(options)?;
    let mut rt = Runtime::with(look, keymap(caps.os), caps);
    rt.undercurl = detect_undercurl();
    let result = run(&mut terminal, &mut rt, app, options);
    restore();
    result
}

/// The event loop: wait for input or the app's next deadline, advance the clock, deliver
/// events, tick, draw, and publish clipboard copies as OSC 52, until `rt.quit` is set.
///
/// # Errors
///
/// When the terminal fails.
pub fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    rt: &mut Runtime,
    app: &mut impl App,
    options: RunOptions,
) -> io::Result<()>
where
    io::Error: From<B::Error>,
{
    use ratatui::crossterm::cursor::SetCursorStyle;
    use ratatui::crossterm::event as ct;
    use ratatui::crossterm::execute;
    let start = Instant::now();
    let mut shape: Option<CursorShape> = None;
    let mut shown_title: Option<String> = None;
    loop {
        rt.now = start.elapsed().as_secs_f64();
        rt.tick();
        app.tick(rt);
        let completed = terminal.draw(|frame| {
            let area = frame.area();
            rt.begin_frame(area);
            app.draw(rt, area, frame.buffer_mut());
            rt.end_frame();
            if !rt.caps.truecolor {
                crate::quantize::quantize_buffer(frame.buffer_mut());
            }
            if !rt.undercurl {
                // SGR 58 (the underline color) is not understood everywhere: xterm reads its
                // parameters as other attributes (a cyan background). Underlines take the
                // text color there.
                let buffer = frame.buffer_mut();
                let area = buffer.area;
                for y in area.top()..area.bottom() {
                    for x in area.left()..area.right() {
                        buffer[(x, y)].underline_color = ratatui::style::Color::Reset;
                    }
                }
            }
            if let Some(cursor) = rt.cursor {
                frame.set_cursor_position((cursor.x, cursor.y));
            }
        })?;
        let curls = rt.undercurl.then(|| undercurl_sequences(completed.buffer));
        let mut out = io::stdout();
        if let Some(curls) = curls.filter(|c| !c.is_empty()) {
            out.write_all(curls.as_bytes())?;
            // The cursor goes back where the frame put it.
            if let Some(cursor) = rt.cursor {
                write!(out, "\x1b[{};{}H", cursor.y + 1, cursor.x + 1)?;
            }
            out.flush()?;
        }
        let wanted = rt.cursor.map(|c| c.shape);
        if wanted != shape {
            match wanted {
                Some(CursorShape::Bar) => execute!(out, SetCursorStyle::SteadyBar)?,
                Some(CursorShape::Block) => execute!(out, SetCursorStyle::SteadyBlock)?,
                Some(CursorShape::Underline) => execute!(out, SetCursorStyle::SteadyUnderScore)?,
                None => {}
            }
            shape = wanted;
        }
        if let Some(sequence) = rt.clipboard.take_sequence() {
            out.write_all(sequence.as_bytes())?;
            out.flush()?;
        }
        if rt.title != shown_title {
            if let Some(title) = &rt.title {
                out.write_all(title_sequence(title).as_bytes())?;
                out.flush()?;
            }
            shown_title.clone_from(&rt.title);
        }
        if rt.quit {
            return Ok(());
        }
        let wait = match app.deadline(rt) {
            Some(at) => {
                Duration::from_secs_f64((at - rt.now).clamp(0.0, 3600.0)).min(options.idle_wait)
            }
            None => options.idle_wait,
        };
        if ct::poll(wait)? {
            // Deliver everything that is waiting, then draw once.
            loop {
                rt.now = start.elapsed().as_secs_f64();
                if let Some(event) = Event::from_crossterm_with(ct::read()?, &rt.caps) {
                    rt.preprocess(&event);
                    app.event(rt, &event);
                }
                if rt.quit || !ct::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
    }
}
