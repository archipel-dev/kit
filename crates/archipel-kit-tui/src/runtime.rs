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
    /// Set to true to leave the loop.
    pub quit: bool,
    /// The screen area of the last frame.
    pub screen: Rect,
    /// Whether the terminal window has focus (notification timers pause when it has not).
    pub window_focused: bool,
    tracker: PointerTracker,
    pointer: Option<Pointer>,
    pointer_pos: Option<(u16, u16)>,
    pending: Option<PendingStroke>,
    resolution: Option<KeyResolution>,
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
            quit: false,
            screen: Rect::default(),
            window_focused: true,
            tracker: PointerTracker::default(),
            pointer: None,
            pointer_pos: None,
            pending: None,
            resolution: None,
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
    }

    /// Ends a frame.
    pub fn end_frame(&mut self) {
        self.focus.end_frame();
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
            self.resolution = Some(self.keymap.feed(&mut self.pending, stroke, self.now));
        }
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
        let (x, y) = self.pointer_pos?;
        self.hits.hit(x, y)
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
    };
    Ok((terminal, caps))
}

fn restore_modes() {
    use ratatui::crossterm::event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, PopKeyboardEnhancementFlags,
    };
    use ratatui::crossterm::execute;
    let mut out = io::stdout();
    let _ = execute!(
        out,
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
    use ratatui::crossterm::event as ct;
    let start = Instant::now();
    loop {
        rt.now = start.elapsed().as_secs_f64();
        rt.tick();
        app.tick(rt);
        terminal.draw(|frame| {
            let area = frame.area();
            rt.begin_frame(area);
            app.draw(rt, area, frame.buffer_mut());
            rt.end_frame();
        })?;
        if let Some(sequence) = rt.clipboard.take_sequence() {
            let mut out = io::stdout();
            out.write_all(sequence.as_bytes())?;
            out.flush()?;
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
                if let Some(event) = Event::from_crossterm(ct::read()?) {
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
