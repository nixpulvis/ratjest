//! Scrolling a terminal UI with a wheel, a trackpad or a finger, a cell at a
//! time.
//!
//! Each gesture only goes one way, up and down, or sideways, so a swipe
//! that's a little diagonal doesn't do both. Hosts that report movement in
//! pixels, like a browser, move the view as far as the hand does, a line or
//! a column at a time once one's worth adds up, and a flick keeps the view
//! gliding for a bit, slowing down, like a page in the browser does.
//!
//! [`Gestures`] does the arithmetic, from pixels and timestamps, and hands
//! each whole cell of movement to a callback as a [`Step`]. The callback
//! moves whatever the app scrolls, with [`Step::scroll`] where that's an
//! offset, like a list's, and returns whether it moved, which ends the
//! gesture at the end of what there is to scroll. Each host feeds it its own
//! events:
//!
//! - In a terminal, [`terminal::Wheel`] takes the wheel's ticks, like
//!   crossterm's `MouseEventKind::ScrollDown`.
//! - In a browser, the `web` feature's `web::listen` follows the wheel and
//!   touch events over the element a view is drawn in, like [Ratzilla]'s.
//! - Anywhere else that reports movement in pixels, call [`Gestures`]'
//!   methods from its events, and [`Gestures::glide`] once a frame after
//!   [`Gestures::touch_end`] says a touch was a flick.
//!
//! Nothing here draws, or depends on what does. A [`Step`] is only so many
//! cells one way, over the cell the gesture is on.
//!
//! ```
//! use ratjest::{Axis, Gestures, Step};
//!
//! // Twenty lines in a view five tall, so the top line is 0 through 15.
//! let mut top = 0;
//! let mut scroll = |step: Step| step.scroll(Axis::Vertical, &mut top, 15);
//!
//! // Cells 8 pixels wide and 16 tall.
//! let mut gestures = Gestures::new(8.0, 16.0);
//!
//! // Half a line doesn't move yet, the rest of a line and a half does.
//! gestures.wheel((0.0, 0.0), (0.0, 8.0), 0.0, &mut scroll);
//! gestures.wheel((0.0, 0.0), (0.0, 24.0), 10.0, &mut scroll);
//! assert_eq!(top, 2);
//! ```
//!
//! [Ratzilla]: https://github.com/ratatui/ratzilla

#![warn(missing_docs)]

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub mod terminal;
#[cfg(feature = "web")]
pub mod web;

/// Slowest a flick glides, in pixels a millisecond.
const MIN_GLIDE: f64 = 0.05;

/// How far a finger goes before its touch picks a way to move, in pixels.
const LOCK_DISTANCE: f64 = 6.0;

/// How long between wheel events before the next starts a new gesture, in
/// milliseconds.
const WHEEL_GESTURE: f64 = 200.0;

