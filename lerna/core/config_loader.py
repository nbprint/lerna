# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from abc import ABC, abstractmethod
from typing import Any

from omegaconf import DictConfig

from lerna.core.config_search_path import ConfigSearchPath
from lerna.core.object_type import ObjectType
from lerna.plugins.config_source import ConfigSource
from lerna.types import RunMode


class ConfigLoader(ABC):
    """
    Config loader interface
    """

    @abstractmethod
    def load_configuration(
        self,
        config_name: str | None,
        overrides: list[str],
        run_mode: RunMode,
        from_shell: bool = True,
        validate_sweep_overrides: bool = True,
    ) -> DictConfig: ...

    @abstractmethod
    def load_sweep_config(self, master_config: DictConfig, sweep_overrides: list[str]) -> DictConfig: ...

    @abstractmethod
    def get_search_path(self) -> ConfigSearchPath: ...

    @abstractmethod
    def get_sources(self) -> list[ConfigSource]: ...

    @abstractmethod
    def list_groups(self, parent_name: str) -> list[str]: ...

    @abstractmethod
    def get_group_options(
        self,
        group_name: str,
        results_filter: ObjectType | None = ObjectType.CONFIG,
        config_name: str | None = None,
        overrides: list[str] | None = None,
    ) -> list[str]: ...

    @abstractmethod
    def compute_defaults_list(
        self,
        config_name: str | None,
        overrides: list[str],
        run_mode: RunMode,
    ) -> Any: ...
