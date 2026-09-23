# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved

import logging

from omegaconf import DictConfig

import lerna
from lerna.core.hydra_config import HydraConfig
from lerna.utils import execution_whitelist

log = logging.getLogger(__name__)


@lerna.main(config_path=".", config_name="config")
def my_app(cfg: DictConfig) -> None:
    log.info(f"Output_dir={HydraConfig.get().runtime.output_dir}")
    log.info(f"cfg.foo={cfg.foo}")


if __name__ == "__main__":
    with execution_whitelist("lerna.experimental.callbacks.*"):
        my_app()
