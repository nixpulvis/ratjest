//! A widget of the app's own, scrolled both ways by the mouse wheel.
//!
//!     cargo run --example custom
//!
//! A [`Grid`] is bigger than the screen, and its [`GridState`] keeps how far
//! it's scrolled, right and down. A step from the wheel moves one or the
//! other, so a swipe that's a little diagonal doesn't slide the grid both
//! ways. The arrow keys scroll too, for a terminal that doesn't report a
//! sideways wheel. `q` quits.

use std::io;

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEvent,
    MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, StatefulWidget};
use ratatui::{DefaultTerminal, Frame};
use ratjest::terminal::Wheel;
use ratjest::{Axis, Step};

/// How many cells the grid has, across and down.
const SIZE: usize = 100;

/// How many columns of the screen a cell of the grid takes.
const CELL: usize = 8;

/// How far a tick of the mouse wheel scrolls, in rows, or columns.
const WHEEL: u16 = 3;

fn main() -> io::Result<()> {
    let result = ratatui::run(|terminal| {
        execute!(io::stdout(), EnableMouseCapture)?;
        App::new().run(terminal)
    });
    execute!(io::stdout(), DisableMouseCapture)?;
    result
}

struct App {
    wheel: Wheel,
    grid: GridState,
}

impl App {
    fn new() -> Self {
        Self {
            wheel: Wheel::new(WHEEL),
            grid: GridState::default(),
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Right => self.grid.scroll_by(CELL as isize, 0),
                    KeyCode::Left => self.grid.scroll_by(-(CELL as isize), 0),
                    KeyCode::Down => self.grid.scroll_by(0, 1),
                    KeyCode::Up => self.grid.scroll_by(0, -1),
                    _ => {}
                },
                Event::Mouse(mouse) => self.mouse(mouse),
                _ => {}
            }
        }
    }

    /// Hands the wheel's ticks to the gesture, which keeps a swipe going one
    /// way, and each of its steps to the grid.
    fn mouse(&mut self, mouse: MouseEvent) {
        let ticks = match mouse.kind {
            MouseEventKind::ScrollDown => (0, 1),
            MouseEventKind::ScrollUp => (0, -1),
            MouseEventKind::ScrollRight => (1, 0),
            MouseEventKind::ScrollLeft => (-1, 0),
            _ => return,
        };
        let grid = &mut self.grid;
        self.wheel
            .tick(mouse.column, mouse.row, ticks, |step| grid.step(step));
    }

    fn draw(&mut self, frame: &mut Frame) {
        let title = format!(" Column {}, row {} ", self.grid.column, self.grid.row);
        let block = Block::bordered().title(title);
        let area = block.inner(frame.area());
        frame.render_widget(block, frame.area());
        frame.render_stateful_widget(Grid, area, &mut self.grid);
    }
}

/// A grid of cells, each showing where in the grid it is, drawn from
/// wherever its state is scrolled to.
struct Grid;

impl StatefulWidget for Grid {
    type State = GridState;

    fn render(self, area: Rect, buffer: &mut Buffer, state: &mut GridState) {
        // Remember where the grid is, for steps over it and for its ends,
        // and stay inside them when the area grows.
        state.area = area;
        state.scroll_by(0, 0);

        for (line, row) in area.rows().zip(state.row..SIZE) {
            let text: String = (0..SIZE)
                .map(|column| format!("{:<CELL$}", format!("{column}:{row}")))
                .collect();
            let width = usize::from(line.width);
            buffer.set_stringn(line.x, line.y, &text[state.column..], width, Style::new());
        }
    }
}

/// How far a [`Grid`] is scrolled.
#[derive(Default)]
struct GridState {
    /// The first column of the screen's worth drawn, counted in the
    /// screen's columns, so a cell of the grid can be cut off part way.
    column: usize,
    /// The first row drawn.
    row: usize,
    /// Where the grid was last drawn.
    area: Rect,
}

impl GridState {
    /// Carries out a step, when it's over the grid, and returns whether it
    /// moved. A step only goes one way, so only one of the two scrolls.
    fn step(&mut self, step: Step) -> bool {
        let (last_column, last_row) = self.last();
        self.area.contains((step.column, step.row).into())
            && (step.scroll(Axis::Horizontal, &mut self.column, last_column)
                || step.scroll(Axis::Vertical, &mut self.row, last_row))
    }

    /// Scrolls `columns` right and `rows` down, as far as the grid goes.
    fn scroll_by(&mut self, columns: isize, rows: isize) {
        let (last_column, last_row) = self.last();
        self.column = self.column.saturating_add_signed(columns).min(last_column);
        self.row = self.row.saturating_add_signed(rows).min(last_row);
    }

    /// The last column and row it can scroll to, which put the grid's far
    /// edges on the area's.
    fn last(&self) -> (usize, usize) {
        (
            (SIZE * CELL).saturating_sub(usize::from(self.area.width)),
            SIZE.saturating_sub(usize::from(self.area.height)),
        )
    }
}
