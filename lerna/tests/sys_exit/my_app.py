# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import sys

import lerna
from omegaconf import DictConfig


@lerna.main(version_base=None)
def my_app(_: DictConfig) -> None:
    sys.exit(42)


if __name__ == "__main__":
    my_app()