/// How long a glide's first frame is taken to be, in milliseconds.
const FRAME: f64 = 16.0;

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
    /// Scrolls `offset` by this step's cells, when it goes `axis`'s way,
    /// keeping it from 0 through `last`. Returns whether it moved, which it
    /// doesn't the other way, or at either end.
    ///
    /// An offset is how far something is scrolled: the line at the top of
    /// some text, the first row drawn of a table, the column a wide line
    /// starts from. [Ratatui]'s `ListState` and `TableState` each keep one,
    /// the index of the first item drawn. They don't know how many items
    /// there are, or how many fit, so the app says where the end is:
    ///
    /// ```
    /// use ratatui::layout::Rect;
    /// use ratatui::widgets::ListState;
    /// use ratjest::{Axis, Step};
    ///
    /// // Twenty items in a list five rows tall.
    /// let items = 20;
    /// let area = Rect::new(0, 0, 30, 5);
    /// let mut state = ListState::default();
    /// let last = items - usize::from(area.height);
    ///
    /// // In a `Wheel`'s or `web::listen`'s callback.
    /// # let step = Step { axis: Axis::Vertical, cells: 3, column: 4, row: 2 };
    /// let moved = area.contains((step.column, step.row).into())
    ///     && step.scroll(Axis::Vertical, state.offset_mut(), last);
    /// assert!(moved);
    /// assert_eq!(state.offset(), 3);
    /// ```
    ///
    /// A list or a table with an item selected keeps that item in view,
    /// whatever its offset, so scroll the selection instead, through the
    /// last item:
    ///
    /// ```
    /// use ratatui::widgets::TableState;
    /// use ratjest::{Axis, Step};
    ///
    /// let rows = 20;
    /// let mut state = TableState::default().with_selected(18);
    ///
    /// # let step = Step { axis: Axis::Vertical, cells: 3, column: 4, row: 2 };
    /// let mut selected = state.selected().unwrap_or(0);
    /// let moved = step.scroll(Axis::Vertical, &mut selected, rows - 1);
    /// state.select(Some(selected));
    /// assert!(moved);
    /// assert_eq!(state.selected(), Some(19));
    /// ```
    ///
    /// [Ratatui]: https://ratatui.rs
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
/// cells, keeping the part of a cell left over for the gesture's next
/// event.
///
/// Positions are in pixels from the view's top left corner, and times in
/// milliseconds from any fixed point, like a browser event's `timeStamp`.
/// Each method that moves takes a `step` callback, which carries out a
/// [`Step`] and returns whether it moved, which it doesn't at the end.
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
        step: impl FnOnce(Step) -> bool,
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
        let moved = self.move_by(axis, axis.part(x, y), step);
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
    pub fn touch_move(&mut self, at: (f64, f64), time: f64, step: impl FnOnce(Step) -> bool) {
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
        let _ = self.move_by(axis, pixels, step);
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
    pub fn glide(&mut self, time: f64, step: impl FnOnce(Step) -> bool) -> bool {
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
        if !self.move_by(axis, pixels, step) {
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
    fn move_by(&mut self, axis: Axis, pixels: f64, step: impl FnOnce(Step) -> bool) -> bool {
        let size = axis.part(self.cell.0, self.cell.1);
        let carried = self.carried_mut(axis);
        let total = *carried + pixels;
        let cells = (total / size).trunc();
        *carried = total - cells * size;
        if cells == 0.0 {
            return true;
        }
        let at = |pixels: f64, size: f64| (pixels / size).max(0.0) as u16;
        step(Step {
            axis,
            cells: cells as isize,
            column: at(self.pointer.0, self.cell.0),
            row: at(self.pointer.1, self.cell.1),
        })
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

    /// A callback recording the steps it takes, which move until there are
    /// `end` of them.
    fn record(steps: &mut Vec<Step>, end: usize) -> impl FnOnce(Step) -> bool + '_ {
        move |step| {
            steps.push(step);
            steps.len() < end
        }
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
    fn moves_a_cell_at_a_time() {
        let mut gestures = Gestures::new(10.0, 10.0);
        let mut steps = Vec::new();
        for (pixels, time) in [(4.0, 0.0), (4.0, 10.0), (25.0, 20.0), (-30.0, 30.0)] {
            gestures.wheel((0.0, 0.0), (0.0, pixels), time, record(&mut steps, 10));
        }
        // 8 pixels isn't a cell yet, 33 is 3 with 3 over, and -27 is -2.
        assert_eq!(cells(&steps), [(Axis::Vertical, 3), (Axis::Vertical, -2)]);
        // After a pause a new gesture starts from nothing, rather than the -7
        // left over, so two 5s make a cell.
        gestures.wheel((0.0, 0.0), (0.0, 5.0), 500.0, record(&mut steps, 10));
        gestures.wheel((0.0, 0.0), (0.0, 5.0), 510.0, record(&mut steps, 10));
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn wheels_move_one_way_until_the_end() {
        let mut gestures = Gestures::new(10.0, 10.0);
        let mut steps = Vec::new();
        gestures.wheel((25.0, 35.0), (20.0, 5.0), 0.0, record(&mut steps, 10));
        // Still sideways, though now it's mostly down.
        gestures.wheel((25.0, 35.0), (10.0, 40.0), 50.0, record(&mut steps, 10));
        assert_eq!(
            cells(&steps),
            [(Axis::Horizontal, 2), (Axis::Horizontal, 1)]
        );
        // Over the cell at the pointer.
        assert_eq!((steps[0].column, steps[0].row), (2, 3));
        // Running into the end frees the next event to go the other way.
        gestures.wheel((0.0, 0.0), (10.0, 0.0), 60.0, record(&mut steps, 3));
        gestures.wheel((0.0, 0.0), (0.0, 10.0), 70.0, record(&mut steps, 10));
        assert_eq!(steps[3].axis, Axis::Vertical);
    }

    #[test]
    fn touches_move_one_way() {
        let mut gestures = Gestures::new(1.0, 1.0);
        let mut steps = Vec::new();
        gestures.touch_start((100.0, 100.0), 0.0);
        // Too short a move to tell which way.
        gestures.touch_move((103.0, 102.0), 10.0, record(&mut steps, 10));
        assert!(steps.is_empty());
        gestures.touch_move((90.0, 104.0), 20.0, record(&mut steps, 10));
        // Still sideways, though now it's mostly down.
        gestures.touch_move((88.0, 140.0), 30.0, record(&mut steps, 10));
        assert_eq!(
            cells(&steps),
            [(Axis::Horizontal, 10), (Axis::Horizontal, 2)]
        );
        // Over where the finger came down.
        assert_eq!((steps[1].column, steps[1].row), (100, 100));
        // Held still before lifting, so it doesn't glide.
        gestures.touch_move((88.0, 140.0), 500.0, record(&mut steps, 10));
        assert!(!gestures.touch_end());
    }

    /// A finger flicking `gestures` up the view, and lifting.
    fn flick(gestures: &mut Gestures) -> bool {
        gestures.touch_start((100.0, 300.0), 0.0);
        for (i, y) in [280.0, 240.0, 200.0].into_iter().enumerate() {
            gestures.touch_move((100.0, y), 10.0 * (i + 1) as f64, |_| true);
        }
        gestures.touch_end()
    }

    #[test]
    fn flicks_glide_until_they_stop() {
        let mut gestures = Gestures::new(1.0, 1.0);
        let mut steps = Vec::new();
        assert!(flick(&mut gestures));
        let mut time = 100.0;
        while gestures.glide(time, record(&mut steps, usize::MAX)) {
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
        assert!(!gestures.glide(time + 16.0, record(&mut steps, usize::MAX)));

        // It stops at the end.
        assert!(flick(&mut gestures));
        assert!(!gestures.glide(100.0, |_| false));

        // Or when another touch catches it.
        assert!(flick(&mut gestures));
        gestures.touch_start((100.0, 300.0), 200.0);
        assert!(!gestures.glide(216.0, |_| panic!("caught glides on")));
    }
}
