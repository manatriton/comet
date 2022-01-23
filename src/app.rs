use anyhow::Result;
use minigem::{Line, LineKind, Lines, Request};

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;
use url::{ParseError, Url};

pub enum Action {
    PageRequest { link: String, push_history: bool },
}

enum NetworkEvent {
    PageLoaded {
        address: String,
        lines: Vec<Line>,
        push_history: bool,
    },
}

pub struct Page {
    pub lines: Vec<Line>,
    pub link_indices: Vec<usize>,
    pub link_numbers: HashMap<usize, usize>,
    pub highlighted_link: Option<usize>,
}

pub struct History {
    prev: Vec<String>,
    next: Vec<String>,
}

impl History {
    fn new() -> Self {
        Self {
            prev: Vec::new(),
            next: Vec::new(),
        }
    }

    fn push(&mut self, link: String) {
        self.prev.push(link);
        self.next.clear();
    }

    fn prev(&mut self) -> Option<&String> {
        if self.prev.len() > 1 {
            self.next.push(self.prev.pop().unwrap());
            self.prev.last()
        } else {
            None
        }
    }

    #[inline]
    pub fn has_prev(&self) -> bool {
        self.prev.len() > 1
    }

    fn next(&mut self) -> Option<&String> {
        self.next.pop().map(|link| {
            self.prev.push(link);
            self.prev.last().unwrap()
        })
    }

    #[inline]
    pub fn has_next(&self) -> bool {
        !self.next.is_empty()
    }
}

pub struct App {
    pub page: Page,
    pub scroll: u16,
    pub height: u16,
    pub search: String,
    pub address: String,
    pub history: History,
    in_rx: Receiver<NetworkEvent>,
    _in_tx: Sender<NetworkEvent>,
}

fn load_page(url: String, push_history: bool, cb: Sender<NetworkEvent>) {
    thread::spawn(move || {
        eprintln!("sending request");
        let res = Request::new(&url).send().unwrap();
        eprintln!(
            "got response with status: {:?}, meta: {:?}",
            res.status, res.meta
        );
        let lines = Lines::from(res.body);
        let mut buf = Vec::new();

        for line in lines {
            let line = line.unwrap();
            buf.push(line);
        }

        cb.send(NetworkEvent::PageLoaded {
            address: url,
            push_history,
            lines: buf,
        })
        .unwrap();
    });
}

#[allow(clippy::new_without_default)]
impl App {
    pub fn new() -> Self {
        let (tx, rx) = channel();

        Self {
            page: Page {
                lines: vec![],
                link_indices: vec![],
                link_numbers: HashMap::new(),
                highlighted_link: None,
            },
            search: "gemini://gemini.circumlunar.space/".to_string(),
            address: "gemini://gemini.circumlunar.space/".to_string(),
            height: 0,
            scroll: 0,
            history: History::new(),
            in_rx: rx,
            _in_tx: tx,
        }
    }

    pub fn tick(&mut self) -> Result<()> {
        self.drain_events()
    }

    pub fn scroll_down(&mut self) {
        self.scroll += 1;
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn page_forward(&mut self) {
        self.scroll += self.height;
        let max_scroll = self.page.lines.len().saturating_sub(1) as u16;
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }

    pub fn page_backward(&mut self) {
        self.scroll = self.scroll.saturating_sub(self.height);
    }

    pub fn page_next(&mut self) {
        let result = self.history.next().cloned();
        if let Some(link) = result {
            self.dispatch(Action::PageRequest {
                link,
                push_history: false,
            })
        }
    }

    pub fn page_prev(&mut self) {
        let result = self.history.prev().cloned();
        if let Some(link) = result {
            self.dispatch(Action::PageRequest {
                link,
                push_history: false,
            })
        }
    }

    pub fn next_link(&mut self) {
        self.page.highlighted_link = match self.page.highlighted_link {
            Some(index) => Some(if index == self.page.link_indices.len() - 1 {
                0
            } else {
                index + 1
            }),
            None => {
                if !self.page.link_indices.is_empty() {
                    Some(0)
                } else {
                    None
                }
            }
        }
    }

    pub fn previous_link(&mut self) {
        self.page.highlighted_link = match self.page.highlighted_link {
            Some(index) => Some(if index == 0 {
                self.page.link_indices.len() - 1
            } else {
                index - 1
            }),
            None => {
                if !self.page.link_indices.is_empty() {
                    Some(self.page.link_indices.len() - 1)
                } else {
                    None
                }
            }
        }
    }

    #[inline]
    pub fn clear_highlighted(&mut self) {
        self.page.highlighted_link = None;
    }

    #[inline]
    pub fn request_page_from_input(&mut self) {
        self.dispatch(Action::PageRequest {
            link: self.search.clone(),
            push_history: true,
        })
    }

    pub fn request_page_from_selected(&mut self) {
        if let Some(index) = self.page.highlighted_link {
            let line = &self.page.lines[self.page.link_indices[index]];
            let text = line.link().unwrap();
            let result = Url::parse(text);
            let url = match result {
                Ok(url) => url,
                Err(ParseError::RelativeUrlWithoutBase) => {
                    Url::parse(&self.address).unwrap().join(text).unwrap()
                }
                Err(_) => return,
            };

            eprintln!("url requested: {}", url.as_str());

            self.dispatch(Action::PageRequest {
                link: url.as_str().to_string(),
                push_history: true,
            });
        }
    }

    fn drain_events(&mut self) -> Result<()> {
        loop {
            match self.in_rx.try_recv() {
                Ok(event) => {
                    self.handle_event(event);
                }
                Err(TryRecvError::Empty) => break,
                err @ Err(_) => {
                    err?;
                }
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::PageLoaded {
                address,
                lines,
                push_history,
            } => {
                self.page.lines = lines;
                self.page.link_indices = vec![];
                for (i, line) in self.page.lines.iter().enumerate() {
                    if let LineKind::Link = line.kind() {
                        self.page
                            .link_numbers
                            .insert(i, self.page.link_indices.len());
                        self.page.link_indices.push(i);
                    }
                }
                self.page.highlighted_link = None;
                self.scroll = 0;
                self.address = address;
                if push_history {
                    self.history.push(self.address.clone());
                }
            }
        }
    }

    pub fn dispatch(&mut self, action: Action) {
        match action {
            Action::PageRequest { link, push_history } => {
                load_page(link, push_history, self._in_tx.clone())
            }
        }
    }
}
