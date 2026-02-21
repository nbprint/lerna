# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import os
from typing import List, Optional

import yaml
from omegaconf import OmegaConf

from lerna.core.object_type import ObjectType
from lerna.plugins.config_source import ConfigLoadError, ConfigResult, ConfigSource

# Try to import Rust module for fast YAML parsing
try:
    import lerna.lerna as _rs

    _RUST_AVAILABLE = True
except ImportError:
    _RUST_AVAILABLE = False


class FileConfigSource(ConfigSource):
    def __init__(self, provider: str, path: str) -> None:
        if path.find("://") == -1:
            path = f"{self.scheme()}://{path}"
        super().__init__(provider=provider, path=path)

    @staticmethod
    def scheme() -> str:
        return "file"

    def load_config(self, config_path: str) -> ConfigResult:
        normalized_config_path = self._normalize_file_name(config_path)
        full_path = os.path.realpath(os.path.join(self.path, normalized_config_path))
        if not os.path.exists(full_path):
            raise ConfigLoadError(f"Config not found : {full_path}")

        with open(full_path, encoding="utf-8") as f:
            content = f.read()

            # Extract header using Rust if available
            if _RUST_AVAILABLE:
                header = _rs.extract_header_dict(content)
            else:
                header = ConfigSource._get_header_dict(content)

            if _RUST_AVAILABLE:
                # Use Rust for fast YAML parsing
                try:
                    raw_config = _rs.parse_yaml(content)
                    if raw_config is None:
                        raw_config = {}
                    cfg = OmegaConf.create(raw_config)
                except Exception:
                    # Fall back to Python YAML parser on errors
                    raw = yaml.safe_load(content)
                    if raw is None:
                        raw = {}
                    cfg = OmegaConf.create(raw)
            else:
                raw = yaml.safe_load(content)
                if raw is None:
                    raw = {}
                cfg = OmegaConf.create(raw)

            return ConfigResult(
                config=cfg,
                path=f"{self.scheme()}://{self.path}",
                provider=self.provider,
                header=header,
            )

    def available(self) -> bool:
        return self.is_group("")

    def is_group(self, config_path: str) -> bool:
        full_path = os.path.realpath(os.path.join(self.path, config_path))
        return os.path.isdir(full_path)

    def is_config(self, config_path: str) -> bool:
        config_path = self._normalize_file_name(config_path)
        full_path = os.path.realpath(os.path.join(self.path, config_path))
        return os.path.isfile(full_path)

    def list(self, config_path: str, results_filter: Optional[ObjectType]) -> List[str]:
        files: List[str] = []
        full_path = os.path.realpath(os.path.join(self.path, config_path))
        for file in os.listdir(full_path):
            file_path = os.path.join(config_path, file)
            self._list_add_result(
                files=files,
                file_path=file_path,
                file_name=file,
                results_filter=results_filter,
            )

        return sorted(list(set(files)))
