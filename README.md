# Lerna

A high-performance configuration framework for Python applications, built with Rust.

Lerna is a rewrite of Facebook's [Hydra](https://github.com/facebookresearch/hydra) configuration framework. It provides the same powerful API with significantly improved performance through a Rust core.

[![Build Status](https://github.com/nbprint/lerna/actions/workflows/build.yaml/badge.svg?branch=main&event=push)](https://github.com/nbprint/lerna/actions/workflows/build.yaml)
[![codecov](https://codecov.io/gh/nbprint/lerna/branch/main/graph/badge.svg)](https://codecov.io/gh/nbprint/lerna)
[![License](https://img.shields.io/github/license/nbprint/lerna)](https://github.com/nbprint/lerna)
[![PyPI](https://img.shields.io/pypi/v/lerna.svg)](https://pypi.python.org/pypi/lerna)

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

# Extend from another config list
python app.py 'tags=extend(${defaults.tags})'

# Remove by index
python app.py 'tags=pop(0)'      # Remove first
python app.py 'tags=pop(-1)'     # Remove last

# Remove by value
python app.py 'tags=remove(old_tag)'

# Delete a slice (equivalent to del tags[1:3])
python app.py 'tags=delete_slice(1,3)'

# Clear entire list
python app.py 'tags=clear()'
```

| Function                     | Description                             | Example result         |
| ---------------------------- | --------------------------------------- | ---------------------- |
| `append(value)`              | Add value to end                        | `[a, b]` → `[a, b, c]` |
| `extend(${path})`            | Add items from another config list      | `[a]` → `[a, b, c]`    |
| `insert(index, value)`       | Insert value at index                   | `[a, c]` → `[a, b, c]` |
| `pop(index)`                 | Remove item at index                    | `[a, b, c]` → `[b, c]` |
| `remove(value)`              | Remove first matching value             | `[a, b, c]` → `[a, c]` |
| `clear()`                    | Remove all items                        | `[a, b, c]` → `[]`     |
| `prepend(...)`               | Add values to beginning                 | `[b, c]` → `[a, b, c]` |
| `append_unique(...)`         | Append values not already present       | `[a, b]` → `[a, b, c]` |
| `remove_all(...)`            | Remove every match                      | `[a, b, a]` → `[b]`    |
| `delete_slice(start, stop?)` | Delete a range using Python slice rules | `[a, b, c]` → `[a]`    |

`remove_at`, `remove_value`, `list_clear`, and `extend_from` remain available as compatibility aliases for `pop`, `remove`, `clear`, and `extend`, respectively.

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

#### Defaults List Patching (`_patch_` directive)

Hydra provides no way to remove or modify specific keys/values inherited from composed configs via the defaults list. Lerna adds a `_patch_` directive that lets you apply override operations to the composed config before CLI overrides are applied.

```yaml
# config.yaml
defaults:
  - some_lib/defaults    # pulls in a library config
  - _self_
  - _patch_:
    - ~unwanted_key                # delete a key
    - ~status=deprecated           # delete key only if value matches
    - items=remove(stale)          # remove a list item by value
    - items=pop(0)                 # remove a list item by index
    - +new_key=injected            # add a new key
    - setting=new_value            # change a value
```

**Key resolution rules:**

| Syntax             | Behavior                                           | Example                                      |
| ------------------ | -------------------------------------------------- | -------------------------------------------- |
| `_patch_:`         | Bare keys auto-prefix with parent config's package | `~drop_me` in `@pkg` config → `~pkg.drop_me` |
| `_patch_@vendor:`  | Bare keys auto-prefix with specified package       | `~debug` → `~vendor.debug`                   |
| `_here_.` prefix   | Explicit relative to parent package                | `_here_.drop_me` → `pkg.drop_me`             |
| `_global_.` prefix | Absolute path from config root                     | `_global_.root_key` → `root_key`             |

For root-level configs (no `@` package), bare keys and `_here_` are equivalent since the parent package is empty.

Patch entries accept the existing override string syntax or a structured mapping. Both forms execute in sequence and use the same package-scoping rules.

| Operation            | String entry                  | Structured entry                                    |
| -------------------- | ----------------------------- | --------------------------------------------------- |
| Change value         | `key=value`                   | `{op: change, path: key, value: value}`             |
| Add key              | `+key=value`                  | `{op: add, path: key, value: value}`                |
| Force-add key        | `++key=value`                 | `{op: force_add, path: key, value: value}`          |
| Delete key           | `~key`                        | `{op: delete, path: key}`                           |
| Conditional delete   | `~key=value`                  | `{op: delete, path: key, value: value}`             |
| List append          | `key=append(a,b)`             | `{op: append, path: key, values: [a, b]}`           |
| List extend          | `key=extend(${source.items})` | `{op: extend, path: key, value: "${source.items}"}` |
| List prepend         | `key=prepend(a,b)`            | `{op: prepend, path: key, values: [a, b]}`          |
| List insert          | `key=insert(1,a,b)`           | `{op: insert, path: key, index: 1, values: [a, b]}` |
| Remove first match   | `key=remove(a)`               | `{op: remove, path: key, value: a}`                 |
| Remove by index      | `key=pop(1)`                  | `{op: pop, path: key, index: 1}`                    |
| Clear list           | `key=clear()`                 | `{op: clear, path: key}`                            |
| Append unique values | `key=append_unique(a,b)`      | `{op: append_unique, path: key, values: [a, b]}`    |
| Remove all matches   | `key=remove_all(a,b)`         | `{op: remove_all, path: key, values: [a, b]}`       |
| Delete slice         | `key=delete_slice(1,3)`       | `{op: delete_slice, path: key, start: 1, stop: 3}`  |

In `delete_slice(start, stop?)`, omitting `stop` deletes from `start` through the end. Negative and out-of-range indexes follow Python slice behavior.

Structured `value` and `values` fields preserve YAML mappings, lists, booleans, nulls, and interpolations without serializing them through the override grammar. A mapping or list in `values` is one item. `extend()` is the operation that splices the selected source list into the destination; nested lists inside the source remain nested.

```yaml
defaults:
  - _patch_@gateway:
    - op: append
      path: modules
      values:
        - /modules/rest
        - /modules/outputs
    - op: change
      path: settings
      value:
        enabled: true
        retries: 3
        label: null
```

`append_unique()` resolves values for structural comparison but preserves the unresolved value when it appends it. Existing duplicates remain. `remove_all()` removes every occurrence of each argument. `extend()` snapshots its source before copying, so extending a list from itself is well-defined.

**Example with packaged config:**

```yaml
# config.yaml — using _patch_@vendor to scope bare keys to the vendor package
defaults:
  - vendor/large_defaults@vendor
  - _self_
  - _patch_@vendor:
    - ~debug_mode           # bare key → targets vendor.debug_mode
    - items=remove(x)       # bare key → targets vendor.items

# Multiple scoped patches can target different packages:
# - _patch_@db:
#   - ~debug
# - _patch_@server:
#   - port=9090
```

**Nested patches:** `_patch_` directives in sub-configs accumulate naturally. If `lib/refined.yaml` has its own `_patch_` that removes `beta`, and your root config adds `_patch_@lib:` to remove `gamma`, both patches apply — `beta` and `gamma` are both removed from the final config.

#### Composition provenance

`compose_with_provenance()` returns a `CompositionResult` containing the unresolved composed config and its composition metadata. Consumers such as [csp-gateway](https://github.com/Point72/csp-gateway) and [ccflow](https://github.com/Point72/ccflow) can build configuration explanations without importing from `lerna._internal`, patching YAML loaders, or disabling Rust YAML parsing.

```python
from lerna import compose_with_provenance, initialize_config_dir

with initialize_config_dir(config_dir="conf"):
  result = compose_with_provenance("config", overrides=["gateway.modules=append(metrics)"])

print(result.config)                         # unresolved DictConfig
print(result.resolved_copy())                # independently resolved copy
print(result.selected_defaults)              # composition order and sources
print(result.available_options["gateway"])  # options seen for the group
print(result.provenance("gateway.modules[0]"))
print(result.patch_operations)               # config-authored operations
print(result.cli_overrides)                  # command-line operations
```

Node provenance includes the source identifier, source config, package, source key, origin, and operation index. List-item provenance follows surviving items as list operations move their indexes. Patch history retains removed items and their provenance.

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

# If only using lerna, you can also register under lerna.plugins:
[project.entry-points."lerna.plugins"]
my-plugin = "my_package.plugin_module"
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
