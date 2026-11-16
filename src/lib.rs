//! Scrolling a [Ratatui] app with a wheel, a trackpad or a finger, a cell at
//! a time.
//!
//! Each gesture only goes one way, up and down, or sideways, so a swipe
//! that's a little diagonal doesn't do both. Hosts that report movement in
//! pixels, like a browser, move the view as far as the hand does, a line or
//! a column at a time once one's worth adds up, and a flick keeps the view
//! gliding for a bit, slowing down, like a page in the browser does.
//!
//! What scrolls is a [`Scroll`]: a widget's state, given a [`Step`] of whole
//! cells and the area the widget is drawn in. It says whether it moved,
//! which ends the gesture at the end of what there is to scroll.
//!
//! - Ratatui's `ListState` and `TableState` are `Scroll`s.
//! - A widget of the app's own implements `Scroll` for its state, out of a
//!   [`Step::scroll`] for each way it goes.
//! - An app with several of them is a `Scroll` too, which splits its area
//!   as it does to draw, and hands each part to the state that's drawn
//!   there.
//!
//! Each host scrolls one from its own events:
//!
//! - In a terminal, [`terminal::Wheel`] takes the wheel's ticks, like
//!   crossterm's `MouseEventKind::ScrollDown`.
//! - In a browser, the `web` feature's `web::listen` follows the wheel and
//!   touch events over the element a view is drawn in, like [Ratzilla]'s.
//! - Anywhere else that reports movement in pixels, [`Gestures`] does the
//!   arithmetic, from pixels and timestamps. Call its methods from the
//!   host's events, and [`Gestures::glide`] once a frame after
//!   [`Gestures::touch_end`] says a touch was a flick.
//!
//! ```
//! use ratatui::layout::Rect;
//! use ratatui::widgets::ListState;
//! use ratjest::{Gestures, Scroll};
//!
//! // A list, and where its items are drawn.
//! let mut state = ListState::default();
//! let area = Rect::new(0, 0, 30, 5);
//!
//! // Cells 8 pixels wide and 16 tall.
//! let mut gestures = Gestures::new(8.0, 16.0);
//!
//! // Half a line doesn't move yet, the rest of a line and a half does.
//! gestures.wheel((0.0, 0.0), (0.0, 8.0), 0.0, &mut state, area);
//! gestures.wheel((0.0, 0.0), (0.0, 24.0), 10.0, &mut state, area);
//! assert_eq!(state.offset(), 2);
//! ```
//!
//! [Ratatui]: https://ratatui.rs
//! [Ratzilla]: https://github.com/ratatui/ratzilla

#![warn(missing_docs)]

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub mod terminal;
#[cfg(feature = "web")]
pub mod web;
mod widgets;

use std::cell::RefCell;
use std::rc::Rc;

use ratatui_core::layout::{Position, Rect};

/// Slowest a flick glides, in pixels a millisecond.
const MIN_GLIDE: f64 = 0.05;

/// How far a finger goes before its touch picks a way to move, in pixels.
const LOCK_DISTANCE: f64 = 6.0;

/// How long between wheel events before the next starts a new gesture, in
/// milliseconds.
const WHEEL_GESTURE: f64 = 200.0;

/// How long a glide's first frame is taken to be, in milliseconds.
const FRAME: f64 = 16.0;

/// Something a gesture scrolls, a [`Step`] at a time.
///
/// It's a widget's state, the part that keeps how far the widget is
/// scrolled. Like when it's drawn, it's told the area the widget is in,
/// which says whether the step is over it, and how much fits.
///
/// It's implemented for Ratatui's `ListState` and `TableState`, which
/// scroll the way they do for the keys: by the selection when there is one,
/// and by the offset when there isn't. They don't know how many rows they
/// have, so a step can go past the last one, and the widget brings it back
/// when it's next drawn.
///
/// A widget of the app's own implements it for its state, out of a
/// [`Step::scroll`] for each way it goes. One that knows how much it has,
/// like this, stops at its end. An app with several widgets is one too,
/// which splits `area` as it does to draw, and tries each widget's state in
/// turn with the part it's drawn in.
///
/// ```
/// use ratatui::layout::Rect;
/// use ratjest::{Axis, Scroll, Step};
///
/// /// How far some lines of text are scrolled.
/// struct Lines {
///     top: usize,
///     len: usize,
/// }
///
/// impl Scroll for Lines {
///     fn scroll(&mut self, step: Step, area: Rect) -> bool {
///         let last = self.len.saturating_sub(usize::from(area.height));
///         step.is_over(area) && step.scroll(Axis::Vertical, &mut self.top, last)
///     }
/// }
///
/// let area = Rect::new(0, 0, 30, 5);
/// let mut lines = Lines { top: 0, len: 20 };
///
/// // A step from a wheel three lines down, over the lines.
/// let step = Step { axis: Axis::Vertical, cells: 3, column: 4, row: 2 };
/// assert!(lines.scroll(step, area));
/// assert_eq!(lines.top, 3);
/// ```
pub trait Scroll {
    /// Carries out `step` on what's drawn in `area`, and returns whether it
    /// moved. It doesn't when the step isn't over `area`, goes a way it
    /// doesn't scroll, or it's at the end.
    fn scroll(&mut self, step: Step, area: Rect) -> bool;
}

