use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

pub enum TermMsg {
    Chunk(String),
    Finished,
}

#[derive(Clone)]
pub struct TerminalLine {
    pub prompt: Option<String>,
    pub content: String,
}

impl TerminalLine {
    pub fn full_text(&self) -> String {
        if let Some(ref p) = self.prompt {
            format!("{p}{}", self.content)
        } else {
            self.content.clone()
        }
    }
}

pub struct Terminal {
    pub is_open: bool,
    pub height: usize,
    pub focused: bool,
    pub lines: Vec<TerminalLine>,
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
    pub selection_anchor: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    output_rx: Receiver<TermMsg>,
    output_tx: Sender<TermMsg>,
    running_child: Arc<Mutex<Option<Child>>>,
}

fn get_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

fn get_hostname() -> String {
    if let Ok(h) = std::fs::read_to_string("/etc/hostname") {
        let trimmed = h.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string())
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
            selection_anchor: None,
            selection_end: None,
            output_rx,
            output_tx,
            running_child: Arc::new(Mutex::new(None)),
        }
    }

    pub fn prompt(&self) -> String {
        let user = get_username();
        let host = get_hostname();
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let path_str = if let Some(ref h) = home {
            if let Ok(rel) = self.cwd.strip_prefix(h) {
                if rel.as_os_str().is_empty() {
                    "~".to_string()
                } else {
                    format!("~/{}", rel.display())
                }
            } else {
                self.cwd.display().to_string()
            }
        } else {
            self.cwd.display().to_string()
        };
        format!("{user}@{host}:{path_str}$ ")
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
        let prompt_len = self.prompt().chars().count();
        let active_col = prompt_len + self.cursor_col;
        let active_line_len = prompt_len + self.current_input.chars().count();

        if active_col < vis_cols {
            self.scroll_col = 0;
        } else {
            if active_col >= self.scroll_col + vis_cols {
                self.scroll_col = active_col.saturating_sub(vis_cols) + 1;
            } else if active_col < self.scroll_col {
                self.scroll_col = active_col;
            }

            if self.cursor_col == self.current_input.chars().count() {
                let ideal_scroll = active_line_len.saturating_sub(vis_cols) + 1;
                if self.scroll_col > ideal_scroll {
                    self.scroll_col = ideal_scroll;
                }
            }
        }

        let max_scroll = self.max_content_cols().saturating_sub(vis_cols);
        if self.scroll_col > max_scroll {
            self.scroll_col = max_scroll;
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

    pub fn get_line_text(&self, idx: usize) -> Option<String> {
        if idx < self.lines.len() {
            Some(self.lines[idx].full_text())
        } else if idx == self.lines.len() && !self.partial_line.is_empty() {
            Some(self.partial_line.clone())
        } else if !self.is_running && idx == self.total_lines().saturating_sub(1) {
            Some(format!("{}{}", self.prompt(), self.current_input))
        } else {
            None
        }
    }

    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        match (self.selection_anchor, self.selection_end) {
            (Some(a), Some(e)) if a != e => {
                if a <= e {
                    Some((a, e))
                } else {
                    Some((e, a))
                }
            }
            _ => None,
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        let ((start_line, start_col), (end_line, end_col)) = self.selection_range()?;
        let mut result = Vec::new();
        for line_idx in start_line..=end_line {
            if let Some(text) = self.get_line_text(line_idx) {
                let chars: Vec<char> = text.chars().collect();
                let s_col = if line_idx == start_line {
                    start_col.min(chars.len())
                } else {
                    0
                };
                let e_col = if line_idx == end_line {
                    end_col.min(chars.len())
                } else {
                    chars.len()
                };
                if s_col <= e_col {
                    let slice: String = chars[s_col..e_col].iter().collect();
                    result.push(slice);
                } else {
                    result.push(String::new());
                }
            }
        }
        Some(result.join("\n"))
    }

    pub fn poll_output(&mut self, vis_rows: usize) -> bool {
        let mut updated = false;
        while let Ok(msg) = self.output_rx.try_recv() {
            match msg {
                TermMsg::Chunk(s) => {
                    if !self.is_running {
                        continue;
                    }
                    for ch in s.chars() {
                        if ch == '\n' {
                            let line = std::mem::take(&mut self.partial_line);
                            if line.chars().count() > self.max_line_len {
                                self.max_line_len = line.chars().count();
                            }
                            self.lines.push(TerminalLine {
                                prompt: None,
                                content: line,
                            });
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
                    if !self.is_running {
                        continue;
                    }
                    if !self.partial_line.is_empty() {
                        let line = std::mem::take(&mut self.partial_line);
                        if line.chars().count() > self.max_line_len {
                            self.max_line_len = line.chars().count();
                        }
                        self.lines.push(TerminalLine {
                            prompt: None,
                            content: line,
                        });
                    }
                    self.is_running = false;
                    self.scroll_col = 0;
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
            self.lines.push(TerminalLine {
                prompt: None,
                content: line,
            });
        }

        let cmd = self.current_input.trim().to_string();
        let p = self.prompt();
        let cmd_line_len = p.chars().count() + self.current_input.chars().count();
        if cmd_line_len > self.max_line_len {
            self.max_line_len = cmd_line_len;
        }
        self.lines.push(TerminalLine {
            prompt: Some(p),
            content: self.current_input.clone(),
        });
        self.current_input.clear();
        self.cursor_col = 0;
        self.scroll_col = 0;
        self.selection_anchor = None;
        self.selection_end = None;
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
                    self.lines.push(TerminalLine {
                        prompt: None,
                        content: format!("cd: not a directory: {}", target.display()),
                    });
                }
            } else {
                self.lines.push(TerminalLine {
                    prompt: None,
                    content: format!("cd: no such file or directory: {}", target.display()),
                });
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
            let mut cmd_builder = Command::new(&shell);
            cmd_builder
                .arg("-c")
                .arg(&cmd)
                .current_dir(&cwd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                cmd_builder.process_group(0);
            }

            let child_res = cmd_builder.spawn();

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
        let child_opt = {
            let mut lock = child_arc.lock().unwrap();
            lock.take()
        };

        if let Some(mut c) = child_opt {
            let pid = c.id() as i32;
            #[cfg(unix)]
            unsafe {
                kill(-pid, 2);
                std::thread::sleep(std::time::Duration::from_millis(50));
                kill(-pid, 9);
            }
            #[cfg(not(unix))]
            {
                let _ = c.kill();
            }
            let _ = c.wait();

            self.is_running = false;

            while let Ok(msg) = self.output_rx.try_recv() {
                if let TermMsg::Chunk(s) = msg {
                    for ch in s.chars() {
                        if ch == '\n' {
                            let line = std::mem::take(&mut self.partial_line);
                            if line.chars().count() > self.max_line_len {
                                self.max_line_len = line.chars().count();
                            }
                            self.lines.push(TerminalLine {
                                prompt: None,
                                content: line,
                            });
                        } else if ch != '\r' {
                            self.partial_line.push(ch);
                        }
                    }
                }
            }

            if !self.partial_line.is_empty() {
                let line = std::mem::take(&mut self.partial_line);
                if line.chars().count() > self.max_line_len {
                    self.max_line_len = line.chars().count();
                }
                self.lines.push(TerminalLine {
                    prompt: None,
                    content: line,
                });
            }

            self.lines.push(TerminalLine {
                prompt: None,
                content: "^C".to_string(),
            });
            self.auto_scroll_to_bottom(vis_rows);
        } else if !self.current_input.is_empty() {
            let p = self.prompt();
            self.lines.push(TerminalLine {
                prompt: Some(p),
                content: format!("{}^C", self.current_input),
            });
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
