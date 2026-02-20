# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import lerna
from omegaconf import DictConfig


def foo(cfg: DictConfig) -> None:
    cfg.foo = "bar"  # does not exist in the config


@lerna.main(version_base=None, config_path=".", config_name="config")
def my_app(cfg: DictConfig) -> None:
    foo(cfg)


if __name__ == "__main__":
    my_app()