impl<S: Scroll + ?Sized> Scroll for &mut S {
    fn scroll(&mut self, step: Step, area: Rect) -> bool {
        (**self).scroll(step, area)
    }
}

/// A state shared between what draws it and what scrolls it, as `web::listen`
/// needs, since it keeps what it scrolls for as long as the page lives.
impl<S: Scroll + ?Sized> Scroll for Rc<RefCell<S>> {
    fn scroll(&mut self, step: Step, area: Rect) -> bool {
        self.borrow_mut().scroll(step, area)
    }
}

/// The way a gesture moves the view. A gesture only ever moves one way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Up and down.
    Vertical,
    /// Left and right.
    Horizontal,
}

impl Axis {
    /// The way a move of `x`, `y` pixels mostly goes.
    fn of(x: f64, y: f64) -> Self {
        if x.abs() > y.abs() {
            Axis::Horizontal
        } else {
            Axis::Vertical
        }
    }

    /// The part of a move of `x`, `y` pixels that goes this way.
    fn part(self, x: f64, y: f64) -> f64 {
        match self {
            Axis::Vertical => y,
            Axis::Horizontal => x,
        }
    }
}

/// A gesture's move of whole cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// Which way it goes.
    pub axis: Axis,
    /// How many lines down, or columns right, it goes. Negative goes up, or
    /// left.
    pub cells: isize,
    /// The column of the cell the gesture is over: the pointer for a wheel,
    /// or where the finger came down for a touch.
    pub column: u16,
    /// The row of the cell the gesture is over.
    pub row: u16,
}

impl Step {
    /// Whether the gesture is over `area`, which says whether it's this
    /// part of the screen that scrolls.
    pub fn is_over(self, area: Rect) -> bool {
        area.contains(Position::new(self.column, self.row))
    }

    /// Scrolls `offset` by this step's cells, when it goes `axis`'s way,
    /// keeping it from 0 through `last`. Returns whether it moved, which it
    /// doesn't the other way, or at either end.
    ///
    /// An offset is how far something is scrolled: the line at the top of
    /// some text, the first row drawn of a grid, the column a wide line
    /// starts from. This is what a [`Scroll`] is made of, once for each way
    /// it scrolls:
    ///
    /// ```
    /// use ratjest::{Axis, Step};
    ///
    /// // Twenty lines in a view five tall, so the top line is 0 through 15.
    /// let mut top = 14;
    ///
    /// let step = Step { axis: Axis::Vertical, cells: 3, column: 4, row: 2 };
    /// assert!(step.scroll(Axis::Vertical, &mut top, 15));
    /// assert_eq!(top, 15);
    /// // At the end, and not sideways.
    /// assert!(!step.scroll(Axis::Vertical, &mut top, 15));
    /// assert!(!step.scroll(Axis::Horizontal, &mut top, 15));
    /// ```
    pub fn scroll(self, axis: Axis, offset: &mut usize, last: usize) -> bool {
        if self.axis != axis {
            return false;
        }
        let before = *offset;
        *offset = before.saturating_add_signed(self.cells).min(last);
        *offset != before
    }
}

/// Turns wheel and touch movement, in pixels, into [`Step`]s of whole
/// cells for a [`Scroll`], keeping the part of a cell left over for the
/// gesture's next event.
///
/// Positions are in pixels from the view's top left corner, and times in
/// milliseconds from any fixed point, like a browser event's `timeStamp`.
/// Each method that moves takes what to scroll, and the area it's drawn in,
/// in cells, which for a whole app is the frame's.
#[derive(Debug, Clone)]
pub struct Gestures {
    /// How wide and tall the view's cells are, in pixels.
    cell: (f64, f64),
    /// The part of a cell left over, down and sideways.
    carried: (f64, f64),
    /// Where the pointer, or the finger, last was.
    pointer: (f64, f64),
    /// The way the wheel gesture goes, and when its last event came, while
    /// it hasn't run into the end.
    wheel: Option<(Axis, f64)>,
    touch: Touch,
}

