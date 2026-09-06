# Certy

A lightweight, low-memory coding IDE written in Rust. 

> Certy is currently under active development. Current builds are targeted and tested specifically for **Fedora Linux**.

## Features

* Create new files and folders, open files and folders, save changes, close folders, and exit the IDE.
* Browse the files and folders of the currently opened directory from the sidebar.
* Open and work with multiple files at the same time in the editor.
* Common keyboard shortcuts like `Ctrl+X`, `Ctrl+C`, `Ctrl+V`, `Ctrl+Z`, `Ctrl+Y`, `Ctrl+A`, and `Ctrl+S`.
* Restart the IDE with the same folder and previously opened files from the last session.
* Show a confirmation modal for unsaved modified files with options to **Save**, **Discard**, or **Cancel** before closing or exiting.

## Prerequisites

### 1. Install C Development Tools

```bash
sudo dnf group install c-development
```

### 2. Install Rust & Cargo

```bash
curl https://sh.rustup.rs -sSf | sh
```

```bash
source "$HOME/.cargo/env"
```

```bash
rustc --version
cargo --version
```

## Running from Source

1. **Clone the repository:**
    ```bash
    git clone https://github.com/lalshubham/certy.git
    cd certy
    ```

2. **Run in development mode:**
    ```bash
    cargo run
    ```

## Contributing

Contributions, bug reports, and performance profiling feedback are welcome. Feel free to open an issue or submit a pull request.