# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved

# Source of truth for Lerna's version (Hydra-compatible API)
__version__ = "2.0.2"

# Callback support from Rust
from lerna import lerna as _rust, utils
from lerna.errors import MissingConfigException
from lerna.main import main
from lerna.types import TaskFunction

from .compose import compose
from .initialize import initialize, initialize_config_dir, initialize_config_module

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
    "__version__",
    "MissingConfigException",
    "main",
    "utils",
    "TaskFunction",
    "compose",
    "initialize",
    "initialize_config_module",
    "initialize_config_dir",
    "CallbackManager",
    "JobReturn",
    "ConfigResult",
    "RustFileConfigSource",
    "ConfigSourceManager",
    "RustBasicLauncher",
    "LauncherManager",
    "RustBasicSweeper",
    "SweeperManager",
]
