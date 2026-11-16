//! Scrolling Ratatui's lists and tables.

use ratatui_core::layout::Rect;
use ratatui_widgets::list::ListState;
use ratatui_widgets::table::TableState;

use crate::{Axis, Scroll, Step};

/// A list scrolls by its selection when it has one, since the list keeps
/// the selected item in view, whatever its offset. With nothing selected it
/// scrolls by its offset, the first item drawn.
///
/// `area` is where the items are drawn, inside the list's block.
///
/// ```
/// use ratatui::layout::Rect;
/// use ratatui::widgets::ListState;
/// use ratjest::{Axis, Scroll, Step};
///
/// let area = Rect::new(0, 0, 30, 5);
/// let mut state = ListState::default();
///
/// // A step from a wheel, three rows down, over the list.
/// let step = Step { axis: Axis::Vertical, cells: 3, column: 4, row: 2 };
/// assert!(state.scroll(step, area));
/// assert_eq!(state.offset(), 3);
///
/// // With an item selected, the selection moves.
/// state.select(Some(1));
/// assert!(state.scroll(step, area));
/// assert_eq!(state.selected(), Some(4));
/// ```
///
/// The state doesn't know how many items there are, so a step doesn't stop
/// at the last one. Like the state's own `scroll_down_by`, it moves past,
/// and the list brings it back to the last item when it's next drawn. So a
/// step at the end still says it moved, and the offset goes on until the
/// last item is at the top. To stop where the last item is at the bottom,
/// give [`Step::scroll`] the state's `offset_mut()` and that end.
impl Scroll for ListState {
    fn scroll(&mut self, step: Step, area: Rect) -> bool {
        if !step.is_over(area) {
            return false;
        }
        match self.selected() {
            Some(mut selected) => {
                let moved = step.scroll(Axis::Vertical, &mut selected, usize::MAX);
                self.select(Some(selected));
                moved
            }
            None => step.scroll(Axis::Vertical, self.offset_mut(), usize::MAX),
        }
    }
}

/// A table scrolls like a list: by its selected row when it has one, and by
/// its offset, the first row drawn, when it doesn't. Nor does it know how
/// many rows there are, so the table brings a step past the last one back
/// when it's next drawn.
///
/// `area` is where the rows are drawn, inside the table's block and below
/// its header.
impl Scroll for TableState {
    fn scroll(&mut self, step: Step, area: Rect) -> bool {
        if !step.is_over(area) {
            return false;
        }
        match self.selected() {
            Some(mut selected) => {
                let moved = step.scroll(Axis::Vertical, &mut selected, usize::MAX);
                self.select(Some(selected));
                moved
            }
            None => step.scroll(Axis::Vertical, self.offset_mut(), usize::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::widgets::{List, StatefulWidget};

    use super::*;

    const AREA: Rect = Rect::new(10, 5, 30, 5);

    /// A step of `cells` down, over `AREA`'s top left corner.
    fn step(cells: isize) -> Step {
        Step {
            axis: Axis::Vertical,
            cells,
            column: AREA.x,
            row: AREA.y,
        }
    }

    /// Draws a list of `len` items in `AREA`, as an app would between steps.
    fn draw(state: &mut ListState, len: usize) {
        let items = (0..len).map(|item| item.to_string());
        let mut buffer = Buffer::empty(AREA);
        List::new(items).render(AREA, &mut buffer, state);
    }

    #[test]
    fn offsets_scroll_and_the_list_brings_them_back_to_its_last_item() {
        let mut state = ListState::default();
        assert!(state.scroll(step(3), AREA));
        assert_eq!(state.offset(), 3);
        assert_eq!(state.selected(), None);
        assert!(state.scroll(step(30), AREA));
        draw(&mut state, 20);
        assert_eq!(state.offset(), 19);
        // Back to the start, and no further.
        assert!(state.scroll(step(-30), AREA));
        assert!(!state.scroll(step(-1), AREA));
        assert_eq!(state.offset(), 0);
    }

    #[test]
    fn selections_scroll_and_the_list_brings_them_back_to_its_last_item() {
        let mut state = ListState::default().with_selected(Some(17));
        assert!(state.scroll(step(3), AREA));
        draw(&mut state, 20);
        assert_eq!(state.selected(), Some(19));
        // The list moved its offset to keep the selection in view.
        assert_eq!(state.offset(), 15);
        assert!(state.scroll(step(-30), AREA));
        assert!(!state.scroll(step(-1), AREA));
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn tables_scroll_like_lists() {
        let mut state = TableState::default();
        assert!(state.scroll(step(3), AREA));
        assert_eq!((state.offset(), state.selected()), (3, None));
        let mut state = TableState::default().with_selected(2);
        assert!(state.scroll(step(3), AREA));
        assert_eq!((state.offset(), state.selected()), (0, Some(5)));
    }

    #[test]
    fn only_steps_down_or_up_over_the_rows_scroll() {
        let mut state = ListState::default();
        let above = Step {
            row: AREA.y - 1,
            ..step(1)
        };
        let sideways = Step {
            axis: Axis::Horizontal,
            ..step(1)
        };
        assert!(!state.scroll(above, AREA));
        assert!(!state.scroll(sideways, AREA));
        assert_eq!(state.offset(), 0);
    }
}
