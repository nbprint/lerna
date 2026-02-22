# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import time
from pathlib import Path

from omegaconf import DictConfig

import lerna
from lerna.core.hydra_config import HydraConfig
from lerna.utils import get_original_cwd


@lerna.main(version_base=None)
def my_app(_: DictConfig) -> None:
    run_dir = str(Path.cwd().relative_to(get_original_cwd()))
    time.sleep(2)
    run_dir_after_sleep = str(Path(HydraConfig.get().run.dir))
    assert run_dir == run_dir_after_sleep


if __name__ == "__main__":
    my_app()
