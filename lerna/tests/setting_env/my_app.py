# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import os

import lerna
from omegaconf import DictConfig


@lerna.main(version_base=None, config_name="config")
def my_app(_: DictConfig) -> None:
    print(f"foo={os.environ['foo']}")
    print(f"bar={os.environ['bar']}")


if __name__ == "__main__":
    my_app()
