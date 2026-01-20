use anyhow::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent};
use futures::StreamExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::Stderr;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Tick,
    Render,
}

pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<Stderr>>,
    pub task: JoinHandle<()>,
    pub cancellation_token: CancellationToken,
    pub event_rx: UnboundedReceiver<Event>,
    pub event_tx: UnboundedSender<Event>,
}

impl Tui {
    pub fn new() -> Result<Self> {
        // Initialize terminal
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = std::io::stderr();
        crossterm::execute!(
            stdout,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;

        let backend = CrosstermBackend::new(stderr());
        let terminal = Terminal::new(backend)?;

        // Create event channel
        let (event_tx, event_rx) = unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let token_clone = cancellation_token.clone();
        let tx_clone = event_tx.clone();

        // Spawn event handling task
        let task = tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
            let mut render_interval = tokio::time::interval(std::time::Duration::from_millis(16));

            loop {
                tokio::select! {
                    _ = token_clone.cancelled() => break,

                    maybe_event = reader.next() => {
                        if let Some(Ok(CrosstermEvent::Key(key))) = maybe_event {
                            let _ = tx_clone.send(Event::Key(key));
                        }
                    }

                    _ = interval.tick() => {
                        let _ = tx_clone.send(Event::Tick);
                    }

                    _ = render_interval.tick() => {
                        let _ = tx_clone.send(Event::Render);
                    }
                }
            }
        });

        Ok(Tui {
            terminal,
            task,
            cancellation_token,
            event_rx,
            event_tx,
        })
    }

    pub fn enter_alternate_screen(&mut self) -> Result<()> {
        let mut stdout = std::io::stderr();
        crossterm::execute!(
            stdout,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        Ok(())
    }

    pub fn exit_alternate_screen(&mut self) -> Result<()> {
        let mut stdout = std::io::stderr();
        crossterm::execute!(
            stdout,
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;
        Ok(())
    }

    pub fn shutdown(self) -> Result<()> {
        // Cancel event loop
        self.cancellation_token.cancel();

        use std::io::Write;
        let mut stdout = std::io::stderr();

        // Leave alternate screen FIRST (while raw mode is still active)
        crossterm::execute!(
            stdout,
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;
        stdout.flush()?;

        // THEN disable raw mode
        crossterm::terminal::disable_raw_mode()?;

        Ok(())
    }
}

fn stderr() -> Stderr {
    std::io::stderr()
}
