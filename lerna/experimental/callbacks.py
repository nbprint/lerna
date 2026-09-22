# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved

import copy
import logging
import pickle
from pathlib import Path
from typing import Any

from omegaconf import DictConfig, OmegaConf, flag_override

from lerna._internal.deprecation_warning import deprecation_warning
from lerna.core.global_hydra import GlobalHydra
from lerna.core.utils import JobReturn
from lerna.errors import Hydra15MigrationWarning
from lerna.experimental.callback import Callback
from lerna.types import RunMode


class LogJobReturnCallback(Callback):
    """Deprecated no-op compatibility stub; removed in Hydra 1.5."""

    def __init__(self) -> None:
        deprecation_warning(
            "LogJobReturnCallback no longer has any effect and will be removed in Hydra 1.5. Task exceptions are logged to per-job logs without it.",
            stacklevel=2,
            category=Hydra15MigrationWarning,
        )


class PickleJobInfoCallback(Callback):
    """Pickle the job config/return-value in ${output_dir}/{config,job_return}.pickle"""

    output_dir: Path

    def __init__(self) -> None:
        self.log = logging.getLogger(f"{__name__}.{self.__class__.__name__}")

    def on_job_start(self, config: DictConfig, **kwargs: Any) -> None:
        """Pickle the job's config in ${output_dir}/config.pickle."""
        self.output_dir = Path(config.hydra.runtime.output_dir) / Path(config.hydra.output_subdir)
        filename = "config.pickle"
        self._save_pickle(obj=config, filename=filename, output_dir=self.output_dir)
        self.log.info(f"Saving job configs in {self.output_dir / filename}")

    def on_job_end(self, config: DictConfig, job_return: JobReturn, **kwargs: Any) -> None:
        """Pickle the job's return value in ${output_dir}/job_return.pickle."""
        filename = "job_return.pickle"
        self._save_pickle(obj=job_return, filename=filename, output_dir=self.output_dir)
        self.log.info(f"Saving job_return in {self.output_dir / filename}")

    def _save_pickle(self, obj: Any, filename: str, output_dir: Path) -> None:
        output_dir.mkdir(parents=True, exist_ok=True)
        assert output_dir is not None
        with open(str(output_dir / filename), "wb") as file:
            pickle.dump(obj, file, protocol=4)


class LogComposeCallback(Callback):
    """Log compose call, result, and debug info"""

    def __init__(self) -> None:
        self.log = logging.getLogger(f"{__name__}.{self.__class__.__name__}")

    def on_compose_config(
        self,
        config: DictConfig,
        config_name: str | None,
        overrides: list[str],
    ) -> None:
        gh = GlobalHydra.instance()
        config_loader = gh.config_loader()
        config_dir = "unknown"
        defaults_list = config_loader.compute_defaults_list(config_name, overrides, RunMode.RUN)
        all_sources = config_loader.get_sources()
        if config_name:
            for src in all_sources:
                if src.is_config(config_name):
                    config_dir = src.full_path()
                    break
        if "hydra" in config:
            config = copy.copy(config)
            with flag_override(config, ["struct", "readonly"], [False, False]):
                config.pop("hydra")
        non_hydra_defaults = [d.config_path for d in defaults_list.defaults if not d.package.startswith("hydra")]
        self.log.info(
            f"""====
Composed config {config_dir}/{config_name!s}
{OmegaConf.to_yaml(config)}
----
Includes overrides {overrides}
Used defaults {non_hydra_defaults}
===="""
        )
