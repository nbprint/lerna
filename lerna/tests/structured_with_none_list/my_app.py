# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from dataclasses import dataclass
from typing import List, Optional

import lerna
from lerna.core.config_store import ConfigStore
from omegaconf import DictConfig


@dataclass
class Config:
    list: Optional[List[int]] = None


cs = ConfigStore.instance()
cs.store(name="config", node=Config)


@lerna.main(version_base=None, config_name="config")
def main(cfg: DictConfig) -> None:
    print(cfg)


if __name__ == "__main__":
    main()
