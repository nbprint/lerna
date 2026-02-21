# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from omegaconf import DictConfig

import lerna
from lerna.core.hydra_config import HydraConfig


@lerna.main(version_base=None, config_path="conf", config_name="config")
def my_app(_: DictConfig) -> None:
    print(HydraConfig.get().mode)


if __name__ == "__main__":
    my_app()
