# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import importlib
from typing import Any

from pytest import MonkeyPatch, mark, raises, warns

from lerna.core.config_search_path import ConfigSearchPath
from lerna.core.plugins import Plugins
from lerna.plugins.launcher import Launcher
from lerna.plugins.plugin import Plugin
from lerna.plugins.search_path_plugin import SearchPathPlugin
from lerna.plugins.sweeper import Sweeper
from lerna.utils import get_class

# This only test core plugins.
# Individual plugins are responsible to test that they are discoverable.
launchers = ["lerna._internal.core_plugins.basic_launcher.BasicLauncher"]
sweepers = ["lerna._internal.core_plugins.basic_sweeper.BasicSweeper"]
search_path_plugins: list[str] = []


@mark.parametrize(
    "plugin_type, expected",
    [
        (Launcher, launchers),
        (Sweeper, sweepers),
        (SearchPathPlugin, search_path_plugins),
        (Plugin, launchers + sweepers + search_path_plugins),
    ],
)
def test_discover(plugin_type: type[Plugin], expected: list[str]) -> None:
    plugins = Plugins.instance().discover(plugin_type)
    expected_classes = [get_class(c) for c in expected]
    for ex in expected_classes:
        assert ex in plugins


def test_register_plugin() -> None:
    class MyPlugin(SearchPathPlugin):
        def manipulate_search_path(self, search_path: ConfigSearchPath) -> None: ...

    Plugins.instance().register(MyPlugin)

    assert MyPlugin in Plugins.instance().discover(Plugin)
    assert MyPlugin in Plugins.instance().discover(SearchPathPlugin)
    assert MyPlugin not in Plugins.instance().discover(Launcher)


def test_register_bad_plugin() -> None:
    class NotAPlugin: ...

    with raises(ValueError, match="Not a valid Hydra Plugin"):
        Plugins.instance().register(NotAPlugin)  # type: ignore


class ExternalLauncher(Launcher):
    def setup(self, *, hydra_context: Any, task_function: Any, config: Any) -> None:
        pass

    def launch(self, job_overrides: Any, initial_job_idx: int) -> Any:
        return []


def _import_module_without_plugin_namespaces() -> Any:
    original_import_module = importlib.import_module

    def import_module(name: str) -> Any:
        if name in ("lerna_plugins", "hydra_plugins"):
            raise ImportError(name)
        return original_import_module(name)

    return import_module


def test_entry_point_plugin_discovery(monkeypatch: MonkeyPatch, hydra_restore_singletons: Any) -> None:
    class EntryPoint:
        name = "external"

        def load(self) -> type[ExternalLauncher]:
            return ExternalLauncher

    with monkeypatch.context() as patch:
        patch.setattr("lerna.core.plugins.importlib.import_module", _import_module_without_plugin_namespaces())
        patch.setattr("lerna.core.plugins.entry_points", lambda group: [EntryPoint()] if group == "hydra.plugins" else [])
        Plugins.instance()._initialize()
        assert ExternalLauncher in Plugins.instance().discover(Launcher)
        stats = Plugins.instance().get_stats()
        assert stats is not None
        assert "entry point: external" in stats.modules_import_time
        assert stats.total_time >= stats.total_modules_import_time


@mark.parametrize("error_type", [AttributeError, RuntimeError])
def test_bad_entry_point_does_not_stop_discovery(
    monkeypatch: MonkeyPatch,
    hydra_restore_singletons: Any,
    error_type: type[Exception],
) -> None:
    class MissingEntryPoint:
        name = "missing"

        def load(self) -> None:
            raise error_type("plugin could not load")

    class ValidEntryPoint:
        name = "valid"

        def load(self) -> type[ExternalLauncher]:
            return ExternalLauncher

    with monkeypatch.context() as patch:
        patch.setattr("lerna.core.plugins.importlib.import_module", _import_module_without_plugin_namespaces())
        patch.setattr(
            "lerna.core.plugins.entry_points",
            lambda group: [MissingEntryPoint(), ValidEntryPoint()] if group == "hydra.plugins" else [],
        )
        with warns(UserWarning, match="Error loading Lerna plugin entry point 'missing'"):
            Plugins.instance()._initialize()
        assert ExternalLauncher in Plugins.instance().discover(Launcher)


def test_entry_point_must_be_a_concrete_plugin(monkeypatch: MonkeyPatch, hydra_restore_singletons: Any) -> None:
    class NotAPluginEntryPoint:
        name = "not_a_plugin"

        def load(self) -> type:
            return int

    with monkeypatch.context() as patch:
        patch.setattr("lerna.core.plugins.importlib.import_module", _import_module_without_plugin_namespaces())
        patch.setattr(
            "lerna.core.plugins.entry_points",
            lambda group: [NotAPluginEntryPoint()] if group == "hydra.plugins" else [],
        )
        with warns(UserWarning, match="is not a concrete plugin class"):
            Plugins.instance()._initialize()


def test_plugin_outside_namespace_packages_is_allowed(hydra_restore_singletons: Any) -> None:
    """Hydra 1.4 dropped the lerna_plugins/hydra_plugins package requirement."""
    Plugins.instance().register(ExternalLauncher)
    assert ExternalLauncher in Plugins.instance().discover(Launcher)
