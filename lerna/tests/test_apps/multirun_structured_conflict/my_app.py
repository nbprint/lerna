# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from dataclasses import dataclass

import lerna
from lerna.core.config_store import ConfigStore
from omegaconf import MISSING, DictConfig


@dataclass
class TestConfig:
    param: int = MISSING


config_store = ConfigStore.instance()
config_store.store(group="test", name="default", node=TestConfig)


@lerna.main(version_base=None, config_path=".", config_name="config")
def run(config: DictConfig) -> None:
    print(config.test.param)


if __name__ == "__main__":
    run()
