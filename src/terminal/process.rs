use super::tab::{TerminalLine, TerminalTab};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

pub enum TermMsg {
    Chunk(String),
    Finished,
}

impl TerminalTab {
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
                    self.command_finished = true;
                    self.scroll_col = 0;
                    self.auto_scroll_to_bottom(vis_rows);
                    updated = true;
                }
            }
        }
        updated
    }

    pub fn execute_command(&mut self, vis_rows: usize) -> bool {
        if self.is_running {
            return false;
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
            return false;
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
            return false;
        }

        if cmd == "exit" {
            return true;
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
            return false;
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
        false
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
            self.command_finished = true;

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
            self.command_finished = true;
            self.auto_scroll_to_bottom(vis_rows);
        }
    }

    pub fn kill_process(&mut self) {
        let child_arc = self.running_child.clone();
        let mut lock = child_arc.lock().unwrap();
        if let Some(mut c) = lock.take() {
            let pid = c.id() as i32;
            #[cfg(unix)]
            unsafe {
                kill(-pid, 2);
                std::thread::sleep(std::time::Duration::from_millis(20));
                kill(-pid, 9);
            }
            #[cfg(not(unix))]
            {
                let _ = c.kill();
            }
            let _ = c.wait();
        }
        self.is_running = false;
        self.command_finished = true;
    }
}
