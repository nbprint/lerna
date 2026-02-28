# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import importlib
import importlib.util
import inspect
import pkgutil
import re
import sys
import warnings
from collections import defaultdict
from dataclasses import dataclass, field
from timeit import default_timer as timer
from typing import Any, Dict, List, Optional, Tuple, Type

from omegaconf import DictConfig

from lerna._internal.sources_registry import SourcesRegistry
from lerna.core.singleton import Singleton
from lerna.plugins.completion_plugin import CompletionPlugin
from lerna.plugins.config_source import ConfigSource
from lerna.plugins.launcher import Launcher
from lerna.plugins.plugin import Plugin
from lerna.plugins.search_path_plugin import SearchPathPlugin
from lerna.plugins.sweeper import Sweeper
from lerna.types import HydraContext, TaskFunction
from lerna.utils import instantiate

if sys.version_info < (3, 10):
    from importlib_metadata import entry_points
else:
    from importlib.metadata import entry_points

PLUGIN_TYPES: List[Type[Plugin]] = [
    Plugin,
    ConfigSource,
    CompletionPlugin,
    Launcher,
    Sweeper,
    SearchPathPlugin,
]


@dataclass
class ScanStats:
    total_time: float = 0
    total_modules_import_time: float = 0
    modules_import_time: Dict[str, float] = field(default_factory=dict)


