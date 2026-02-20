# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import lerna
from lerna.core.hydra_config import HydraConfig
from omegaconf import DictConfig


@lerna.main(version_base=None)
def my_app(_: DictConfig) -> None:
    print(HydraConfig.instance().get().runtime.output_dir)


if __name__ == "__main__":
    my_app()
