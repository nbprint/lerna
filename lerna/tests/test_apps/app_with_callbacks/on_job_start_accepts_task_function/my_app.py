# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved

import logging
from typing import Any

from omegaconf import DictConfig

import lerna
from lerna.experimental.callback import Callback
from lerna.types import TaskFunction

log = logging.getLogger(__name__)


class OnJobStartCallback(Callback):
    def on_job_start(self, config: DictConfig, *, task_function: TaskFunction, **kwargs: Any) -> None:
        assert task_function(...) == "called my_app"
        log.info(f"on_job_start task_function: {task_function}")


@lerna.main(version_base=None, config_path=".", config_name="config")
def my_app(cfg: DictConfig) -> Any:
    return "called my_app"


if __name__ == "__main__":
    my_app()
