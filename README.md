Certy is a lightweight and low-memory coding IDE written in Rust.<br />
**Memory usage:** ~10 MB<br />
**Binary size:** ~7 MB

> Certy is currently under active development. Current builds are targeted and tested specifically for **Fedora Linux**.

## Features

* New File and Folder, Open File and Folder, Close Folder, Save and Exit.
* Browse the files and folders from the sidebar.
* Open and work with multiple files at the same time in the editor.
* Create and manage multiple terminals.
* Ctrl+X, Ctrl+C, Ctrl+V, Ctrl+Z, Ctrl+Y, Ctrl+A and Ctrl+S.
* Save, Discard, or Cancel before closing files and exit.
* Restore previous session- files, foders and unsaved changes.

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

Bug reports and contributions are welcome. Feel free to open an issue or create a pull request.