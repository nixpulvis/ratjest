# ratjest

[![CI](https://github.com/nixpulvis/ratjest/actions/workflows/ci.yml/badge.svg)](https://github.com/nixpulvis/ratjest/actions/workflows/ci.yml)
[![Documentation](https://img.shields.io/badge/docs-master-blue)](https://nixpulvis.com/ratjest/)

Scroll gestures for terminal UIs, like [Ratatui]'s, from a wheel, a trackpad or
a finger.

It turns a host's wheel and touch events into steps of whole cells, and hands
each one to a callback that moves whatever the app scrolls. Each gesture only
goes one way, up and down or sideways, so a swipe that's a little diagonal
doesn't do both. In a browser a flick keeps gliding for a bit, like a page does.

Nothing in it draws, or depends on what does, so it works with any widget, and
without Ratatui too.

## Usage

In a terminal, give a `Wheel` the wheel's ticks, here from [crossterm]:

```rust
use crossterm::event::{MouseEvent, MouseEventKind};
use ratjest::terminal::Wheel;
use ratjest::{Axis, Step};

// Twenty lines in a view five tall, so the top line is 0 through 15.
let mut top: isize = 0;
let mut scroll = |step: Step| {
    if step.axis != Axis::Vertical {
        return false;
    }
    let before = top;
    top = (top + step.cells).clamp(0, 15);
    top != before
};
// Three lines, or columns, a tick.
let mut wheel = Wheel::new(3);

// On each mouse event.
let ticks = match mouse.kind {
    MouseEventKind::ScrollDown => (0, 1),
    MouseEventKind::ScrollUp => (0, -1),
    MouseEventKind::ScrollRight => (1, 0),
    MouseEventKind::ScrollLeft => (-1, 0),
    _ => return,
};
wheel.tick(mouse.column, mouse.row, ticks, &mut scroll);
```

A `Step` is so many cells one way, over the cell the gesture is on, which says
which part of the screen to scroll. The callback returns whether it moved. At
the end of what there is to scroll it returns `false`, which ends the gesture
and stops a glide.

In a browser, the `web` feature's `web::listen` follows the wheel and touch
events over the element a view is drawn in, like [Ratzilla]'s, with the same
callback. It needs the size of a cell in CSS pixels, to turn pixels into cells.

Anywhere else that reports movement in pixels, feed `Gestures` from its events.

## Testing

`cargo test` runs the unit tests and the examples in the documentation.
`cargo test --all-features` builds the `web` feature too.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

[Ratatui]: https://ratatui.rs
[crossterm]: https://github.com/crossterm-rs/crossterm
[Ratzilla]: https://github.com/ratatui/ratzilla
