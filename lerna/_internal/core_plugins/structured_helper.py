# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""
Helper functions for Rust to access ConfigStore (structured://) configs.

These callbacks allow Rust's PyHybridConfigRepository to load configs
from Python's ConfigStore singleton.
"""

from typing import Any, Dict, List, Optional

from omegaconf import OmegaConf

from lerna.core.config_store import ConfigStore
from lerna.core.object_type import ObjectType


def load_structured_config(config_path: str) -> Optional[Dict[str, Any]]:
    """
    Load a config from ConfigStore and return as dict.

    Args:
        config_path: The config path (e.g., "db/mysql" or "config")

    Returns:
        Dict representation of the config, or None if not found
    """
    try:
        cs = ConfigStore.instance()
        # Add .yaml suffix if not present (ConfigStore always stores with .yaml)
        path = config_path
        if not path.endswith(".yaml"):
            path = f"{path}.yaml"
        config_node = cs.load(path)
        # Convert DictConfig to plain dict for Rust
        return OmegaConf.to_container(config_node.node, resolve=False)
    except Exception:
        return None


def structured_config_exists(config_path: str) -> bool:
    """
    Check if a config exists in ConfigStore.

    Args:
        config_path: The config path to check

    Returns:
        True if the config exists
    """
    try:
        cs = ConfigStore.instance()
        # Add .yaml suffix if not present
        path = config_path
        if not path.endswith(".yaml"):
            path = f"{path}.yaml"
        obj_type = cs.get_type(path)
        return obj_type == ObjectType.CONFIG
    except Exception:
        return False


def structured_group_exists(group_path: str) -> bool:
    """
    Check if a group exists in ConfigStore.

    Args:
        group_path: The group path to check (e.g., "db" or "")

    Returns:
        True if the group exists
    """
    try:
        cs = ConfigStore.instance()
        if group_path == "":
            # Root always exists if ConfigStore has any content
            return bool(cs.repo)
        obj_type = cs.get_type(group_path)
        return obj_type == ObjectType.GROUP
    except Exception:
        return False


def structured_list_options(group_path: str) -> List[str]:
    """
    List options (configs and subgroups) in a ConfigStore group.

    Args:
        group_path: The group path to list

    Returns:
        List of option names (configs have .yaml suffix, groups don't)
    """
    try:
        cs = ConfigStore.instance()
        return cs.list(group_path)
    except Exception:
        return []


def get_structured_package(config_path: str) -> Optional[str]:
    """
    Get the package for a structured config.

    Args:
        config_path: The config path

    Returns:
        The package string, or None
    """
    try:
        cs = ConfigStore.instance()
        config_node = cs.load(config_path)
        return config_node.package
    except Exception:
        return None
