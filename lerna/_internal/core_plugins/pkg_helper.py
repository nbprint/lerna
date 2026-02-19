# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""
Helper functions for pkg:// protocol support using importlib.resources.

These functions are designed to be passed to RustHybridConfigRepository as callbacks.
"""

from importlib import resources
from typing import Any, Dict, List, Optional

import yaml


def load_pkg_config(module_path: str, config_path: str) -> Optional[Dict[str, Any]]:
    """
    Load a config file from a Python package.

    Args:
        module_path: Python module path (e.g., 'hydra.conf')
        config_path: Relative path to config within the module (e.g., 'defaults/launcher/basic.yaml')

    Returns:
        Parsed YAML as dict, or None if the file doesn't exist
    """
    try:
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
        files = resources.files(module_path)

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
        files = resources.files(module_path)
        if not group_path:
            # Root of the package
            return True
        resource = files.joinpath(group_path)
        return resource.is_dir()
    except (ModuleNotFoundError, FileNotFoundError, TypeError):
        return False


def pkg_list_options(module_path: str, group_path: str) -> List[str]:
    """
    List config options (YAML files) in a package group.

    Args:
        module_path: Python module path (e.g., 'hydra.conf')
        group_path: Relative path to group (e.g., 'defaults/launcher')

    Returns:
        List of option names (without .yaml extension)
    """
    try:
        files = resources.files(module_path)
        if group_path:
            files = files.joinpath(group_path)

        options = []
        for item in files.iterdir():
            if item.is_file():
                name = item.name
                if name.endswith(".yaml") or name.endswith(".yml"):
                    # Remove extension to get option name
                    options.append(name.rsplit(".", 1)[0])

        return sorted(options)
    except (ModuleNotFoundError, FileNotFoundError, TypeError, AttributeError):
        return []
