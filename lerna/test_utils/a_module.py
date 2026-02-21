# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from omegaconf import DictConfig

import lerna


@lerna.main()
def experiment(_: DictConfig) -> None:
    pass


if __name__ == "__main__":
    experiment()
