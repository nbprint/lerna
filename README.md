# Lerna

A high-performance configuration framework for Python applications, built with Rust.

Lerna is a rewrite of Facebook's [Hydra](https://github.com/facebookresearch/hydra) configuration framework. It provides the same powerful API with significantly improved performance through a Rust core.

## Features

- **Same Hydra API**: Drop-in replacement for Hydra - just change `import hydra` to `import lerna`
- **Rust-powered**: Core config parsing and loading implemented in Rust via PyO3
- **Full Compatibility**: 2,854 tests passing, nearly 100% Hydra compatibility
- **No ANTLR**: Override parser completely rewritten in Rust (~2,400 LOC removed)
- **Zero Warnings**: Clean Rust codebase with no compiler warnings
- **Extension Points**: Rust traits for Callback, ConfigSource, Launcher, and Sweeper with Python interoperability

## Installation

```bash
pip install lerna
```

## Quick Start

```python
import lerna
from omegaconf import DictConfig

@lerna.main(config_path="conf", config_name="config")
def my_app(cfg: DictConfig) -> None:
    print(cfg.db.driver)
    print(cfg.db.user)

if __name__ == "__main__":
    my_app()
```

## Migration from Hydra

Lerna is a **drop-in replacement** for Hydra. To migrate:

### 1. Change Imports

```python
# Before (Hydra)
import hydra
from hydra import compose, initialize
from hydra.core.config_store import ConfigStore

# After (Lerna)
import lerna
from lerna import compose, initialize
from lerna.core.config_store import ConfigStore
```

### 2. That's It!

All your existing configs, overrides, and patterns work unchanged:

```bash
# Same CLI interface
python my_app.py db=postgres server.port=8080

# Same multirun syntax
python my_app.py -m db=mysql,postgres server.port=8080,8081

# Same sweep functions
python my_app.py -m learning_rate=interval(0.001,0.1) batch_size=choice(16,32,64)
```

## Compatibility Notes

### What Works Identically (100%)

| Feature                                                    | Status                          |
| ---------------------------------------------------------- | ------------------------------- |
| `@lerna.main()` decorator                                  | ✅ Identical to `@hydra.main()` |
| `compose()` API                                            | ✅ Same signature and behavior  |
| `initialize()` / `initialize_config_dir()`                 | ✅ Same API                     |
| Config composition with defaults                           | ✅ Full support                 |
| Override syntax (`key=value`, `+key`, `~key`, `key@pkg`)   | ✅ All syntax supported         |
| Sweep functions (`choice`, `range`, `interval`, `glob`)    | ✅ Full support                 |
| Cast functions (`int`, `float`, `str`, `bool`, `json_str`) | ✅ Full support                 |
| Modifiers (`shuffle`, `sort`, `tag`, `extend_list`)        | ✅ Full support                 |
| Structured configs (dataclasses)                           | ✅ Full support                 |
| Package directives (`@package`)                            | ✅ Full support                 |
| Interpolations (`${key}`, `${oc.env:VAR}`)                 | ✅ Via OmegaConf                |
| ConfigStore                                                | ✅ Full support                 |
| Shell completion (bash, zsh, fish)                         | ✅ Full support                 |

### Known Differences (17 edge cases)

| Difference                    | Impact   | Workaround                                      |
| ----------------------------- | -------- | ----------------------------------------------- |
| Zsh tilde completion          | 16 tests | Use full paths instead of `~` in zsh completion |
| Multirun completion edge case | 1 test   | Minor CLI completion limitation                 |

These are shell-specific completion behaviors, not functional differences.

### Third-Party Plugins

Hydra's plugin ecosystem (Optuna, Ray, Submitit, etc.) references `hydra` internally. To use them with Lerna:

```python
# Option 1: Import aliasing (recommended)
import lerna as hydra  # Alias for plugin compatibility

# Option 2: Use Lerna's built-in extensions
from lerna import RustBasicLauncher, RustBasicSweeper
```

### Dependencies

Lerna requires OmegaConf (same as Hydra):

```bash
pip install lerna omegaconf
```

## Performance

| Operation            | Hydra    | Lerna | Speedup |
| -------------------- | -------- | ----- | ------- |
| YAML parsing         | 240μs    | 6.5μs | **37x** |
| Config composition   | 18,826μs | 929μs | **20x** |
| Config load (cached) | -        | 2.0μs | -       |

## Key Components

### Override Parser (Rust)

The override parser is fully implemented in Rust with support for:

- All sweep types: `choice()`, `range()`, `interval()`, `glob()`
- Cast functions: `int()`, `float()`, `str()`, `bool()`, `json_str()`
- Modifiers: `shuffle()`, `sort()`, `tag()`, `extend_list()`
- User-defined functions via Python callbacks (with proper shadowing)
- Complex nested structures and interpolations

### Config Loading (Rust + Python)

- High-performance YAML parsing in Rust
- Defaults list processing with proper package resolution
- Config merging and override application
- Full interpolation support via OmegaConf

### Job Runner (Rust)

- Job context management
- Output directory computation and creation
- Config/override file serialization

### Extension Points (Rust + Python)

Pluggable architecture allowing both Rust and Python implementations:

- **Callback**: Lifecycle hooks (`on_job_start`, `on_job_end`, `on_run_start`, etc.)
- **ConfigSource**: Config loading from file://, pkg://, structured:// sources
- **Launcher**: Job execution orchestration (BasicLauncher included)
- **Sweeper**: Parameter sweep strategies (BasicSweeper with cartesian product included)

## Architecture

```
lerna/
├── lerna/              # Python package (Hydra API)
├── rust/               # Pure Rust core library (no Python deps)
│   └── src/
│       ├── parser/     # Override parser (2,800 LOC)
│       ├── config/     # Config loading
│       ├── omegaconf/  # OmegaConf compatibility
│       └── ...
└── src/                # PyO3 bindings
```

## Test Status

| Component        | Tests | Status                 |
| ---------------- | ----- | ---------------------- |
| Full Suite       | 2,854 | ✅ Passing             |
| Parser           | 515   | ✅ Passing (0 xfailed) |
| Rust Core        | 229   | ✅ Passing             |
| Extension Points | 65    | ✅ Passing             |

## Remaining Xfails (17)

All remaining xfails are known shell-specific limitations, not bugs:

- 16 zsh completion tests (tilde handling in shells)
- 1 multirun completion test (partial override parsing)

## Development

```bash
# Build Rust extension
make develop

# Run tests
make test
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

This project is based on [Hydra](https://github.com/facebookresearch/hydra) by Facebook Research.
