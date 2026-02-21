# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""
Tests for plugin discovery via hydra-core when using lerna plugins.

This tests the bridge functionality that allows plugins registered via lerna's
entrypoint system (hydra.lernaplugins) to be discovered by hydra-core.
"""

from pathlib import Path
from subprocess import check_call

import pytest
from hydra.core.plugins import Plugins


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
        p = Plugins()
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
