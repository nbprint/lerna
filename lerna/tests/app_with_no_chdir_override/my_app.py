# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import lerna
from omegaconf import DictConfig


@lerna.main(version_base="1.1", config_path=".")
def my_app(_: DictConfig) -> None:
    pass


if __name__ == "__main__":
    my_app()
