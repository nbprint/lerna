# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""
Helper functions for pkg:// protocol support using importlib.resources.

These functions are designed to be passed to RustHybridConfigRepository as callbacks.
"""

from __future__ import annotations

import importlib.util
import os
import sys
from importlib import resources
from importlib.machinery import all_suffixes
from pathlib import Path
from typing import TYPE_CHECKING, Any

import yaml

if TYPE_CHECKING:
    from importlib.abc import Traversable


def _has_init(path: str) -> bool:
    return any(os.path.isfile(os.path.join(path, "__init__" + suffix)) for suffix in all_suffixes())


def find_package_dir(module_path: str) -> Path | None:
    """
    Locate a regular package's directory without importing it or its parents.

    importlib.resources.files() imports the package, which runs every parent
    ``__init__`` along the way. Plugin search paths are resolved on every startup,
    so walk the import path instead, following the path finder's order: a regular
    package wins, then a module, then namespace portions. Returns None whenever
    the walk can't give the same answer as an import (zip imports, namespace or
    already-imported packages, modules shadowing packages); callers then fall back
    to importlib.
    """
    parts = module_path.split(".")
    if module_path in sys.modules or not all(parts):
        return None
    try:
        spec = importlib.util.find_spec(parts[0])
    except (ImportError, ValueError):
        return None
    if spec is None or spec.submodule_search_locations is None:
        return None
    locations = list(spec.submodule_search_locations)
    for depth, part in enumerate(parts[1:], start=1):
        parent = sys.modules.get(".".join(parts[:depth]))
        if parent is not None:
            locations = list(getattr(parent, "__path__", []))
        portions = []
        for location in locations:
            candidate = os.path.join(location, part)
            if _has_init(candidate):
                portions = [candidate]
                break
            if any(os.path.isfile(candidate + suffix) for suffix in all_suffixes()):
                return None
            if os.path.isdir(candidate):
                portions.append(candidate)
        if not portions:
            return None
        locations = portions
    if len(locations) != 1 or not _has_init(locations[0]):
        return None
    return Path(locations[0])


def pkg_files(module_path: str) -> Traversable:
    """Return ``importlib.resources.files(module_path)``, without importing the package when possible."""
    return find_package_dir(module_path) or resources.files(module_path)


def load_pkg_config(module_path: str, config_path: str) -> dict[str, Any] | None:
    """
    Load a config file from a Python package.

    Args:
        module_path: Python module path (e.g., 'hydra.conf')
        config_path: Relative path to config within the module (e.g., 'defaults/launcher/basic.yaml')

    Returns:
        Parsed YAML as dict, or None if the file doesn't exist
    """
    try:
        # Reading a config imports the package as before, so its import side effects still run
        files = resources.files(module_path)

        # Normalize path - add .yaml extension if needed
        path = config_path
        if not path.endswith(".yaml") and not path.endswith(".yml"):
            path = f"{path}.yaml"

        file_resource = files.joinpath(path)

        if not file_resource.is_file():
            return None

        content = file_resource.read_text(encoding="utf-8")
        if not content.strip():
            return {}
        return yaml.safe_load(content)
    except (ModuleNotFoundError, FileNotFoundError, TypeError):
        return None


def pkg_config_exists(module_path: str, config_path: str) -> bool:
    """
    Check if a config file exists in a Python package.

    Args:
        module_path: Python module path (e.g., 'hydra.conf')
        config_path: Relative path to config (e.g., 'defaults/launcher/basic.yaml')

    Returns:
        True if the file exists, False otherwise
    """
    try:
        files = pkg_files(module_path)

        # Normalize path - add .yaml extension if needed
        path = config_path
        if not path.endswith(".yaml") and not path.endswith(".yml"):
            path = f"{path}.yaml"

        resource = files.joinpath(path)
        return resource.is_file()
    except (ModuleNotFoundError, FileNotFoundError, TypeError):
        return False


def pkg_group_exists(module_path: str, group_path: str) -> bool:
    """
    Check if a config group (directory) exists in a Python package.

    Args:
        module_path: Python module path (e.g., 'hydra.conf')
        group_path: Relative path to group (e.g., 'defaults/launcher')

    Returns:
        True if the directory exists, False otherwise
    """
    try:
        files = pkg_files(module_path)
        if not group_path:
            # Root of the package
            return True
        resource = files.joinpath(group_path)
        return resource.is_dir()
    except (ModuleNotFoundError, FileNotFoundError, TypeError):
        return False


def pkg_list_options(module_path: str, group_path: str) -> list[str]:
    """
    List config options (YAML files) in a package group.

    Args:
        module_path: Python module path (e.g., 'hydra.conf')
        group_path: Relative path to group (e.g., 'defaults/launcher')

    Returns:
        List of option names (without .yaml extension)
    """
    try:
        files = pkg_files(module_path)
        if group_path:
            files = files.joinpath(group_path)

        options = []
        for item in files.iterdir():
            if item.is_file():
                name = item.name
                if name.endswith((".yaml", ".yml")):
                    # Remove extension to get option name
                    options.append(name.rsplit(".", 1)[0])

        return sorted(options)
    except (ModuleNotFoundError, FileNotFoundError, TypeError, AttributeError):
        return []
