//! Two lists, each scrolled by the mouse wheel when the pointer's over it.
//!
//!     cargo run --example list
//!
//! The list on the left has nothing selected, so the wheel moves its offset,
//! the first item drawn. The list on the right keeps its selected item in
//! view, whatever its offset, so there the wheel moves the selection, like
//! `↑` and `↓` do. `q` quits.

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
use ratjest::{Axis, Step};

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
                offset_area: Rect::default(),
                selection: ListState::default().with_selected(Some(0)),
                selection_area: Rect::default(),
            },
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.lists.draw(frame))?;
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => self.lists.selection.select_next(),
                    KeyCode::Up => self.lists.selection.select_previous(),
                    _ => {}
                },
                Event::Mouse(mouse) => self.mouse(mouse),
                _ => {}
            }
        }
    }

    /// Hands the wheel's ticks to the gesture, which keeps a swipe going one
    /// way, and each of its steps to the lists.
    fn mouse(&mut self, mouse: MouseEvent) {
        let ticks = match mouse.kind {
            MouseEventKind::ScrollDown => (0, 1),
            MouseEventKind::ScrollUp => (0, -1),
            MouseEventKind::ScrollRight => (1, 0),
            MouseEventKind::ScrollLeft => (-1, 0),
            _ => return,
        };
        let lists = &mut self.lists;
        self.wheel
            .tick(mouse.column, mouse.row, ticks, |step| lists.step(step));
    }
}

/// The two lists' states, and where each list's items were last drawn.
struct Lists {
    /// Nothing selected, scrolled by its offset.
    offset: ListState,
    offset_area: Rect,
    /// An item selected, scrolled by moving the selection.
    selection: ListState,
    selection_area: Rect,
}

impl Lists {
    /// Carries out a step on the list it's over, and returns whether it
    /// moved. Neither list moves sideways, so a sideways step doesn't.
    fn step(&mut self, step: Step) -> bool {
        let over = (step.column, step.row).into();
        if self.offset_area.contains(over) {
            // The last offset puts the last item on the bottom row.
            let last = ITEMS.saturating_sub(usize::from(self.offset_area.height));
            step.scroll(Axis::Vertical, self.offset.offset_mut(), last)
        } else if self.selection_area.contains(over) {
            let mut selected = self.selection.selected().unwrap_or(0);
            let moved = step.scroll(Axis::Vertical, &mut selected, ITEMS - 1);
            self.selection.select(Some(selected));
            moved
        } else {
            false
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [left, right] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(frame.area());

        let block = Block::bordered().title(" Offset ");
        self.offset_area = block.inner(left);
        let list = List::new(items()).block(block);
        frame.render_stateful_widget(list, left, &mut self.offset);

        let block = Block::bordered().title(" Selection ");
        self.selection_area = block.inner(right);
        let list = List::new(items())
            .block(block)
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, right, &mut self.selection);
    }
}

fn items() -> impl Iterator<Item = String> {
    (1..=ITEMS).map(|item| format!("Item {item}"))
}
