use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub enum TermMsg {
    Chunk(String),
    Finished,
}

pub struct Terminal {
    pub is_open: bool,
    pub height: usize,
    pub focused: bool,
    pub lines: Vec<String>,
    pub partial_line: String,
    pub current_input: String,
    pub cursor_col: usize,
    pub cwd: PathBuf,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub scroll_line: usize,
    pub scroll_col: usize,
    pub max_line_len: usize,
    pub is_running: bool,
    pub hovered_close: bool,
    output_rx: Receiver<TermMsg>,
    output_tx: Sender<TermMsg>,
    running_child: Arc<Mutex<Option<Child>>>,
}

impl Terminal {
    pub fn new() -> Self {
        let (output_tx, output_rx) = channel();
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            is_open: false,
            height: 220,
            focused: false,
            lines: Vec::new(),
            partial_line: String::new(),
            current_input: String::new(),
            cursor_col: 0,
            cwd,
            history: Vec::new(),
            history_idx: None,
            scroll_line: 0,
            scroll_col: 0,
            max_line_len: 0,
            is_running: false,
            hovered_close: false,
            output_rx,
            output_tx,
            running_child: Arc::new(Mutex::new(None)),
        }
    }

    pub fn prompt(&self) -> String {
        let folder = self.cwd.file_name().and_then(|n| n.to_str()).unwrap_or("~");
        format!("{folder}$ ")
    }

    pub fn total_lines(&self) -> usize {
        let mut count = self.lines.len();
        if !self.partial_line.is_empty() {
            count += 1;
        }
        if !self.is_running {
            count += 1;
        }
        count
    }

    pub fn vis_rows(&self, line_h: usize) -> usize {
        if line_h == 0 {
            return 0;
        }
        let header_h = 26;
        let body_h = self.height.saturating_sub(header_h);
        let track_h = body_h.saturating_sub(crate::config::SCROLLBAR_THICKNESS);
        track_h.saturating_sub(4) / line_h
    }

    pub fn max_content_cols(&self) -> usize {
        let mut max_c = self.max_line_len;
        if !self.is_running {
            let active_len = self.prompt().chars().count() + self.current_input.chars().count();
            if active_len > max_c {
                max_c = active_len;
            }
        }
        if !self.partial_line.is_empty() {
            let partial_len = self.partial_line.chars().count();
            if partial_len > max_c {
                max_c = partial_len;
            }
        }
        max_c
    }

    pub fn ensure_cursor_visible(&mut self, vis_cols: usize) {
        if vis_cols == 0 {
            return;
        }
        let active_col = self.prompt().chars().count() + self.cursor_col;
        if active_col >= self.scroll_col + vis_cols {
            self.scroll_col = active_col - vis_cols + 1;
        } else if active_col < self.scroll_col {
            self.scroll_col = active_col;
        }
    }

    pub fn auto_scroll_to_bottom(&mut self, vis_rows: usize) {
        let total = self.total_lines();
        if total > vis_rows {
            self.scroll_line = total - vis_rows;
        } else {
            self.scroll_line = 0;
        }
    }

    pub fn poll_output(&mut self, vis_rows: usize) -> bool {
        let mut updated = false;
        while let Ok(msg) = self.output_rx.try_recv() {
            match msg {
                TermMsg::Chunk(s) => {
                    for ch in s.chars() {
                        if ch == '\n' {
                            let line = std::mem::take(&mut self.partial_line);
                            if line.chars().count() > self.max_line_len {
                                self.max_line_len = line.chars().count();
                            }
                            self.lines.push(line);
                        } else if ch != '\r' {
                            self.partial_line.push(ch);
                        }
                    }
                    if self.partial_line.chars().count() > self.max_line_len {
                        self.max_line_len = self.partial_line.chars().count();
                    }
                    self.auto_scroll_to_bottom(vis_rows);
                    updated = true;
                }
                TermMsg::Finished => {
                    if !self.partial_line.is_empty() {
                        let line = std::mem::take(&mut self.partial_line);
                        if line.chars().count() > self.max_line_len {
                            self.max_line_len = line.chars().count();
                        }
                        self.lines.push(line);
                    }
                    self.is_running = false;
                    self.auto_scroll_to_bottom(vis_rows);
                    updated = true;
                }
            }
        }
        updated
    }

    pub fn execute_command(&mut self, vis_rows: usize) {
        if self.is_running {
            return;
        }

        if !self.partial_line.is_empty() {
            let line = std::mem::take(&mut self.partial_line);
            if line.chars().count() > self.max_line_len {
                self.max_line_len = line.chars().count();
            }
            self.lines.push(line);
        }

        let cmd = self.current_input.trim().to_string();
        let p = self.prompt();
        let cmd_line = format!("{p}{}", self.current_input);
        if cmd_line.chars().count() > self.max_line_len {
            self.max_line_len = cmd_line.chars().count();
        }
        self.lines.push(cmd_line);
        self.current_input.clear();
        self.cursor_col = 0;
        self.scroll_col = 0;
        self.auto_scroll_to_bottom(vis_rows);

        if cmd.is_empty() {
            return;
        }

        if self.history.last() != Some(&cmd) {
            self.history.push(cmd.clone());
        }
        self.history_idx = None;

        if cmd == "clear" || cmd == "cls" {
            self.lines.clear();
            self.partial_line.clear();
            self.max_line_len = 0;
            self.scroll_line = 0;
            self.scroll_col = 0;
            return;
        }

        if cmd == "exit" {
            self.is_open = false;
            self.focused = false;
            return;
        }

        if cmd.starts_with("cd ") || cmd == "cd" {
            let target = if cmd == "cd" {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| self.cwd.clone())
            } else {
                let path_str = cmd[3..].trim();
                if path_str == "~" {
                    std::env::var_os("HOME")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| self.cwd.clone())
                } else {
                    self.cwd.join(path_str)
                }
            };

            if let Ok(canon) = std::fs::canonicalize(&target) {
                if canon.is_dir() {
                    self.cwd = canon;
                } else {
                    self.lines
                        .push(format!("cd: not a directory: {}", target.display()));
                }
            } else {
                self.lines.push(format!(
                    "cd: no such file or directory: {}",
                    target.display()
                ));
            }
            self.auto_scroll_to_bottom(vis_rows);
            return;
        }

        self.is_running = true;
        let tx = self.output_tx.clone();
        let cwd = self.cwd.clone();
        let child_holder = self.running_child.clone();
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        thread::spawn(move || {
            let child_res = Command::new(&shell)
                .arg("-c")
                .arg(&cmd)
                .current_dir(&cwd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();

            match child_res {
                Ok(mut child) => {
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();

                    {
                        let mut lock = child_holder.lock().unwrap();
                        *lock = Some(child);
                    }

                    let tx_out = tx.clone();
                    let h1 = thread::spawn(move || {
                        if let Some(mut out) = stdout {
                            let mut buf = [0u8; 1024];
                            while let Ok(n) = out.read(&mut buf) {
                                if n == 0 {
                                    break;
                                }
                                let s = String::from_utf8_lossy(&buf[..n]).to_string();
                                let _ = tx_out.send(TermMsg::Chunk(s));
                            }
                        }
                    });

                    let tx_err = tx.clone();
                    let h2 = thread::spawn(move || {
                        if let Some(mut err) = stderr {
                            let mut buf = [0u8; 1024];
                            while let Ok(n) = err.read(&mut buf) {
                                if n == 0 {
                                    break;
                                }
                                let s = String::from_utf8_lossy(&buf[..n]).to_string();
                                let _ = tx_err.send(TermMsg::Chunk(s));
                            }
                        }
                    });

                    let _ = h1.join();
                    let _ = h2.join();

                    let mut lock = child_holder.lock().unwrap();
                    if let Some(mut c) = lock.take() {
                        let _ = c.wait();
                    }
                    let _ = tx.send(TermMsg::Finished);
                }
                Err(e) => {
                    let _ = tx.send(TermMsg::Chunk(format!("Error starting process: {e}\n")));
                    let _ = tx.send(TermMsg::Finished);
                }
            }
        });
    }

    pub fn interrupt(&mut self, vis_rows: usize) {
        let child_arc = self.running_child.clone();
        let killed = {
            let mut lock = child_arc.lock().unwrap();
            if let Some(mut c) = lock.take() {
                let _ = c.kill();
                let _ = c.wait();
                true
            } else {
                false
            }
        };

        if killed {
            self.lines.push("^C".to_string());
            self.is_running = false;
            self.auto_scroll_to_bottom(vis_rows);
        } else if !self.current_input.is_empty() {
            let p = self.prompt();
            self.lines.push(format!("{p}{}^C", self.current_input));
            self.current_input.clear();
            self.cursor_col = 0;
            self.auto_scroll_to_bottom(vis_rows);
        }
    }

    pub fn history_up(&mut self) {
        if !self.history.is_empty() {
            let next = match self.history_idx {
                Some(i) if i > 0 => i - 1,
                Some(_) => 0,
                None => self.history.len().saturating_sub(1),
            };
            self.history_idx = Some(next);
            self.current_input = self.history[next].clone();
            self.cursor_col = self.current_input.chars().count();
        }
    }

    pub fn history_down(&mut self) {
        if let Some(i) = self.history_idx {
            if i + 1 < self.history.len() {
                self.history_idx = Some(i + 1);
                self.current_input = self.history[i + 1].clone();
            } else {
                self.history_idx = None;
                self.current_input.clear();
            }
            self.cursor_col = self.current_input.chars().count();
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        let mut chars: Vec<char> = self.current_input.chars().collect();
        chars.insert(self.cursor_col, ch);
        self.current_input = chars.into_iter().collect();
        self.cursor_col += 1;
    }

    pub fn delete_backwards(&mut self) {
        if self.cursor_col > 0 {
            let mut chars: Vec<char> = self.current_input.chars().collect();
            chars.remove(self.cursor_col - 1);
            self.current_input = chars.into_iter().collect();
            self.cursor_col -= 1;
        }
    }

    pub fn delete_forward(&mut self) {
        let mut chars: Vec<char> = self.current_input.chars().collect();
        if self.cursor_col < chars.len() {
            chars.remove(self.cursor_col);
            self.current_input = chars.into_iter().collect();
        }
    }

    pub fn move_left(&mut self) {
        self.cursor_col = self.cursor_col.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        let len = self.current_input.chars().count();
        if self.cursor_col < len {
            self.cursor_col += 1;
        }
    }
}
