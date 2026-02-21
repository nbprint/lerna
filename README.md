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

### Hydra Issues Fixed in Lerna

Lerna addresses several long-standing Hydra issues that have been open for years:

#### List Modification from CLI ([#1547](https://github.com/facebookresearch/hydra/issues/1547), [#2477](https://github.com/facebookresearch/hydra/issues/2477))

Lerna adds intuitive, cross-platform list operations:

```bash
# Append items to a list
python app.py 'tags=append(new_tag)'
python app.py 'tags=append(a,b,c)'  # Multiple items

# Prepend items
python app.py 'tags=prepend(first)'

# Insert at specific index
python app.py 'tags=insert(0,first_item)'

# Remove by index
python app.py 'tags=remove_at(0)'      # Remove first
python app.py 'tags=remove_at(-1)'     # Remove last

# Remove by value
python app.py 'tags=remove_value(old_tag)'

# Clear entire list
python app.py 'tags=list_clear()'
```

| Function            | Description            | Example Result         |
| ------------------- | ---------------------- | ---------------------- |
| `append(...)`       | Add items to end       | `[a, b]` → `[a, b, c]` |
| `prepend(...)`      | Add items to beginning | `[b, c]` → `[a, b, c]` |
| `insert(idx, val)`  | Insert at index        | `[a, c]` → `[a, b, c]` |
| `remove_at(idx)`    | Remove by index        | `[a, b, c]` → `[b, c]` |
| `remove_value(val)` | Remove first match     | `[a, b, c]` → `[a, c]` |
| `list_clear()`      | Clear all items        | `[a, b, c]` → `[]`     |

These functions use shell-safe syntax (quote the entire override) and work on bash, zsh, fish, PowerShell, and cmd.

#### No More ANTLR ([#2570](https://github.com/facebookresearch/hydra/issues/2570))

Hydra's ANTLR-based parser breaks when `PYTHONOPTIMIZE=1` or `PYTHONOPTIMIZE=2` is set. Lerna's Rust parser has no Python dependencies and works in all environments.

```bash
# This breaks Hydra but works with Lerna
PYTHONOPTIMIZE=2 python app.py db=postgres
```

#### Default Overrides in Decorator ([#2459](https://github.com/facebookresearch/hydra/issues/2459))

Lerna adds an `overrides` parameter to `@lerna.main()` for setting default overrides that can be overridden from CLI:

```python
@lerna.main(
    config_path="conf",
    config_name="config",
    overrides=["db.driver=postgres", "server.port=8080"]  # Default overrides
)
def my_app(cfg: DictConfig) -> None:
    print(cfg.db.driver)  # "postgres" by default, CLI can override
```

```bash
# Uses decorator defaults
python app.py                        # db.driver=postgres

# CLI overrides take precedence
python app.py db.driver=mysql        # db.driver=mysql
```

#### Instantiate Lookup Without Calling ([#2140](https://github.com/facebookresearch/hydra/issues/2140))

Lerna adds `_call_=False` to `instantiate()` for importing non-callable objects (like `torch.int64`):

```python
from lerna.utils import instantiate
from omegaconf import OmegaConf

# Import a non-callable object directly
cfg = OmegaConf.create({
    "_target_": "torch.int64",
    "_call_": False,  # Don't try to call it
})
dtype = instantiate(cfg)  # Returns torch.int64 directly
```

#### Backward-Compatible Plugin Discovery

Lerna discovers plugins from both `lerna_plugins` and `hydra_plugins` namespaces, enabling gradual migration:

```python
# Both work:
# - lerna_plugins.my_plugin.MyPlugin  (new Lerna plugins)
# - hydra_plugins.my_plugin.MyPlugin  (existing Hydra plugins)
```

#### Subfolder Config Append Fix ([#2935](https://github.com/facebookresearch/hydra/issues/2935))

Hydra incorrectly treats appended defaults as relative paths when the main config is in a subfolder:

```bash
# Hydra bug: this fails because it looks for server/db/postgresql
python app.py --config-name=server/alpha +db@db_2=postgresql

# Lerna: correctly treats appended configs as absolute paths
python app.py --config-name=server/alpha +db@db_2=postgresql  # Works!
```

#### Relative Path in Defaults Fix ([#2878](https://github.com/facebookresearch/hydra/issues/2878))

Hydra produces empty string keys when using `..` in defaults list paths:

```yaml
# Hydra bug with ../dir2 produces config with empty string keys
# Lerna normalizes paths correctly
defaults:
  - ../dir2: child.yaml  # Now works correctly
```

#### importlib-resources 6.2+ Compatibility ([#2870](https://github.com/facebookresearch/hydra/issues/2870))

Hydra breaks with importlib-resources 6.2+ due to `OrphanPath` objects not having `is_file()`/`is_dir()` methods. Lerna handles this gracefully.

### Plugin Registration Compatible with Hydra

Lerna provides a bridge that allows plugins registered via lerna to work with hydra-core. This enables you to write plugins once and have them work with both frameworks.

#### Registering Plugins via Entry Points

Add your plugin to `pyproject.toml` using the `hydra.lernaplugins` entry point group:

```toml
# For SearchPathPlugin modules:
[project.entry-points."hydra.lernaplugins"]
my-plugin = "my_package.plugin_module"

# For package-style config directories:
[project.entry-points."hydra.lernaplugins"]
my-plugin = "pkg:my_package.hydra"
```

**Module-style entry points** (like `my_package.plugin_module`) are imported and scanned for `SearchPathPlugin` subclasses.

**Package-style entry points** (like `pkg:my_package.hydra`) register config search paths directly.

#### How It Works

When hydra-core is used, lerna's `LernaGenericSearchPathPlugin` (installed in the `hydra_plugins` namespace) discovers all plugins registered under `hydra.lernaplugins` and makes them available to hydra's plugin system.

This enables gradual migration: you can write plugins for lerna and they'll automatically work with existing hydra-core installations.

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
