# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import hydra

from omegaconf import DictConfig


@hydra.main()
def experiment(_: DictConfig) -> None:
    pass


if __name__ == "__main__":
    experiment()
