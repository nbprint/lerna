# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import os
from pathlib import Path

from omegaconf import DictConfig

import lerna
from lerna.core.hydra_config import HydraConfig


@lerna.main(version_base=None)
def main(_: DictConfig) -> None:
    subdir = Path(HydraConfig.get().run.dir) / Path("subdir")
    subdir.mkdir(exist_ok=True, parents=True)
    os.chdir(subdir)


if __name__ == "__main__":
    main()

print(f"current dir: {os.getcwd()}")
