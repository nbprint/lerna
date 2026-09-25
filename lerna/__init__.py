# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved

# Source of truth for Lerna's version (Hydra-compatible API)
__version__ = "2.2.0"

# Callback support from Rust
from lerna import lerna as _rust, utils
from lerna.errors import MissingConfigException
from lerna.main import main
from lerna.types import TaskFunction

from .compose import compose, compose_with_provenance
from .initialize import initialize, initialize_config_dir, initialize_config_module
from .provenance import (
    CompositionOperation,
    CompositionResult,
    CompositionSource,
    NodeProvenance,
    RemovedItem,
    SelectedDefault,
)

CallbackManager = _rust.CallbackManager
JobReturn = _rust.JobReturn

# ConfigSource support from Rust
ConfigResult = _rust.ConfigResult
RustFileConfigSource = _rust.RustFileConfigSource
ConfigSourceManager = _rust.ConfigSourceManager

# Launcher support from Rust
RustBasicLauncher = _rust.RustBasicLauncher
LauncherManager = _rust.LauncherManager

# Sweeper support from Rust
RustBasicSweeper = _rust.RustBasicSweeper
SweeperManager = _rust.SweeperManager

__all__ = [
    "CallbackManager",
    "CompositionOperation",
    "CompositionResult",
    "CompositionSource",
    "ConfigResult",
    "ConfigSourceManager",
    "JobReturn",
    "LauncherManager",
    "MissingConfigException",
    "NodeProvenance",
    "RemovedItem",
    "RustBasicLauncher",
    "RustBasicSweeper",
    "RustFileConfigSource",
    "SelectedDefault",
    "SweeperManager",
    "TaskFunction",
    "__version__",
    "compose",
    "compose_with_provenance",
    "initialize",
    "initialize_config_dir",
    "initialize_config_module",
    "main",
    "utils",
]