class Plugins(metaclass=Singleton):
    @staticmethod
    def instance(*args: Any, **kwargs: Any) -> "Plugins":
        ret = Singleton.instance(Plugins, *args, **kwargs)
        assert isinstance(ret, Plugins)
        return ret

    def __init__(self) -> None:
        self.plugin_type_to_subclass_list: Dict[Type[Plugin], List[Type[Plugin]]] = {}
        self.class_name_to_class: Dict[str, Type[Plugin]] = {}
        self.stats: Optional[ScanStats] = None
        self._initialize()

    def _initialize(self) -> None:
        top_level: List[Any] = []
        core_plugins = importlib.import_module("lerna._internal.core_plugins")
        top_level.append(core_plugins)

        # Support both lerna_plugins and hydra_plugins for backward compatibility
        for plugin_namespace in ["lerna_plugins", "hydra_plugins"]:
            try:
                plugins_module = importlib.import_module(plugin_namespace)
                top_level.append(plugins_module)
            except ImportError:
                # If no plugins are installed the plugins package does not exist.
                pass

        self.plugin_type_to_subclass_list = defaultdict(list)
        self.class_name_to_class = {}

        scanned_plugins, self.stats = self._scan_all_plugins(modules=top_level)
        scanned_plugins.extend(_scan_entrypoint_search_path_plugins())
        for clazz in scanned_plugins:
            self._register(clazz)

    def register(self, clazz: Type[Plugin]) -> None:
        """
        Call Plugins.instance().register(MyPlugin) to manually register a plugin class.
        """
        if not _is_concrete_plugin_type(clazz):
            raise ValueError("Not a valid Hydra Plugin")
        self._register(clazz)

    def _register(self, clazz: Type[Plugin]) -> None:
        assert _is_concrete_plugin_type(clazz)
        for plugin_type in PLUGIN_TYPES:
            if issubclass(clazz, plugin_type):
                if clazz not in self.plugin_type_to_subclass_list[plugin_type]:
                    self.plugin_type_to_subclass_list[plugin_type].append(clazz)
        name = f"{clazz.__module__}.{clazz.__name__}"
        self.class_name_to_class[name] = clazz
        if issubclass(clazz, ConfigSource):
            SourcesRegistry.instance().register(clazz)

    def _instantiate(self, config: DictConfig) -> Plugin:
        from lerna._internal import utils as internal_utils

        classname = internal_utils._get_cls_name(config, pop=False)
        try:
            if classname is None:
                raise ImportError("class not configured")

            if not self.is_in_toplevel_plugins_module(classname):
                # All plugins must be defined inside the approved top level modules.
                # For plugins outside of lerna-core, the approved module is lerna_plugins or hydra_plugins.
                raise RuntimeError(f"Invalid plugin '{classname}': not in lerna_plugins or hydra_plugins package")

            if classname not in self.class_name_to_class.keys():
                raise RuntimeError(f"Unknown plugin class : '{classname}'")
            clazz = self.class_name_to_class[classname]
            plugin = instantiate(config=config, _target_=clazz)
            assert isinstance(plugin, Plugin)

        except ImportError as e:
            raise ImportError(f"Could not instantiate plugin {classname} : {str(e)}\n\n\tIS THE PLUGIN INSTALLED?\n\n")

        return plugin

    @staticmethod
    def is_in_toplevel_plugins_module(clazz: str) -> bool:
        return (
            clazz.startswith("lerna_plugins.")
            or clazz.startswith("hydra_plugins.")
            or clazz.startswith("lerna._internal.core_plugins.")
            or clazz.startswith("lerna._internal.core_plugins.")
        )

    def instantiate_sweeper(
        self,
        *,
        hydra_context: HydraContext,
        task_function: TaskFunction,
        config: DictConfig,
    ) -> Sweeper:
        Plugins.check_usage(self)
        if config.hydra.sweeper is None:
            raise RuntimeError("Hydra sweeper is not configured")
        sweeper = self._instantiate(config.hydra.sweeper)
        assert isinstance(sweeper, Sweeper)
        sweeper.setup(hydra_context=hydra_context, task_function=task_function, config=config)
        return sweeper

    def instantiate_launcher(
        self,
        hydra_context: HydraContext,
        task_function: TaskFunction,
        config: DictConfig,
    ) -> Launcher:
        Plugins.check_usage(self)
        if config.hydra.launcher is None:
            raise RuntimeError("Hydra launcher is not configured")

        launcher = self._instantiate(config.hydra.launcher)
        assert isinstance(launcher, Launcher)
        launcher.setup(hydra_context=hydra_context, task_function=task_function, config=config)
        return launcher

    @staticmethod
    def _scan_all_plugins(
        modules: List[Any],
    ) -> Tuple[List[Type[Plugin]], ScanStats]:
        stats = ScanStats()
        stats.total_time = timer()

        scanned_plugins: List[Type[Plugin]] = []

        for mdl in modules:
            for importer, modname, ispkg in pkgutil.walk_packages(path=mdl.__path__, prefix=mdl.__name__ + ".", onerror=lambda x: None):
                try:
                    module_name = modname.rsplit(".", 1)[-1]
                    # If module's name starts with "_", do not load the module.
                    # But if the module's name starts with a "__", then load the
                    # module.
                    if module_name.startswith("_") and not module_name.startswith("__"):
                        continue
                    import_time = timer()

                    with warnings.catch_warnings(record=True) as recorded_warnings:
                        if sys.version_info < (3, 10):
                            m = importer.find_module(modname)  # type: ignore
                            assert m is not None
                            loaded_mod = m.load_module(modname)
                        else:
                            spec = importer.find_spec(modname)  # type: ignore[call-arg]
                            assert spec is not None
                            if modname in sys.modules:
                                loaded_mod = sys.modules[modname]
                            else:
                                loaded_mod = importlib.util.module_from_spec(spec)
                            if loaded_mod is not None:
                                assert spec.loader is not None
                                spec.loader.exec_module(loaded_mod)
                                sys.modules[modname] = loaded_mod

                    import_time = timer() - import_time
                    if len(recorded_warnings) > 0:
                        sys.stderr.write(f"[Hydra plugins scanner] : warnings from '{modname}'. Please report to plugin author.\n")
                        for w in recorded_warnings:
                            warnings.showwarning(
                                message=w.message,
                                category=w.category,
                                filename=w.filename,
                                lineno=w.lineno,
                                file=w.file,
                                line=w.line,
                            )

                    stats.total_modules_import_time += import_time

                    assert modname not in stats.modules_import_time
                    stats.modules_import_time[modname] = import_time

                    if loaded_mod is not None:
                        for name, obj in inspect.getmembers(loaded_mod):
                            if _is_concrete_plugin_type(obj):
                                scanned_plugins.append(obj)
                except ImportError as e:
                    warnings.warn(
                        message=f"\n"
                        f"\tError importing '{modname}'.\n"
                        f"\tPlugin is incompatible with this Hydra version or buggy.\n"
                        f"\tRecommended to uninstall or upgrade plugin.\n"
                        f"\t\t{type(e).__name__} : {e}",
                        category=UserWarning,
                    )

        stats.total_time = timer() - stats.total_time
        return scanned_plugins, stats

    def get_stats(self) -> Optional[ScanStats]:
        return self.stats

    def discover(self, plugin_type: Optional[Type[Plugin]] = None) -> List[Type[Plugin]]:
        """
        :param plugin_type: class of plugin to discover, None for all
        :return: a list of plugins implementing the plugin type (or all if plugin type is None)
        """
        Plugins.check_usage(self)
        ret: List[Type[Plugin]] = []
        if plugin_type is None:
            plugin_type = Plugin
        assert issubclass(plugin_type, Plugin)
        if plugin_type not in self.plugin_type_to_subclass_list:
            return []
        for clazz in self.plugin_type_to_subclass_list[plugin_type]:
            ret.append(clazz)

        return ret

    @staticmethod
    def check_usage(self_: Any) -> None:
        if not isinstance(self_, Plugins):
            raise ValueError(f"Plugins is now a Singleton. usage: Plugins.instance().{inspect.stack()[1][3]}(...)")


