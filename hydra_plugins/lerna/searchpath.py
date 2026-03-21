# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""
Bridge plugin that enables hydra-core to discover plugins registered via lerna.

When using hydra-core (not lerna directly), this plugin discovers plugins
registered under the `hydra.lernaplugins` entry point group and makes them
available to hydra's plugin system.

This enables gradual migration: plugins can be written for lerna and still
work with hydra-core installations.
"""

import sys
from importlib import import_module

from hydra.core.config_search_path import ConfigSearchPath
from hydra.core.config_store import ConfigStore
from hydra.plugins.search_path_plugin import SearchPathPlugin

if sys.version_info < (3, 10):
    from importlib_metadata import entry_points
else:
    from importlib.metadata import entry_points

# NOTE: use `lernaplugins` instead of `plugins`
# for https://github.com/facebookresearch/hydra/pull/3052
_discovered_plugins = entry_points(group="hydra.lernaplugins")
_searchpaths_pkg = {}
_discovered_plugin_names = []
for entry_point in _discovered_plugins:
    if entry_point.value.startswith(("pkg:", "file:")):
        # This is a package style entry point
        kind, pkg_name = entry_point.value.split(":", 1)
        _searchpaths_pkg[entry_point.name] = f"{kind}://{pkg_name}"
        continue
    # Otherwise, it's a module style entry point
    try:
        mod = import_module(entry_point.value)
    except ImportError:
        # Silently skip entry points that fail to import. These are optional
        # third-party plugins and their absence should not disrupt the system.
        continue
    for attr in dir(mod):
        thing = getattr(mod, attr)
        if isinstance(thing, type) and issubclass(thing, SearchPathPlugin):
            _discovered_plugin_names.append(thing.__name__)
            globals()[thing.__name__] = thing


class LernaGenericSearchPathPlugin(SearchPathPlugin):
    """
    A SearchPathPlugin that bridges lerna plugins to hydra-core.

    This plugin is automatically discovered by hydra-core due to being in
    the hydra_plugins namespace. It then discovers any plugins registered
    under the `hydra.lernaplugins` entry point group.
    """

    def manipulate_search_path(self, search_path: ConfigSearchPath) -> None:
        if _searchpaths_pkg:
            for provider, path in _searchpaths_pkg.items():
                inst = ConfigStore.instance()
                inst.store(name=provider, node=None, group=provider, provider=provider)
                search_path.append(provider=provider, path=path)
