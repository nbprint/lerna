# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved

import lerna
from lerna.tests.test_apps.app_with_cfg_decorated.decorators.inner_decorator import (
    inner_decorator,
)
from lerna.tests.test_apps.app_with_cfg_decorated.decorators.outer_decorator import (
    data,
    outer_decorator,
)
from omegaconf import DictConfig


@outer_decorator("test")  # type: ignore[misc]
@lerna.main(version_base=None, config_path=".", config_name="config")
@inner_decorator  # type: ignore[misc]
def main(conf: DictConfig) -> None:
    assert conf.dataset.name == "imagenet"


if __name__ == "__main__":
    main()

    assert data == ["test"]
