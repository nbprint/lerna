from pathlib import Path
from subprocess import check_call

from hydra.core.plugins import Plugins


class TestSearchpathPlugin:
    def test_discover_self(self):
        folder = (Path(__file__).parent / "fake_package").resolve()
        check_call(["pip", "install", str(folder)])

        p = Plugins()
        all_ps = [_.__name__ for _ in p.discover()]
        assert "LernaGenericSearchPathPlugin" in all_ps
