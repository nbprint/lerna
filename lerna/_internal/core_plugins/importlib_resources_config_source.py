# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import os
import zipfile
from importlib import resources
from typing import Any, List, Optional

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


class ImportlibResourcesConfigSource(ConfigSource):
    def __init__(self, provider: str, path: str) -> None:
        super().__init__(provider=provider, path=path)
        # normalize to pkg format
        self.path = self.path.replace("/", ".").rstrip(".")

    @staticmethod
    def scheme() -> str:
        return "pkg"

    @staticmethod
    def _safe_is_file(res: Any) -> bool:
        """Safely check if resource is a file.

        Works around importlib-resources 6.2+ OrphanPath issue (Hydra #2870).
        OrphanPath objects may not have is_file/is_dir/exists methods.
        """
        try:
            return res.is_file()
        except AttributeError:
            # OrphanPath or similar object without is_file method
            return False

    @staticmethod
    def _safe_is_dir(res: Any) -> bool:
        """Safely check if resource is a directory.

        Works around importlib-resources 6.2+ OrphanPath issue (Hydra #2870).
        """
        try:
            return res.is_dir()
        except AttributeError:
            return False

    def _read_config(self, res: Any) -> ConfigResult:
        try:
            if isinstance(res, zipfile.Path):
                # zipfile does not support encoding, read() calls returns bytes.
                f = res.open()
            else:
                f = res.open(encoding="utf-8")
            content = f.read()
            if isinstance(content, bytes):
                # if content is bytes, utf-8 decode (zipfile path)
                content_str = content.decode("utf-8")
            else:
                content_str = content

            # Use Rust for header extraction if available
            if _RUST_AVAILABLE:
                header = _rs.extract_header_dict(content_str)
            else:
                header = ConfigSource._get_header_dict(content_str)

            # Use Rust for fast YAML parsing if available
            if _RUST_AVAILABLE:
                try:
                    raw_config = _rs.parse_yaml(content_str)
                    if raw_config is None:
                        raw_config = {}
                    cfg = OmegaConf.create(raw_config)
                except Exception:
                    # Fall back to Python YAML parser on errors
                    raw = yaml.safe_load(content_str)
                    if raw is None:
                        raw = {}
                    cfg = OmegaConf.create(raw)
            else:
                raw = yaml.safe_load(content_str)
                if raw is None:
                    raw = {}
                cfg = OmegaConf.create(raw)

            return ConfigResult(
                config=cfg,
                path=f"{self.scheme()}://{self.path}",
                provider=self.provider,
                header=header,
            )
        finally:
            f.close()

    def load_config(self, config_path: str) -> ConfigResult:
        normalized_config_path = self._normalize_file_name(config_path)
        res = resources.files(self.path).joinpath(normalized_config_path)
        if not (self._safe_is_file(res) or self._safe_is_dir(res)):
            raise ConfigLoadError(f"Config not found : {normalized_config_path}")

        return self._read_config(res)

    def available(self) -> bool:
        try:
            files = resources.files(self.path)
        except (ValueError, ModuleNotFoundError, TypeError):
            return False
        return any(f.name == "__init__.py" and self._safe_is_file(f) for f in files.iterdir())

    def is_group(self, config_path: str) -> bool:
        try:
            files = resources.files(self.path)
        except (ValueError, ModuleNotFoundError, TypeError):
            return False

        res = files.joinpath(config_path)
        return self._safe_is_dir(res)

    def is_config(self, config_path: str) -> bool:
        config_path = self._normalize_file_name(config_path)
        try:
            files = resources.files(self.path)
        except (ValueError, ModuleNotFoundError, TypeError):
            return False
        res = files.joinpath(config_path)
        return self._safe_is_file(res)

    def list(self, config_path: str, results_filter: Optional[ObjectType]) -> List[str]:
        files: List[str] = []
        for file in resources.files(self.path).joinpath(config_path).iterdir():
            fname = file.name
            fpath = os.path.join(config_path, fname)
            self._list_add_result(
                files=files,
                file_path=fpath,
                file_name=fname,
                results_filter=results_filter,
            )

        return sorted(list(set(files)))
