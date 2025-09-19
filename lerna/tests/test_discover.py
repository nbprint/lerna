from pathlib import Path
from subprocess import check_call

import pytest
from hydra.core.plugins import Plugins


@pytest.fixture(scope="module", autouse=True)
def install_fake_package():
    folder = (Path(__file__).parent / "fake_package").resolve()
    check_call(["pip", "install", str(folder)])

    folder = (Path(__file__).parent / "fake_package2").resolve()
    check_call(["pip", "install", str(folder)])


class TestSearchpathPlugin:
    def test_discover_self(self):
        p = Plugins()
        all_ps = [_.__name__ for _ in p.discover()]
        assert "LernaGenericSearchPathPlugin" in all_ps
        assert "FakePackageSearchPathPlugin" in all_ps
        import hydra_plugins.lerna.searchpath

        assert len(hydra_plugins.lerna.searchpath._searchpaths_pkg) == 1
