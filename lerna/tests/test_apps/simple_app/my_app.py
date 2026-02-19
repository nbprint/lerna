# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import lerna
from omegaconf import DictConfig, OmegaConf


@lerna.main(version_base=None)
def my_app(cfg: DictConfig) -> None:
    print(OmegaConf.to_yaml(cfg, resolve=True))


if __name__ == "__main__":
    my_app()
