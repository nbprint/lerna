# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved

from omegaconf import DictConfig

from lerna import version
from lerna._internal.deprecation_warning import deprecation_warning


def compose(
    config_name: str | None = None,
    overrides: list[str] | None = None,
    return_hydra_config: bool = False,
    strict: bool | None = None,
) -> DictConfig:
    from lerna import compose as real_compose

    if overrides is None:
        overrides = []
    message = "hydra.experimental.compose() is no longer experimental. Use hydra.compose()"

    if version.base_at_least("1.2"):
        raise ImportError(message)

    deprecation_warning(message=message)
    return real_compose(
        config_name=config_name,
        overrides=overrides,
        return_hydra_config=return_hydra_config,
        strict=strict,
    )
