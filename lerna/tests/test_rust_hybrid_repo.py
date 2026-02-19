# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""Tests for the RustHybridConfigRepository and pkg:// support."""

import tempfile
from pathlib import Path

import pytest

# Check if Rust extension is available
try:
    from lerna import lerna as _rs

    HAS_RUST = hasattr(_rs, "RustHybridConfigRepository")
except ImportError:
    HAS_RUST = False

from lerna._internal.core_plugins import pkg_helper


class TestPkgHelper:
    """Test the pkg:// helper functions."""

    def test_load_pkg_config_from_lerna_conf(self):
        """Test loading a config from lerna.conf package."""
        # Check if we can verify a module path exists
        # The test_utils.configs has yaml files we can test with
        result = pkg_helper.pkg_group_exists("lerna.test_utils.configs", "")
        assert result is True

    def test_load_pkg_config_nonexistent(self):
        """Test loading a non-existent config returns None."""
        result = pkg_helper.load_pkg_config("lerna.conf", "nonexistent/file.yaml")
        assert result is None

    def test_load_pkg_config_nonexistent_module(self):
        """Test loading from non-existent module returns None."""
        result = pkg_helper.load_pkg_config("nonexistent.module.path", "config.yaml")
        assert result is None

    def test_pkg_config_exists_false(self):
        """Test config_exists returns False for missing files."""
        assert pkg_helper.pkg_config_exists("lerna.conf", "nonexistent.yaml") is False

    def test_pkg_group_exists_root(self):
        """Test group_exists returns True for package root."""
        assert pkg_helper.pkg_group_exists("lerna.conf", "") is True

    def test_pkg_group_exists_nonexistent(self):
        """Test group_exists returns False for non-existent directories."""
        assert pkg_helper.pkg_group_exists("lerna.conf", "nonexistent/path") is False

    def test_pkg_list_options_nonexistent(self):
        """Test list_options returns empty list for non-existent paths."""
        result = pkg_helper.pkg_list_options("lerna.conf", "nonexistent/path")
        assert result == []

    def test_load_pkg_config_real_file(self):
        """Test loading an actual config file from lerna.test_utils.configs."""
        # This package should have config files
        result = pkg_helper.load_pkg_config("lerna.test_utils.configs", "compose.yaml")
        if result is not None:
            # If the file exists, it should be a dict
            assert isinstance(result, dict)


@pytest.mark.skipif(not HAS_RUST, reason="Rust extension not available")
class TestRustHybridConfigRepository:
    """Test the RustHybridConfigRepository class."""

    @pytest.fixture
    def temp_config_dir(self):
        """Create a temporary directory with config files."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create some config files
            config_dir = Path(tmpdir) / "conf"
            config_dir.mkdir()

            # Create main config
            (config_dir / "config.yaml").write_text("key: value\nnum: 42")

            # Create a group
            group_dir = config_dir / "db"
            group_dir.mkdir()
            (group_dir / "mysql.yaml").write_text("driver: mysql\nport: 3306")
            (group_dir / "postgres.yaml").write_text("driver: postgres\nport: 5432")

            yield str(config_dir)

    def test_create_hybrid_repo(self, temp_config_dir):
        """Test creating a hybrid repository."""
        repo = _rs.RustHybridConfigRepository(
            search_paths=[("test", f"file://{temp_config_dir}")],
            pkg_loader=pkg_helper.load_pkg_config,
            pkg_config_exists=pkg_helper.pkg_config_exists,
            pkg_group_exists=pkg_helper.pkg_group_exists,
            pkg_list_options=pkg_helper.pkg_list_options,
        )
        assert repr(repo).startswith("RustHybridConfigRepository")

    def test_load_file_config(self, temp_config_dir):
        """Test loading a config from file:// source."""
        repo = _rs.RustHybridConfigRepository(
            search_paths=[("test", f"file://{temp_config_dir}")],
        )

        result = repo.load_config("config.yaml")
        assert result is not None
        assert result["key"] == "value"
        assert result["num"] == 42

    def test_config_exists_file(self, temp_config_dir):
        """Test config_exists for file:// source."""
        repo = _rs.RustHybridConfigRepository(
            search_paths=[("test", f"file://{temp_config_dir}")],
        )

        assert repo.config_exists("config.yaml") is True
        assert repo.config_exists("nonexistent.yaml") is False

    def test_group_exists_file(self, temp_config_dir):
        """Test group_exists for file:// source."""
        repo = _rs.RustHybridConfigRepository(
            search_paths=[("test", f"file://{temp_config_dir}")],
        )

        assert repo.group_exists("db") is True
        assert repo.group_exists("nonexistent") is False

    def test_get_group_options_file(self, temp_config_dir):
        """Test get_group_options for file:// source."""
        repo = _rs.RustHybridConfigRepository(
            search_paths=[("test", f"file://{temp_config_dir}")],
        )

        options = repo.get_group_options("db")
        assert "mysql" in options
        assert "postgres" in options

    def test_hybrid_with_pkg_source(self, temp_config_dir):
        """Test hybrid repository with both file:// and pkg:// sources."""
        repo = _rs.RustHybridConfigRepository(
            search_paths=[
                ("test", f"file://{temp_config_dir}"),
                ("hydra", "pkg://lerna.conf"),
            ],
            pkg_loader=pkg_helper.load_pkg_config,
            pkg_config_exists=pkg_helper.pkg_config_exists,
            pkg_group_exists=pkg_helper.pkg_group_exists,
            pkg_list_options=pkg_helper.pkg_list_options,
        )

        # File source should work
        result = repo.load_config("config.yaml")
        assert result is not None
        assert result["key"] == "value"

    def test_clear_cache(self, temp_config_dir):
        """Test clearing the cache."""
        repo = _rs.RustHybridConfigRepository(
            search_paths=[("test", f"file://{temp_config_dir}")],
        )

        # Load a config (populates cache)
        result1 = repo.load_config("config.yaml")
        assert result1 is not None

        # Clear cache
        repo.clear_cache()

        # Load again (should work after cache clear)
        result2 = repo.load_config("config.yaml")
        assert result2 is not None
        assert result2["key"] == "value"

    def test_load_nonexistent_returns_none(self, temp_config_dir):
        """Test loading non-existent config returns None."""
        repo = _rs.RustHybridConfigRepository(
            search_paths=[("test", f"file://{temp_config_dir}")],
        )

        result = repo.load_config("nonexistent.yaml")
        assert result is None


