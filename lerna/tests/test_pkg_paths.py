import sys
import zipfile
from importlib import resources
from pathlib import Path

import pytest

from lerna._internal.core_plugins.importlib_resources_config_source import ImportlibResourcesConfigSource
from lerna._internal.core_plugins.pkg_helper import find_package_dir, pkg_config_exists, pkg_files
from lerna.core.plugins import _is_pkg_path_available

RAISES = "raise RuntimeError('package code should not run')\n"


def _write(root: Path, files: dict[str, str]) -> None:
    for name, content in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)


@pytest.fixture
def syspath(tmp_path, monkeypatch):
    monkeypatch.syspath_prepend(str(tmp_path))
    before = set(sys.modules)
    yield tmp_path
    for name in set(sys.modules) - before:
        del sys.modules[name]


def test_parents_are_not_imported(syspath):
    _write(syspath, {"heavy/__init__.py": RAISES, "heavy/conf/__init__.py": RAISES, "heavy/conf/db/sqlite.yaml": "driver: sqlite\n"})

    assert find_package_dir("heavy.conf") == syspath / "heavy" / "conf"
    assert _is_pkg_path_available("heavy/conf")
    assert pkg_config_exists("heavy.conf", "db/sqlite")
    source = ImportlibResourcesConfigSource(provider="test", path="pkg://heavy.conf")
    assert source.available()
    assert source.is_group("db")
    assert source.is_config("db/sqlite")
    assert source.list("db", None) == ["sqlite"]
    assert "heavy" not in sys.modules


def test_reading_a_config_imports_the_package(syspath):
    _write(syspath, {"sideeffect/__init__.py": "", "sideeffect/conf/__init__.py": "LOADED = True\n", "sideeffect/conf/a.yaml": "a: 1\n"})
    source = ImportlibResourcesConfigSource(provider="test", path="pkg://sideeffect.conf")

    assert source.is_config("a")
    assert "sideeffect.conf" not in sys.modules
    assert source.load_config("a").config == {"a": 1}
    assert sys.modules["sideeffect.conf"].LOADED


def test_namespace_parent(syspath):
    _write(syspath, {"nsparent/conf/__init__.py": RAISES})

    assert find_package_dir("nsparent.conf") == syspath / "nsparent" / "conf"


def test_imported_parent_uses_its_path(syspath):
    _write(syspath, {"extended/__init__.py": "", "elsewhere/conf/__init__.py": ""})
    import extended

    extended.__path__.append(str(syspath / "elsewhere"))

    assert find_package_dir("extended.conf") == syspath / "elsewhere" / "conf"


@pytest.mark.parametrize(
    ("files", "module_path"),
    [
        ({}, "missing.conf"),
        ({"shadow/__init__.py": "", "shadow/conf.py": ""}, "shadow.conf"),
        ({"nsleaf/conf/a.yaml": ""}, "nsleaf.conf"),
    ],
)
def test_unresolved_paths_defer_to_importlib(syspath, files, module_path):
    _write(syspath, files)

    assert find_package_dir(module_path) is None


def test_zip_import_falls_back(tmp_path, monkeypatch):
    archive = tmp_path / "zipped.zip"
    with zipfile.ZipFile(archive, "w") as zf:
        zf.writestr("zipped/__init__.py", "")
        zf.writestr("zipped/conf/__init__.py", "")
        zf.writestr("zipped/conf/a.yaml", "a: 1\n")
    monkeypatch.syspath_prepend(str(archive))
    try:
        assert find_package_dir("zipped.conf") is None
        assert pkg_files("zipped.conf").joinpath("a.yaml").read_text() == resources.files("zipped.conf").joinpath("a.yaml").read_text()
    finally:
        for name in [n for n in sys.modules if n == "zipped" or n.startswith("zipped.")]:
            del sys.modules[name]
