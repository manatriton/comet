use crossterm::event::{Event as CrosstermEvent, KeyEvent};
use std::sync::mpsc::{channel, Receiver, RecvError, Sender};
use std::thread;
use std::time::Duration;

pub enum Event {
    Input(KeyEvent),
    Tick,
}

pub struct Events {
    rx: Receiver<Event>,
    _tx: Sender<Event>,
}

impl Events {
    pub fn new(timeout: Duration) -> Self {
        let (tx, rx) = channel();
        let tx2 = tx.clone();

        thread::spawn(move || loop {
            if crossterm::event::poll(timeout).unwrap() {
                if let CrosstermEvent::Key(event) = crossterm::event::read().unwrap() {
                    tx2.send(Event::Input(event)).unwrap();
                }
            }
            tx2.send(Event::Tick).unwrap();
        });

        Events { rx, _tx: tx }
    }

    pub fn recv(&mut self) -> Result<Event, RecvError> {
        self.rx.recv()
    }
}
