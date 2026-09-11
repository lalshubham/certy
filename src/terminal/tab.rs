use super::screen::{TerminalRow, TerminalScreen};
use portable_pty::{CommandBuilder, MasterPty, PtySize};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

pub fn detect_shell() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Ok(shell) = std::env::var("SHELL") {
            return shell;
        }
        if std::env::var_os("PSModulePath").is_some() {
            return "powershell.exe".to_string();
        }
        "cmd.exe".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    }
}

pub struct TerminalTab {
    pub name: String,
    pub cwd: PathBuf,
    pub screen: TerminalScreen,
    pub scroll_line: usize,
    pub selection_anchor: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
    output_rx: Receiver<Vec<u8>>,
}

impl TerminalTab {
    pub fn new(name: String, cwd: PathBuf, rows: usize, cols: usize) -> Self {
        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: rows.max(1) as u16,
                cols: cols.max(1) as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to create pty");
        let shell = detect_shell();
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(&cwd);
        #[cfg(not(target_os = "windows"))]
        cmd.arg("-l");
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        let child = pair
            .slave
            .spawn_command(cmd)
            .expect("Failed to spawn shell");
        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("Failed to clone pty reader");
        let writer = pair
            .master
            .take_writer()
            .expect("Failed to take pty writer");
        let (tx, rx) = channel();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        Self {
            name,
            cwd,
            screen: TerminalScreen::new(rows, cols),
            scroll_line: 0,
            selection_anchor: None,
            selection_end: None,
            master: pair.master,
            writer: Arc::new(Mutex::new(writer)),
            child: Arc::new(Mutex::new(child)),
            output_rx: rx,
        }
    }

    #[inline]
    pub fn width(&self, char_w: usize) -> usize {
        self.name.chars().count() * char_w + 50
    }

    pub fn resize_pty(&mut self, rows: usize, cols: usize) {
        self.screen.set_size(rows, cols);
        let _ = self.master.resize(PtySize {
            rows: rows.max(1) as u16,
            cols: cols.max(1) as u16,
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    pub fn write_bytes(&self, bytes: &[u8]) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = w.write_all(bytes);
            let _ = w.flush();
        }
    }

    pub fn kill_process(&mut self) {
        if let Ok(mut c) = self.child.lock() {
            let _ = c.kill();
        }
    }

    pub fn is_running(&self) -> bool {
        if let Ok(mut c) = self.child.lock() {
            matches!(c.try_wait(), Ok(None))
        } else {
            false
        }
    }

    pub fn poll_output(&mut self, vis_rows: usize) -> bool {
        let mut updated = false;
        while let Ok(chunk) = self.output_rx.try_recv() {
            self.screen.process_bytes(&chunk);
            updated = true;
        }
        if updated {
            let total = self.screen.total_lines();
            self.scroll_line = total.saturating_sub(vis_rows);
        }
        updated
    }

    pub fn total_lines(&self) -> usize {
        self.screen.total_lines()
    }

    pub fn get_row(&self, idx: usize) -> Option<&TerminalRow> {
        self.screen.get_display_row(idx)
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
        let ((s_line, s_col), (e_line, e_col)) = self.selection_range()?;
        let mut result = Vec::with_capacity(e_line.saturating_sub(s_line) + 1);
        for line_idx in s_line..=e_line {
            if let Some(row) = self.get_row(line_idx) {
                let cell_len = row.cells.len();
                let start = if line_idx == s_line {
                    s_col.min(cell_len)
                } else {
                    0
                };
                let end = if line_idx == e_line {
                    e_col.min(cell_len)
                } else {
                    cell_len
                };
                if start <= end {
                    let mut s: String = row.cells[start..end].iter().map(|c| c.ch).collect();
                    if line_idx != e_line {
                        let trimmed_len = s.trim_end().len();
                        s.truncate(trimmed_len);
                    }
                    result.push(s);
                } else {
                    result.push(String::new());
                }
            }
        }
        Some(result.join("\n"))
    }
}
