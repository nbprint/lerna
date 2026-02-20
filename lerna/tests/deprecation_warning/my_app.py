# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import lerna
from lerna._internal.deprecation_warning import deprecation_warning
from omegaconf import DictConfig


@lerna.main(version_base=None)
def my_app(cfg: DictConfig) -> None:
    deprecation_warning("Feature FooBar is deprecated")


if __name__ == "__main__":
    my_app()
