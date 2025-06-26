from importlib import import_module
from importlib.metadata import EntryPoints, entry_points
from logging import getLogger

from hydra.core.config_search_path import ConfigSearchPath
from hydra.plugins.search_path_plugin import SearchPathPlugin

_log = getLogger("lerna")

# NOTE: use `lernaplugins` instead of `plugins`
# for https://github.com/facebookresearch/hydra/pull/3052
_discovered_plugins: EntryPoints = entry_points(group="hydra.lernaplugins")
for entry_point in _discovered_plugins:
    try:
        globals()[entry_point.name] = import_module(entry_point.value)
    except ImportError as e:
        _log.warning(f"Failed to import entry point {entry_point.name} from {entry_point.value}: {e}")
    # search_path.append(provider=entry_point.name, path=entry_point.value)


class LernaGenericSearchPathPlugin(SearchPathPlugin):
    def manipulate_search_path(self, search_path: ConfigSearchPath) -> None: ...
