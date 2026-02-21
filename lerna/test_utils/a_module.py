# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import lerna
from omegaconf import DictConfig


@lerna.main()
def experiment(_: DictConfig) -> None:
    pass


if __name__ == "__main__":
    experiment()
