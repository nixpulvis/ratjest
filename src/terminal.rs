//! Following a terminal's mouse wheel with [`Gestures`].
//!
//! A terminal reports the wheel a tick at a time, up, down, left or right,
//! and a trackpad swipe that's a little diagonal sends ticks both ways. A
//! [`Wheel`] keeps each gesture going one way, so the swipe goes either up
//! and down or sideways, not both.
//!
//! It counts time with std's [`Instant`], so it isn't built for
//! `wasm32-unknown-unknown`, which has no clock for it. In a browser, use
//! the `web` feature's `web::listen` instead.

use std::time::Instant;

use ratatui_core::layout::Rect;

use crate::{Gestures, Scroll};

/// A terminal's mouse wheel, moving the view a few cells a tick, one way a
/// gesture.
///
/// The terminal's events are the host's to read, so each gives a `match`
/// from its wheel events to ticks, like this one for crossterm's:
///
/// ```
/// use crossterm::event::{MouseEvent, MouseEventKind};
/// # use crossterm::event::KeyModifiers;
/// use ratatui::layout::Rect;
/// use ratatui::widgets::ListState;
/// use ratjest::Scroll;
/// use ratjest::terminal::Wheel;
///
/// /// How far a crossterm wheel event goes, right and down, in ticks.
/// fn ticks(kind: MouseEventKind) -> Option<(isize, isize)> {
///     match kind {
///         MouseEventKind::ScrollDown => Some((0, 1)),
///         MouseEventKind::ScrollUp => Some((0, -1)),
///         MouseEventKind::ScrollRight => Some((1, 0)),
///         MouseEventKind::ScrollLeft => Some((-1, 0)),
///         _ => None,
///     }
/// }
///
/// // A list, and where its items are drawn.
/// let mut state = ListState::default();
/// let area = Rect::new(0, 0, 30, 5);
/// // Three rows, or columns, a tick.
/// let mut wheel = Wheel::new(3);
///
/// // On each mouse event.
/// # let mouse = MouseEvent {
/// #     kind: MouseEventKind::ScrollDown,
/// #     column: 4,
/// #     row: 2,
/// #     modifiers: KeyModifiers::NONE,
/// # };
/// if let Some(ticks) = ticks(mouse.kind) {
///     wheel.tick(mouse.column, mouse.row, ticks, &mut state, area);
/// }
/// assert_eq!(state.offset(), 3);
/// ```
#[derive(Debug, Clone)]
pub struct Wheel {
    gestures: Gestures,
    /// What the gestures' times count from.
    start: Instant,
    /// How many lines, or columns, a tick moves.
    cells: f64,
}

impl Wheel {
    /// A wheel moving `cells` lines, or columns, a tick.
    pub fn new(cells: u16) -> Self {
        Self {
            // Ticks and the pointer are in whole cells already.
            gestures: Gestures::new(1.0, 1.0),
            start: Instant::now(),
            cells: f64::from(cells),
        }
    }

    /// Ticks over `column`, `row`, going `ticks.0` right and `ticks.1` down,
    /// scrolling `scroll`, which is drawn in `area`. Each gesture goes the
    /// way its first tick does, until a pause, or until it runs into the
    /// end.
    pub fn tick(
        &mut self,
        column: u16,
        row: u16,
        ticks: (isize, isize),
        scroll: impl Scroll,
        area: Rect,
    ) {
        let time = self.start.elapsed().as_secs_f64() * 1000.0;
        self.tick_at(column, row, ticks, time, scroll, area);
    }

    fn tick_at(
        &mut self,
        column: u16,
        row: u16,
        (x, y): (isize, isize),
        time: f64,
        scroll: impl Scroll,
        area: Rect,
    ) {
        let at = (f64::from(column), f64::from(row));
        let delta = (x as f64 * self.cells, y as f64 * self.cells);
        self.gestures.wheel(at, delta, time, scroll, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Axis, Step};

    /// A view recording the steps it's given.
    struct Record(Vec<Step>);

    impl Scroll for Record {
        fn scroll(&mut self, step: Step, _: Rect) -> bool {
            self.0.push(step);
            true
        }
    }

    #[test]
    fn ticks_move_one_way_over_the_pointer() {
        let mut wheel = Wheel::new(3);
        let area = Rect::new(0, 0, 80, 24);
        let mut record = Record(Vec::new());
        wheel.tick_at(7, 4, (1, 0), 0.0, &mut record, area);
        // A diagonal swipe's tick the other way, in the same gesture.
        wheel.tick_at(7, 4, (0, 1), 20.0, &mut record, area);
        wheel.tick_at(7, 4, (-1, 0), 40.0, &mut record, area);
        // After a pause, a new gesture can go the other way.
        wheel.tick_at(7, 4, (0, 1), 500.0, &mut record, area);
        let step = |axis, cells| Step {
            axis,
            cells,
            column: 7,
            row: 4,
        };
        assert_eq!(
            record.0,
            [
                step(Axis::Horizontal, 3),
                step(Axis::Horizontal, -3),
                step(Axis::Vertical, 3),
            ]
        );
    }
}
