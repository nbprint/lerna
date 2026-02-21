# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from dataclasses import dataclass
from typing import Dict

from omegaconf import MISSING

import lerna
from lerna.core.config_store import ConfigStore
from lerna.core.hydra_config import HydraConfig


@dataclass
class Config:
    age: int = MISSING
    name: str = MISSING
    group: Dict[str, str] = MISSING


ConfigStore.instance().store(name="config_schema", node=Config)
ConfigStore.instance().store(name="config_schema", node=Config, group="test")


@lerna.main(version_base=None, config_path=".", config_name="config")
def my_app(cfg: Config) -> None:
    print(f"job_name: {HydraConfig().get().job.name}, name: {cfg.name}, age: {cfg.age}, group: {cfg.group['name']}")


if __name__ == "__main__":
    my_app()
