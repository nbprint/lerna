# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import logging
import os

from omegaconf import DictConfig

import lerna

log = logging.getLogger(__name__)


@lerna.main(version_base=None)
def my_app(_: DictConfig) -> None:
    assert os.getenv("FOO") == "bar"


if __name__ == "__main__":
    my_app()
