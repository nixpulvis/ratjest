# ratjest

[![CI](https://github.com/nixpulvis/ratjest/actions/workflows/ci.yml/badge.svg)](https://github.com/nixpulvis/ratjest/actions/workflows/ci.yml)
[![Documentation](https://img.shields.io/badge/docs-master-blue)](https://nixpulvis.com/ratjest/)

Scroll gestures for [Ratatui] apps, from a wheel, a trackpad or a finger.

It turns a host's wheel and touch events into steps of whole cells, and gives
each one to what scrolls. Each gesture only goes one way, up and down or
sideways, so a swipe that's a little diagonal doesn't do both. In a browser a
flick keeps gliding for a bit, like a page does.

## Usage

What scrolls is a `Scroll`: a widget's state, given a step and the area the
widget is drawn in, like when it's drawn.

```rust
pub trait Scroll {
    fn scroll(&mut self, step: Step, area: Rect) -> bool;
}
```

Ratatui's `ListState` and `TableState` are `Scroll`s. In a terminal, give a
`Wheel` the wheel's ticks, here from [crossterm], and the state to scroll:

```rust
use crossterm::event::{MouseEvent, MouseEventKind};
use ratjest::terminal::Wheel;

// Three rows, or columns, a tick.
let mut wheel = Wheel::new(3);

// On each mouse event.
let ticks = match mouse.kind {
    MouseEventKind::ScrollDown => (0, 1),
    MouseEventKind::ScrollUp => (0, -1),
    MouseEventKind::ScrollRight => (1, 0),
    MouseEventKind::ScrollLeft => (-1, 0),
    _ => return,
};
wheel.tick(mouse.column, mouse.row, ticks, &mut state, area);
```

In a browser, the `web` feature's `web::listen` follows the wheel and touch
events over the element a view is drawn in, like [Ratzilla]'s, and scrolls a
`Scroll` it shares with what draws, an `Rc<RefCell<_>>`. It needs the size of
a cell in CSS pixels, to turn pixels into cells.

Anywhere else that reports movement in pixels, feed `Gestures` from its events.

## Several widgets

A `Step` is so many cells one way, over the cell the gesture is on. A `Scroll`
does nothing with a step that isn't over its area, and says whether it moved.
So an app with several widgets is a `Scroll` too. It splits its area the way
it does to draw, and tries each state with the part it's drawn in:

```rust
impl Scroll for App {
    fn scroll(&mut self, step: Step, area: Rect) -> bool {
        let [left, right] = Self::layout(area);
        self.files.scroll(step, left) || self.preview.scroll(step, right)
    }
}

wheel.tick(mouse.column, mouse.row, ticks, &mut app, frame_area);
```

When nothing moved, the gesture ends, which also stops a glide.

## Lists and tables

For a `ListState` or a `TableState`, `area` is where the rows are, inside the
block and below a table's header. With a row selected a step moves the
selection, which the widget keeps in view. With nothing selected it moves the
offset, the first row drawn.

Those states don't know how many rows there are. So like their own
`scroll_down_by`, a step can go past the last row, and the widget brings it
back when it's next drawn. That lets an offset go on until the last row is at
the top. To stop with the last row at the bottom, the app's own state can know
the count:

```rust
struct Items {
    state: ListState,
    len: usize,
}

impl Scroll for Items {
    fn scroll(&mut self, step: Step, area: Rect) -> bool {
        let last = self.len.saturating_sub(usize::from(area.height));
        step.is_over(area) && step.scroll(Axis::Vertical, self.state.offset_mut(), last)
    }
}
```

## Your own widgets

`Step::scroll` moves an offset by a step, one way, from 0 through the last
place it can be. A widget of the app's own implements `Scroll` for its state
with one for each way it goes:

```rust
impl Scroll for GridState {
    fn scroll(&mut self, step: Step, area: Rect) -> bool {
        let (last_column, last_row) = Self::last(area);
        step.is_over(area)
            && (step.scroll(Axis::Horizontal, &mut self.column, last_column)
                || step.scroll(Axis::Vertical, &mut self.row, last_row))
    }
}
```

## Examples

Two small apps to run in a terminal, and to copy from:

- `cargo run --example list` has two lists side by side, which are a `Scroll`
  together. The wheel scrolls the one under the pointer, the first by its
  offset and the second by its selection.
- `cargo run --example custom` has a widget of its own, a grid bigger than the
  screen, that the wheel scrolls both ways, with a `Scroll` of its own.

## Testing

`cargo test` runs the unit tests and the examples in the documentation.
`cargo test --all-features` builds the `web` feature too.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

[Ratatui]: https://ratatui.rs
[crossterm]: https://github.com/crossterm-rs/crossterm
[Ratzilla]: https://github.com/ratatui/ratzilla