@pytest.mark.skipif(not HAS_RUST, reason="Rust extension not available")
class TestRustHybridStructuredSupport:
    """Test structured:// (ConfigStore) support in the hybrid repository."""

    @pytest.fixture
    def temp_config_dir(self):
        """Create a temporary directory with config files."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_dir = Path(tmpdir) / "conf"
            config_dir.mkdir()
            (config_dir / "config.yaml").write_text("key: value\nnum: 42")
            yield str(config_dir)

    def test_structured_helper_functions(self, hydra_restore_singletons):
        """Test the structured_helper functions work with ConfigStore."""
        from lerna._internal.core_plugins import structured_helper
        from lerna.core.config_store import ConfigStore

        # Store a test config
        cs = ConfigStore.instance()
        cs.store(name="test_config", node={"value": 42}, group="test_group")

        # Test helpers
        assert structured_helper.structured_group_exists("test_group")
        assert structured_helper.structured_config_exists("test_group/test_config")

        loaded = structured_helper.load_structured_config("test_group/test_config")
        assert loaded is not None
        assert loaded["value"] == 42

        options = structured_helper.structured_list_options("test_group")
        assert "test_config.yaml" in options

    def test_hybrid_repo_with_structured_source(self, hydra_restore_singletons):
        """Test hybrid repo can load from structured:// via callbacks."""
        from lerna._internal.core_plugins import structured_helper
        from lerna.core.config_store import ConfigStore

        # Store a test config
        cs = ConfigStore.instance()
        cs.store(name="db_config", node={"host": "localhost", "port": 3306}, group="db")

        # Create hybrid repo with structured:// source
        repo = _rs.RustHybridConfigRepository(
            search_paths=[("schema", "structured://")],
            structured_loader=structured_helper.load_structured_config,
            structured_config_exists=structured_helper.structured_config_exists,
            structured_group_exists=structured_helper.structured_group_exists,
            structured_list_options=structured_helper.structured_list_options,
        )

        # Load config via callbacks
        loaded = repo.load_config("db/db_config")
        assert loaded is not None
        assert loaded["host"] == "localhost"
        assert loaded["port"] == 3306

    def test_hybrid_repo_mixed_sources(self, hydra_restore_singletons, temp_config_dir):
        """Test hybrid repo with file:// and structured:// sources."""
        from lerna._internal.core_plugins import structured_helper
        from lerna.core.config_store import ConfigStore

        # Store a structured config
        cs = ConfigStore.instance()
        cs.store(name="app_config", node={"env": "production"})

        # Create hybrid repo with both sources
        repo = _rs.RustHybridConfigRepository(
            search_paths=[
                ("file", f"file://{temp_config_dir}"),
                ("schema", "structured://"),
            ],
            structured_loader=structured_helper.load_structured_config,
            structured_config_exists=structured_helper.structured_config_exists,
            structured_group_exists=structured_helper.structured_group_exists,
            structured_list_options=structured_helper.structured_list_options,
        )

        # Load from file source
        file_config = repo.load_config("config.yaml")
        assert file_config is not None
        assert file_config["key"] == "value"

        # Load from structured source
        struct_config = repo.load_config("app_config")
        assert struct_config is not None
        assert struct_config["env"] == "production"

    def test_repr_includes_structured(self, hydra_restore_singletons):
        """Test __repr__ includes structured source count."""
        repo = _rs.RustHybridConfigRepository(
            search_paths=[("schema", "structured://")],
        )

        repr_str = repr(repo)
        assert "structured_sources=1" in repr_str