def _is_concrete_plugin_type(obj: Any) -> bool:
    return inspect.isclass(obj) and issubclass(obj, Plugin) and not inspect.isabstract(obj)


def _sanitize_entrypoint_name(name: str) -> str:
    return re.sub(r"[^0-9a-zA-Z_]", "_", name)


def _is_pkg_path_available(pkg_path: str) -> bool:
    """Check whether a pkg:// path refers to an importable Python package."""
    # Convert path separators to dots for importlib
    module_path = pkg_path.replace("/", ".")
    try:
        spec = importlib.util.find_spec(module_path)
        return spec is not None
    except (ModuleNotFoundError, ValueError):
        return False


def _scan_entrypoint_search_path_plugins() -> List[Type[Plugin]]:
    """
    Discover SearchPathPlugin classes registered via the ``hydra.lernaplugins``
    and ``lerna.plugins`` entry-point groups so that they are available when
    lerna is used directly (without hydra-core).

    Two flavours of entry point are supported:

    * **pkg / file** – value starts with ``pkg:`` or ``file:``.  A dynamic
      SearchPathPlugin is synthesised that appends the path.  The package is
      validated first; if it cannot be imported the entry point is silently
      skipped (avoids warnings under ``-Werror``).

    * **module** – value is a dotted Python module path.  The module is
      imported and any concrete *lerna* ``SearchPathPlugin`` subclasses found
      inside are registered.  Hydra-only ``SearchPathPlugin`` subclasses are
      intentionally skipped here because the hydra bridge plugin
      (``hydra_plugins.lerna.searchpath``) already handles them.
    """
    scanned_plugins: List[Type[Plugin]] = []

    discovered: List[Any] = []
    for group in ("hydra.lernaplugins", "lerna.plugins"):
        try:
            discovered.extend(entry_points(group=group))
        except TypeError:
            discovered.extend(entry_points().get(group, []))  # type: ignore[arg-type]

    for entry_point in discovered:
        if entry_point.value.startswith(("pkg:", "file:")):
            kind, path = entry_point.value.split(":", 1)
            entrypoint_path = f"{kind}://{path}"
            provider = entry_point.name

            # Validate that the package actually exists before registering –
            # otherwise an unavailable source warning is emitted which turns
            # into a hard error under ``python -Werror``.
            if kind == "pkg" and not _is_pkg_path_available(path):
                continue

            sanitized_name = _sanitize_entrypoint_name(provider)

            def manipulate_search_path(self: Any, search_path: Any, provider: str = provider, entrypoint_path: str = entrypoint_path) -> None:
                search_path.append(provider=provider, path=entrypoint_path)

            dynamic_plugin = type(
                f"EntryPointSearchPathPlugin_{sanitized_name}",
                (SearchPathPlugin,),
                {
                    "__module__": __name__,
                    "manipulate_search_path": manipulate_search_path,
                },
            )
            scanned_plugins.append(dynamic_plugin)
            continue

        # Module-style entry point
        try:
            module = importlib.import_module(entry_point.value)
        except ImportError as e:
            warnings.warn(
                f"Failed to import entry point {entry_point.name} from {entry_point.value}: {e}",
                category=UserWarning,
            )
            continue

        # Only register lerna-native SearchPathPlugin subclasses.
        # Hydra SearchPathPlugin subclasses are handled by the hydra bridge
        # (hydra_plugins/lerna/searchpath.py) and should not be double-registered.
        for _, obj in inspect.getmembers(module):
            if _is_concrete_plugin_type(obj) and issubclass(obj, SearchPathPlugin):
                scanned_plugins.append(obj)

    return scanned_plugins
