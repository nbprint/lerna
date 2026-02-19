# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import lerna
from lerna.core.hydra_config import HydraConfig
from omegaconf import DictConfig


@lerna.main(version_base=None, config_path="conf", config_name="config")
def my_app(_: DictConfig) -> None:
    print(HydraConfig.get().mode)


if __name__ == "__main__":
    my_app()
