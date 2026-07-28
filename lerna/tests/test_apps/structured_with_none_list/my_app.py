# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from dataclasses import dataclass

from omegaconf import DictConfig

import lerna
from lerna.core.config_store import ConfigStore


@dataclass
class Config:
    list: list[int] | None = None


cs = ConfigStore.instance()
cs.store(name="config", node=Config)


@lerna.main(version_base=None, config_name="config")
def main(cfg: DictConfig) -> None:
    print(cfg)


if __name__ == "__main__":
    main()
