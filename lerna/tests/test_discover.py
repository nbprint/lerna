# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""
Tests for plugin discovery via hydra-core when using lerna plugins.

This tests the bridge functionality that allows plugins registered via lerna's
entrypoint system (hydra.lernaplugins) to be discovered by hydra-core.
"""

from pathlib import Path
from subprocess import check_call

import pytest
from hydra.core.plugins import Plugins as HydraPlugins


@pytest.fixture(scope="module", autouse=True)
def install_fake_packages():
    """Install fake test packages that register plugins via entry points."""
    folder = (Path(__file__).parent / "fake_package").resolve()
    check_call(["pip", "install", str(folder)])

    folder = (Path(__file__).parent / "fake_package2").resolve()
    check_call(["pip", "install", str(folder)])


class TestSearchpathPlugin:
    """Tests for the LernaGenericSearchPathPlugin bridge."""

    def test_discover_self(self):
        """Test that hydra-core can discover lerna's bridge plugin and plugins registered via lerna."""
        p = HydraPlugins()
        all_ps = [_.__name__ for _ in p.discover()]
        # The bridge plugin itself should be discovered
        assert "LernaGenericSearchPathPlugin" in all_ps
        # The fake_package plugin (registered via module entry point) should be discovered
        assert "FakePackageSearchPathPlugin" in all_ps
        # The fake_package2 (registered via pkg: style entry point) should be in searchpaths
        import hydra_plugins.lerna.searchpath

        # At least the fake_package2 should be registered
        assert "fake-package2" in hydra_plugins.lerna.searchpath._searchpaths_pkg
        assert hydra_plugins.lerna.searchpath._searchpaths_pkg["fake-package2"] == "pkg://fake_package2.hydra"


class TestLernaSearchpathEntryPoints:
    """Tests for lerna-side discovery of entrypoint search path plugins."""

    def test_lerna_skips_unavailable_entrypoint_search_paths(self):
        """
        Entrypoint-registered search paths whose packages are not importable
        (e.g. test fixtures without __init__.py) must be silently skipped so
        that they don't produce warnings that crash under ``-Werror``.
        """
        from lerna._internal.utils import create_config_search_path
        from lerna.core.plugins import Plugins as LernaPlugins
        from lerna.core.singleton import Singleton

        Singleton._instances.pop(LernaPlugins, None)

        search_path = create_config_search_path(None)
        entries = {(entry.provider, entry.path) for entry in search_path.get_path()}

        # fake-package2 has pkg:fake_package2.hydra but fake_package2/hydra has
        # no __init__.py, so the package is not importable – must be skipped.
        assert ("fake-package2", "pkg://fake_package2.hydra") not in entries

        # fake-package is a module-style entry point with a hydra-only
        # SearchPathPlugin.  Lerna intentionally does not wrap hydra plugins
        # (the bridge handles them), so it must not appear here either.
        assert ("fake-package", "pkg://fake_package/hydra") not in entries

    def test_lerna_discovers_native_search_path_plugins_from_entrypoints(self):
        """
        If a lerna-native SearchPathPlugin is registered via entrypoints,
        lerna's Plugins system should discover it.
        """
        from lerna.core.plugins import Plugins as LernaPlugins
        from lerna.core.singleton import Singleton
        from lerna.plugins.search_path_plugin import SearchPathPlugin as LernaSearchPathPlugin

        Singleton._instances.pop(LernaPlugins, None)

        plugins = LernaPlugins.instance()
        discovered = plugins.discover(LernaSearchPathPlugin)
        discovered_names = [cls.__name__ for cls in discovered]

        # The hydra-only FakePackageSearchPathPlugin should NOT be in lerna's
        # discovered list (it doesn't subclass lerna.plugins.SearchPathPlugin).
        assert "FakePackageSearchPathPlugin" not in discovered_names
