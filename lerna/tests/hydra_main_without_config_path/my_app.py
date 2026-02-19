# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import lerna
from omegaconf import DictConfig


@lerna.main()  # NOTE: version_base parameter intentionally omitted
def my_app(_: DictConfig) -> None:
    pass


if __name__ == "__main__":
    my_app()
