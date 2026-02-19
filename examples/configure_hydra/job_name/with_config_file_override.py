# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from omegaconf import DictConfig

import lerna
from lerna.core.hydra_config import HydraConfig


@lerna.main(version_base=None, config_path=".", config_name="config")
def experiment(_cfg: DictConfig) -> None:
    print(HydraConfig.get().job.name)


if __name__ == "__main__":
    experiment()
