# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from omegaconf import DictConfig

import lerna


def foo(cfg: DictConfig) -> None:
    cfg.foo = "bar"  # does not exist in the config


@lerna.main(config_path=".", config_name="config")
def my_app(cfg: DictConfig) -> None:
    foo(cfg)


if __name__ == "__main__":
    my_app()
