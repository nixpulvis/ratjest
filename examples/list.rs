//! Two lists, each scrolled by the mouse wheel when the pointer's over it.
//!
//!     cargo run --example list
//!
//! The list on the left has nothing selected, so the wheel moves its offset,
//! the first item drawn. The list on the right keeps its selected item in
//! view, whatever its offset, so there the wheel moves the selection, like
//! `↑` and `↓` do. `q` quits.
//!
//! Each `ListState` is a [`Scroll`], and so are the two together, [`Lists`],
//! which is what the wheel scrolls. It lays the lists out to scroll them
//! the same way it does to draw them.

use std::io;

use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEvent,
    MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, List, ListState};
use ratatui::{DefaultTerminal, Frame};
use ratjest::terminal::Wheel;
use ratjest::{Scroll, Step};

/// How many items each list has.
const ITEMS: usize = 100;

/// How far a tick of the mouse wheel scrolls, in items.
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
    lists: Lists,
}

impl App {
    fn new() -> Self {
        Self {
            wheel: Wheel::new(WHEEL),
            lists: Lists {
                offset: ListState::default(),
                selection: ListState::default().with_selected(Some(0)),
            },
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            let area = terminal.draw(|frame| self.lists.draw(frame))?.area;
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => self.lists.selection.select_next(),
                    KeyCode::Up => self.lists.selection.select_previous(),
                    _ => {}
                },
                Event::Mouse(mouse) => self.mouse(mouse, area),
                _ => {}
            }
        }
    }

    /// Hands the wheel's ticks to the gesture, which keeps a swipe going one
    /// way, and scrolls the lists, drawn in `area`.
    fn mouse(&mut self, mouse: MouseEvent, area: Rect) {
        let ticks = match mouse.kind {
            MouseEventKind::ScrollDown => (0, 1),
            MouseEventKind::ScrollUp => (0, -1),
            MouseEventKind::ScrollRight => (1, 0),
            MouseEventKind::ScrollLeft => (-1, 0),
            _ => return,
        };
        self.wheel
            .tick(mouse.column, mouse.row, ticks, &mut self.lists, area);
    }
}

/// The two lists' states.
struct Lists {
    /// Nothing selected, scrolled by its offset.
    offset: ListState,
    /// An item selected, scrolled by moving the selection.
    selection: ListState,
}

impl Scroll for Lists {
    /// Scrolls whichever list the step is over.
    fn scroll(&mut self, step: Step, area: Rect) -> bool {
        let [left, right] = Self::layout(area);
        let block = Block::bordered();
        self.offset.scroll(step, block.inner(left))
            || self.selection.scroll(step, block.inner(right))
    }
}

impl Lists {
    fn draw(&mut self, frame: &mut Frame) {
        let [left, right] = Self::layout(frame.area());

        let block = Block::bordered().title(" Offset ");
        let list = List::new(items()).block(block);
        frame.render_stateful_widget(list, left, &mut self.offset);

        let block = Block::bordered().title(" Selection ");
        let list = List::new(items())
            .block(block)
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, right, &mut self.selection);
    }

    /// Where the two lists go in `area`, side by side.
    fn layout(area: Rect) -> [Rect; 2] {
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(area)
    }
}

fn items() -> impl Iterator<Item = String> {
    (1..=ITEMS).map(|item| format!("Item {item}"))
}