impl Gestures {
    /// Gestures over a view with cells `width` by `height` pixels.
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            cell: (width, height),
            carried: (0.0, 0.0),
            pointer: (0.0, 0.0),
            wheel: None,
            touch: Touch::default(),
        }
    }

    /// A wheel event at `at`, going `delta` pixels right and down, at
    /// `time`. A wheel gesture, like a swipe on a trackpad, goes the way its
    /// first event mostly goes, until a pause, or until it runs into the
    /// end, like a sideways flick's momentum at the end of a line.
    pub fn wheel(
        &mut self,
        at: (f64, f64),
        delta: (f64, f64),
        time: f64,
        scroll: impl Scroll,
        area: Rect,
    ) {
        self.pointer = at;
        let (x, y) = delta;
        let axis = match self.wheel {
            Some((axis, last)) if time - last < WHEEL_GESTURE => axis,
            _ => {
                let axis = Axis::of(x, y);
                self.start(axis);
                axis
            }
        };
        let moved = self.move_by(axis, axis.part(x, y), scroll, area);
        self.wheel = moved.then_some((axis, time));
    }

    /// A finger coming down at `at`, at `time`, catching any glide.
    pub fn touch_start(&mut self, at: (f64, f64), time: f64) {
        self.pointer = at;
        self.touch = Touch {
            start: Some(at),
            last: Some(at),
            at: time,
            ..Touch::default()
        };
    }

    /// The finger moving to `at`, at `time`, dragging the view along. It
    /// doesn't move until it's gone far enough to pick which way it goes.
    pub fn touch_move(&mut self, at: (f64, f64), time: f64, scroll: impl Scroll, area: Rect) {
        let touch = &mut self.touch;
        let (Some(start), Some(last)) = (touch.start, touch.last) else {
            return;
        };
        let axis = match touch.axis {
            Some(axis) => axis,
            None => {
                let (x, y) = (at.0 - start.0, at.1 - start.1);
                if x.abs().max(y.abs()) < LOCK_DISTANCE {
                    return;
                }
                let axis = *touch.axis.insert(Axis::of(x, y));
                self.start(axis);
                axis
            }
        };
        let touch = &mut self.touch;
        touch.last = Some(at);
        let pixels = axis.part(last.0 - at.0, last.1 - at.1);
        let ms = (time - touch.at).max(1.0);
        // Smoothed, so one jittery event doesn't decide a flick.
        touch.speed = 0.8 * pixels / ms + 0.2 * touch.speed;
        touch.at = time;
        let _ = self.move_by(axis, pixels, scroll, area);
    }

    /// The finger lifting. Returns whether it was a flick, which glides on
    /// with [`glide`](Self::glide) once a frame.
    pub fn touch_end(&mut self) -> bool {
        let touch = &mut self.touch;
        touch.start = None;
        touch.last = None;
        touch.gliding = touch.axis.take().filter(|_| touch.speed.abs() > MIN_GLIDE);
        touch.glided = None;
        touch.gliding.is_some()
    }

    /// Keeps a flick going, for a frame drawn at `time`. Returns whether
    /// it's still gliding, until it slows to a stop, runs into the end, or
    /// another touch catches it.
    pub fn glide(&mut self, time: f64, scroll: impl Scroll, area: Rect) -> bool {
        let touch = &mut self.touch;
        let Some(axis) = touch.gliding else {
            return false;
        };
        let ms = touch
            .glided
            .map_or(FRAME, |last| (time - last).clamp(1.0, 100.0));
        touch.glided = Some(time);
        let pixels = touch.speed * ms;
        // About 5% slower every 16 milliseconds.
        touch.speed *= 0.95_f64.powf(ms / FRAME);
        if touch.speed.abs() <= MIN_GLIDE {
            touch.gliding = None;
        }
        if !self.move_by(axis, pixels, scroll, area) {
            self.touch.gliding = None;
        }
        self.touch.gliding.is_some()
    }

    /// Starts a gesture going `axis`, dropping what the last one left over,
    /// so this one doesn't start part of a cell in.
    fn start(&mut self, axis: Axis) {
        *self.carried_mut(axis) = 0.0;
    }

    /// Moves `pixels` further along `axis`, and says whether the view
    /// moved, which it doesn't at the end.
    fn move_by(&mut self, axis: Axis, pixels: f64, mut scroll: impl Scroll, area: Rect) -> bool {
        let size = axis.part(self.cell.0, self.cell.1);
        let carried = self.carried_mut(axis);
        let total = *carried + pixels;
        let cells = (total / size).trunc();
        *carried = total - cells * size;
        if cells == 0.0 {
            return true;
        }
        let at = |pixels: f64, size: f64| (pixels / size).max(0.0) as u16;
        let step = Step {
            axis,
            cells: cells as isize,
            column: at(self.pointer.0, self.cell.0),
            row: at(self.pointer.1, self.cell.1),
        };
        scroll.scroll(step, area)
    }

    fn carried_mut(&mut self, axis: Axis) -> &mut f64 {
        match axis {
            Axis::Vertical => &mut self.carried.1,
            Axis::Horizontal => &mut self.carried.0,
        }
    }
}

