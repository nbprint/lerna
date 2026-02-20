# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import re
from textwrap import dedent
from typing import Any, List, Sequence, Union
from unittest.mock import Mock

from pytest import mark, raises

from lerna import TaskFunction
from lerna._internal.callbacks import Callbacks
from lerna._internal.config_loader_impl import ConfigLoaderImpl
from lerna._internal.utils import create_config_search_path
from lerna.core.config_loader import ConfigLoader
from lerna.core.plugins import Plugins
from lerna.core.utils import JobReturn, _check_hydra_context
from lerna.plugins.launcher import Launcher
from lerna.plugins.sweeper import Sweeper
from lerna.test_utils.test_utils import chdir_hydra_root
from lerna.types import HydraContext
from omegaconf import DictConfig, OmegaConf

chdir_hydra_root()


class IncompatibleSweeper(Sweeper):
    def __init__(self) -> None:
        pass

    def setup(  # type: ignore
        self,
        config: DictConfig,
        config_loader: ConfigLoader,
        task_function: TaskFunction,
    ) -> None:
        pass

    def sweep(self, arguments: List[str]) -> Any:
        pass


class IncompatibleLauncher(Launcher):
    def __init__(self) -> None:
        pass

    def setup(  # type: ignore
        self,
        config: DictConfig,
        config_loader: ConfigLoader,
        task_function: TaskFunction,
    ) -> None:
        pass

    def launch(  # type: ignore[empty-body]
        self, job_overrides: Sequence[Sequence[str]], initial_job_idx: int
    ) -> Sequence[JobReturn]:
        pass


@mark.parametrize(
    "plugin, config",
    [
        (IncompatibleLauncher(), OmegaConf.create({"hydra": {"launcher": {}}})),
        (IncompatibleSweeper(), OmegaConf.create({"hydra": {"sweeper": {}}})),
    ],
)
def test_setup_plugins(monkeypatch: Any, plugin: Union[Launcher, Sweeper], config: DictConfig) -> None:
    task_function = Mock(spec=TaskFunction)
    config_loader = ConfigLoaderImpl(config_search_path=create_config_search_path(None))
    hydra_context = HydraContext(config_loader=config_loader, callbacks=Callbacks())
    plugin_instance = Plugins.instance()
    monkeypatch.setattr(Plugins, "check_usage", lambda _: None)
    monkeypatch.setattr(plugin_instance, "_instantiate", lambda _: plugin)

    msg = "setup() got an unexpected keyword argument 'hydra_context'"
    with raises(TypeError, match=re.escape(msg)):
        if isinstance(plugin, Launcher):
            Plugins.instance().instantiate_launcher(
                hydra_context=hydra_context,
                task_function=task_function,
                config=config,
            )
        else:
            Plugins.instance().instantiate_sweeper(
                hydra_context=hydra_context,
                task_function=task_function,
                config=config,
            )


def test_run_job() -> None:
    hydra_context = None
    msg = dedent(
        """
        run_job's signature has changed: the `hydra_context` arg is now required.
        For more info, check https://github.com/facebookresearch/hydra/pull/1581."""
    )
    with raises(TypeError, match=msg):
        _check_hydra_context(hydra_context)
