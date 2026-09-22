# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from textwrap import dedent

from omegaconf import DictConfig, OmegaConf, open_dict

from lerna import version
from lerna.core.global_hydra import GlobalHydra
from lerna.provenance import CompositionResult
from lerna.types import RunMode

from ._internal.deprecation_warning import deprecation_warning


def compose(
    config_name: str | None = None,
    overrides: list[str] | None = None,
    return_hydra_config: bool = False,
    strict: bool | None = None,
) -> DictConfig:
    """
    :param config_name: the name of the config
           (usually the file name without the .yaml extension)
    :param overrides: list of overrides for config file
    :param return_hydra_config: True to return the hydra config node in the result
    :param strict: DEPRECATED. If false, returned config has struct mode disabled.
    :return: the composed config
    """

    if overrides is None:
        overrides = []

    assert GlobalHydra().is_initialized(), "GlobalHydra is not initialized, use @hydra.main() or call one of the hydra initialization methods first"

    gh = GlobalHydra.instance()
    assert gh.hydra is not None
    cfg = gh.hydra.compose_config(
        config_name=config_name,
        overrides=overrides,
        run_mode=RunMode.RUN,
        from_shell=False,
        with_log_configuration=False,
    )
    assert isinstance(cfg, DictConfig)

    if not return_hydra_config and "hydra" in cfg:
        with open_dict(cfg):
            del cfg["hydra"]

    if strict is not None:
        if version.base_at_least("1.2"):
            raise TypeError("got an unexpected 'strict' argument")
        else:
            deprecation_warning(
                dedent(
                    """
                    The strict flag in the compose API is deprecated.
                    See https://hydra.cc/docs/1.2/upgrades/0.11_to_1.0/strict_mode_flag_deprecated for more info.
                    """
                )
            )
            OmegaConf.set_struct(cfg, strict)

    return cfg


def compose_with_provenance(
    config_name: str | None = None,
    overrides: list[str] | None = None,
    return_hydra_config: bool = False,
) -> CompositionResult:
    """Compose a config and return unresolved values with composition provenance."""
    from lerna._internal.callbacks import Callbacks
    from lerna._internal.config_loader_impl import ConfigLoaderImpl
    from lerna.core.hydra_config import HydraConfig

    if overrides is None:
        overrides = []

    assert GlobalHydra().is_initialized(), "GlobalHydra is not initialized, use @lerna.main() or call an initialization method first"
    hydra = GlobalHydra.instance().hydra
    assert hydra is not None
    loader = hydra.config_loader
    if not isinstance(loader, ConfigLoaderImpl):
        raise TypeError("compose_with_provenance() requires Lerna's ConfigLoaderImpl")

    cfg, result = loader.load_configuration_with_provenance(
        config_name=config_name,
        overrides=overrides,
        run_mode=RunMode.RUN,
        from_shell=False,
        validate_sweep_overrides=True,
    )
    orig_hydra_cfg = HydraConfig.instance().cfg
    was_readonly = OmegaConf.is_readonly(cfg.hydra)
    HydraConfig.instance().set_config(cfg)
    try:
        Callbacks(cfg, check_cache=False).on_compose_config(config=cfg, config_name=config_name, overrides=overrides)
    finally:
        HydraConfig.instance().cfg = orig_hydra_cfg
        OmegaConf.set_readonly(cfg.hydra, was_readonly)

    if not return_hydra_config and "hydra" in cfg:
        with open_dict(cfg):
            del cfg["hydra"]
    return result
