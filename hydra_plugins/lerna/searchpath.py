import sys
from importlib import import_module
from logging import getLogger

from hydra.core.config_search_path import ConfigSearchPath
from hydra.plugins.search_path_plugin import SearchPathPlugin

if sys.version_info < (3, 10):
    from importlib_metadata import entry_points
else:
    from importlib.metadata import entry_points

_log = getLogger("lerna")

# NOTE: use `lernaplugins` instead of `plugins`
# for https://github.com/facebookresearch/hydra/pull/3052
_discovered_plugins = entry_points(group="hydra.lernaplugins")
for entry_point in _discovered_plugins:
    try:
        mod = import_module(entry_point.value)
    except ImportError as e:
        _log.warning(f"Failed to import entry point {entry_point.name} from {entry_point.value}: {e}")
        continue
    for attr in dir(mod):
        thing = getattr(mod, attr)
        if isinstance(thing, type) and issubclass(thing, SearchPathPlugin):
            _log.info(f"Discovered search path plugin: {thing.__name__}")
            globals()[thing.__name__] = thing
    # search_path.append(provider=entry_point.name, path=entry_point.value)


class LernaGenericSearchPathPlugin(SearchPathPlugin):
    def manipulate_search_path(self, search_path: ConfigSearchPath) -> None: ...
