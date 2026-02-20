# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from typing import Any

from omegaconf import DictConfig

import lerna


def add(app_cfg: DictConfig, key1: str, key2: str) -> Any:
    num1 = app_cfg[key1]
    num2 = app_cfg[key2]
    ret = num1 + num2
    print(f"Hello {app_cfg.user}, {num1} + {num2} = {ret}")
    return ret


@lerna.main(version_base=None, config_path="conf", config_name="config")
def main(cfg: DictConfig) -> None:
    add(cfg.app, "num1", "num2")


if __name__ == "__main__":
    main()
