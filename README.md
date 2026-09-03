# Certy

A lightweight, low-memory coding IDE written in Rust. 

> **Status:** Certy is currently under active development. Current builds are targeted and tested specifically for **Fedora Linux**.

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