/// A finger on the screen, and how fast it was going.
#[derive(Debug, Clone, Default)]
struct Touch {
    /// Where the finger came down, and where it was last.
    start: Option<(f64, f64)>,
    last: Option<(f64, f64)>,
    /// The way this touch moves the view, once it's gone far enough to tell.
    axis: Option<Axis>,
    /// When the last touch event came.
    at: f64,
    /// In pixels a millisecond, down or right when positive.
    speed: f64,
    /// The way a flick is gliding, and when its last frame was.
    gliding: Option<Axis>,
    glided: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where the tests' views are drawn.
    const AREA: Rect = Rect::new(0, 0, 80, 24);

    /// A view recording the steps it's given, which moves until there are
    /// `end` of them.
    struct Record<'a> {
        steps: &'a mut Vec<Step>,
        end: usize,
    }

    impl Scroll for Record<'_> {
        fn scroll(&mut self, step: Step, _: Rect) -> bool {
            self.steps.push(step);
            self.steps.len() < self.end
        }
    }

    fn record(steps: &mut Vec<Step>, end: usize) -> Record<'_> {
        Record { steps, end }
    }

    fn cells(steps: &[Step]) -> Vec<(Axis, isize)> {
        steps.iter().map(|s| (s.axis, s.cells)).collect()
    }

    #[test]
    fn steps_scroll_offsets_their_way_to_the_ends() {
        let step = |axis, cells| Step {
            axis,
            cells,
            column: 0,
            row: 0,
        };
        let mut offset = 2;
        assert!(step(Axis::Vertical, 5).scroll(Axis::Vertical, &mut offset, 10));
        assert_eq!(offset, 7);
        // Not the other way.
        assert!(!step(Axis::Horizontal, 1).scroll(Axis::Vertical, &mut offset, 10));
        assert_eq!(offset, 7);
        // Only as far as the end, and not past it.
        assert!(step(Axis::Vertical, 5).scroll(Axis::Vertical, &mut offset, 10));
        assert!(!step(Axis::Vertical, 1).scroll(Axis::Vertical, &mut offset, 10));
        assert_eq!(offset, 10);
        // Or the start.
        assert!(step(Axis::Vertical, -20).scroll(Axis::Vertical, &mut offset, 10));
        assert!(!step(Axis::Vertical, -1).scroll(Axis::Vertical, &mut offset, 10));
        assert_eq!(offset, 0);
        // An offset left past a shorter end comes back to it.
        let mut offset = 12;
        assert!(step(Axis::Vertical, -1).scroll(Axis::Vertical, &mut offset, 10));
        assert_eq!(offset, 10);
    }

    #[test]
    fn steps_are_over_the_areas_their_cell_is_in() {
        let step = Step {
            axis: Axis::Vertical,
            cells: 1,
            column: 12,
            row: 5,
        };
        assert!(step.is_over(Rect::new(10, 5, 3, 1)));
        // Not past its right edge, or above it.
        assert!(!step.is_over(Rect::new(10, 5, 2, 1)));
        assert!(!step.is_over(Rect::new(10, 6, 3, 1)));
    }

    #[test]
    fn moves_a_cell_at_a_time() {
        let mut gestures = Gestures::new(10.0, 10.0);
        let mut steps = Vec::new();
        for (pixels, time) in [(4.0, 0.0), (4.0, 10.0), (25.0, 20.0), (-30.0, 30.0)] {
            gestures.wheel(
                (0.0, 0.0),
                (0.0, pixels),
                time,
                record(&mut steps, 10),
                AREA,
            );
        }
        // 8 pixels isn't a cell yet, 33 is 3 with 3 over, and -27 is -2.
        assert_eq!(cells(&steps), [(Axis::Vertical, 3), (Axis::Vertical, -2)]);
        // After a pause a new gesture starts from nothing, rather than the -7
        // left over, so two 5s make a cell.
        gestures.wheel((0.0, 0.0), (0.0, 5.0), 500.0, record(&mut steps, 10), AREA);
        gestures.wheel((0.0, 0.0), (0.0, 5.0), 510.0, record(&mut steps, 10), AREA);
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn wheels_move_one_way_until_the_end() {
        let mut gestures = Gestures::new(10.0, 10.0);
        let mut steps = Vec::new();
        gestures.wheel((25.0, 35.0), (20.0, 5.0), 0.0, record(&mut steps, 10), AREA);
        // Still sideways, though now it's mostly down.
        gestures.wheel(
            (25.0, 35.0),
            (10.0, 40.0),
            50.0,
            record(&mut steps, 10),
            AREA,
        );
        assert_eq!(
            cells(&steps),
            [(Axis::Horizontal, 2), (Axis::Horizontal, 1)]
        );
        // Over the cell at the pointer.
        assert_eq!((steps[0].column, steps[0].row), (2, 3));
        // Running into the end frees the next event to go the other way.
        gestures.wheel((0.0, 0.0), (10.0, 0.0), 60.0, record(&mut steps, 3), AREA);
        gestures.wheel((0.0, 0.0), (0.0, 10.0), 70.0, record(&mut steps, 10), AREA);
        assert_eq!(steps[3].axis, Axis::Vertical);
    }

    #[test]
    fn touches_move_one_way() {
        let mut gestures = Gestures::new(1.0, 1.0);
        let mut steps = Vec::new();
        gestures.touch_start((100.0, 100.0), 0.0);
        // Too short a move to tell which way.
        gestures.touch_move((103.0, 102.0), 10.0, record(&mut steps, 10), AREA);
        assert!(steps.is_empty());
        gestures.touch_move((90.0, 104.0), 20.0, record(&mut steps, 10), AREA);
        // Still sideways, though now it's mostly down.
        gestures.touch_move((88.0, 140.0), 30.0, record(&mut steps, 10), AREA);
        assert_eq!(
            cells(&steps),
            [(Axis::Horizontal, 10), (Axis::Horizontal, 2)]
        );
        // Over where the finger came down.
        assert_eq!((steps[1].column, steps[1].row), (100, 100));
        // Held still before lifting, so it doesn't glide.
        gestures.touch_move((88.0, 140.0), 500.0, record(&mut steps, 10), AREA);
        assert!(!gestures.touch_end());
    }

    /// A finger flicking `gestures` up the view, and lifting.
    fn flick(gestures: &mut Gestures) -> bool {
        let mut steps = Vec::new();
        gestures.touch_start((100.0, 300.0), 0.0);
        for (i, y) in [280.0, 240.0, 200.0].into_iter().enumerate() {
            let moves = record(&mut steps, usize::MAX);
            gestures.touch_move((100.0, y), 10.0 * (i + 1) as f64, moves, AREA);
        }
        gestures.touch_end()
    }

    #[test]
    fn flicks_glide_until_they_stop() {
        let mut gestures = Gestures::new(1.0, 1.0);
        let mut steps = Vec::new();
        assert!(flick(&mut gestures));
        let mut time = 100.0;
        while gestures.glide(time, record(&mut steps, usize::MAX), AREA) {
            time += 16.0;
        }
        // It glides on down, then slows to a stop.
        assert!(steps.len() > 1);
        assert!(
            steps
                .iter()
                .all(|s| s.axis == Axis::Vertical && s.cells > 0)
        );
        assert!(steps[0].cells > steps[steps.len() - 1].cells);
        assert!(!gestures.glide(time + 16.0, record(&mut steps, usize::MAX), AREA));

        // It stops at the end.
        assert!(flick(&mut gestures));
        assert!(!gestures.glide(100.0, record(&mut Vec::new(), 0), AREA));

        // Or when another touch catches it.
        assert!(flick(&mut gestures));
        gestures.touch_start((100.0, 300.0), 200.0);
        let mut caught = Vec::new();
        assert!(!gestures.glide(216.0, record(&mut caught, usize::MAX), AREA));
        assert!(caught.is_empty());
    }
}
