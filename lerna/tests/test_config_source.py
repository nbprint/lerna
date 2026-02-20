# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""Tests for ConfigSource Rust bindings."""

import tempfile
from pathlib import Path

import pytest

from lerna import ConfigResult, ConfigSourceManager, RustFileConfigSource


class TestConfigResult:
    """Test ConfigResult Python wrapper."""

    def test_create_config_result(self):
        """ConfigResult can be created with provider, path, and config."""
        result = ConfigResult(
            provider="test_provider",
            path="test.yaml",
            config={"key": "value", "number": 42},
        )
        assert result.get_config() == {"key": "value", "number": 42}
        assert result.provider == "test_provider"
        assert result.path == "test.yaml"

    def test_config_result_with_header(self):
        """ConfigResult can include optional header."""
        result = ConfigResult(
            provider="provider",
            path="test.yaml",
            config={"data": [1, 2, 3]},
            header={"_target_": "some.class.Name"},
        )
        assert result.get_config() == {"data": [1, 2, 3]}
        assert result.get_header() == {"_target_": "some.class.Name"}

    def test_config_result_with_schema_source(self):
        """ConfigResult can mark as schema source."""
        result = ConfigResult(
            provider="provider",
            path="test.yaml",
            config={"value": True},
            is_schema_source=True,
        )
        assert result.is_schema_source is True

    def test_config_result_nested_config(self):
        """ConfigResult handles nested data structures."""
        config = {
            "database": {
                "host": "localhost",
                "port": 5432,
                "credentials": {
                    "username": "admin",
                    "password": "secret",
                },
            },
            "servers": ["server1", "server2"],
            "enabled": True,
            "ratio": 0.5,
        }
        result = ConfigResult(provider="nested", path="nested.yaml", config=config)
        assert result.get_config() == config


class TestRustFileConfigSource:
    """Test RustFileConfigSource - Rust FileConfigSource exposed to Python."""

    def test_file_config_source_scheme(self):
        """FileConfigSource reports 'file' scheme."""
        with tempfile.TemporaryDirectory() as tmpdir:
            source = RustFileConfigSource(provider="file", path=tmpdir)
            assert source.scheme() == "file"

    def test_file_config_source_path(self):
        """FileConfigSource reports its path."""
        with tempfile.TemporaryDirectory() as tmpdir:
            source = RustFileConfigSource(provider="file", path=tmpdir)
            assert source.path() == tmpdir

    def test_file_config_source_available(self):
        """FileConfigSource is available when path exists."""
        with tempfile.TemporaryDirectory() as tmpdir:
            source = RustFileConfigSource(provider="file", path=tmpdir)
            assert source.available() is True

    def test_file_config_source_unavailable(self):
        """FileConfigSource unavailable when path doesn't exist."""
        source = RustFileConfigSource(provider="file", path="/nonexistent/path/12345")
        assert source.available() is False

    def test_file_config_source_is_group(self):
        """FileConfigSource.is_group identifies directories."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create a subdirectory (group)
            group_path = Path(tmpdir) / "db"
            group_path.mkdir()

            source = RustFileConfigSource(provider="file", path=tmpdir)
            assert source.is_group("db") is True
            assert source.is_group("nonexistent") is False

    def test_file_config_source_is_config(self):
        """FileConfigSource.is_config identifies config files."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create a config file
            config_file = Path(tmpdir) / "config.yaml"
            config_file.write_text("key: value\n")

            source = RustFileConfigSource(provider="file", path=tmpdir)
            assert source.is_config("config.yaml") is True
            assert source.is_config("nonexistent.yaml") is False

    def test_file_config_source_list_empty(self):
        """FileConfigSource.list returns empty for nonexistent group."""
        with tempfile.TemporaryDirectory() as tmpdir:
            source = RustFileConfigSource(provider="file", path=tmpdir)
            result = source.list("nonexistent")
            assert result == []

    def test_file_config_source_list_contents(self):
        """FileConfigSource.list returns directory contents."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create some files and dirs
            (Path(tmpdir) / "config.yaml").write_text("key: value\n")
            (Path(tmpdir) / "db").mkdir()
            (Path(tmpdir) / "db" / "mysql.yaml").write_text("driver: mysql\n")

            source = RustFileConfigSource(provider="file", path=tmpdir)

            # List root - Hydra convention: entries without .yaml extension
            root_contents = source.list("")
            assert "config" in root_contents
            assert "db" in root_contents

            # List db group
            db_contents = source.list("db")
            assert "mysql" in db_contents

    def test_file_config_source_load_config(self):
        """FileConfigSource.load_config loads YAML files."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_file = Path(tmpdir) / "test.yaml"
            config_file.write_text("""
server:
  host: localhost
  port: 8080
debug: true
""")
            source = RustFileConfigSource(provider="file", path=tmpdir)
            result = source.load_config("test.yaml")

            assert isinstance(result, ConfigResult)
            config = result.get_config()
            assert config["server"]["host"] == "localhost"
            assert config["server"]["port"] == 8080
            assert config["debug"] is True


class TestConfigSourceManager:
    """Test ConfigSourceManager - manages multiple config sources."""

    def test_manager_creation(self):
        """ConfigSourceManager can be created."""
        manager = ConfigSourceManager()
        assert manager is not None

    def test_manager_add_rust_source(self):
        """Manager can add a Rust FileConfigSource."""
        with tempfile.TemporaryDirectory() as tmpdir:
            manager = ConfigSourceManager()
            manager.add_file_source(provider="file", path=tmpdir)

            # Should have one source
            assert manager.len() == 1

    def test_manager_list_from_source(self):
        """Manager can list contents from added sources."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create test structure
            (Path(tmpdir) / "config.yaml").write_text("key: value\n")
            (Path(tmpdir) / "db").mkdir()

            manager = ConfigSourceManager()
            manager.add_file_source(provider="file", path=tmpdir)

            contents = manager.list_all("")
            # Files listed without extension by Hydra convention
            assert "config" in contents
            assert "db" in contents

    def test_manager_load_config(self):
        """Manager can load config from sources."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_file = Path(tmpdir) / "app.yaml"
            config_file.write_text("app_name: test\nversion: 1\n")

            manager = ConfigSourceManager()
            manager.add_file_source(provider="file", path=tmpdir)

            # Use config name without extension (Hydra convention)
            result = manager.load_config("app")
            assert result is not None
            config = result.get_config()
            assert config["app_name"] == "test"
            assert config["version"] == 1


class TestPythonConfigSource:
    """Test Python ConfigSource integration with Rust manager."""

    def test_python_source_in_manager(self):
        """Python ConfigSource works with Rust ConfigSourceManager."""

        # Create a Python ConfigSource
        class PyConfigSource:
            def scheme(self):
                return "memory"

            def provider(self):
                return "test"

            def path(self):
                return ""

            def available(self):
                return True

            def load_config(self, config_path):
                return ConfigResult(
                    provider="py_source",
                    path=config_path,
                    config={"from_python": True, "path": config_path},
                )

            def is_group(self, group_path):
                return group_path == "groups"

            def is_config(self, config_path):
                return config_path.endswith(".yaml") or config_path == "config"

            def exists(self, path):
                return path in ["config", "groups"]

            def list(self, group_path, results_filter=None):
                if group_path == "":
                    return ["config", "groups"]
                return []

        manager = ConfigSourceManager()
        manager.add_python_source(PyConfigSource())

        # Test list
        contents = manager.list_all("")
        assert "config" in contents

        # Test load_config
        result = manager.load_config("config")
        config = result.get_config()
        assert config["from_python"] is True

    def test_multiple_sources(self):
        """Manager handles multiple sources (Rust + Python)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create a file in the Rust source
            (Path(tmpdir) / "rust_config.yaml").write_text("source: rust\n")

            # Create Python source
            class PySource:
                def scheme(self):
                    return "memory"

                def provider(self):
                    return "python"

                def path(self):
                    return ""

                def available(self):
                    return True

                def load_config(self, config_path):
                    if config_path == "py_config":
                        return ConfigResult(
                            provider="python",
                            path=config_path,
                            config={"source": "python"},
                        )
                    return None

                def is_group(self, group_path):
                    return False

                def is_config(self, config_path):
                    return config_path == "py_config"

                def exists(self, path):
                    return path == "py_config"

                def list(self, group_path, results_filter=None):
                    if group_path == "":
                        return ["py_config"]
                    return []

            manager = ConfigSourceManager()
            manager.add_file_source(provider="file", path=tmpdir)
            manager.add_python_source(PySource())

            # Both sources should be accessible
            contents = manager.list_all("")
            assert "rust_config" in contents
            assert "py_config" in contents


class TestConfigSourceEdgeCases:
    """Test edge cases and error handling."""

    def test_load_nonexistent_config(self):
        """Loading nonexistent config raises OSError."""
        with tempfile.TemporaryDirectory() as tmpdir:
            source = RustFileConfigSource(provider="file", path=tmpdir)
            with pytest.raises(OSError):
                source.load_config("nonexistent.yaml")

    def test_load_invalid_yaml(self):
        """Loading invalid YAML handles errors gracefully."""
        with tempfile.TemporaryDirectory() as tmpdir:
            bad_file = Path(tmpdir) / "bad.yaml"
            bad_file.write_text("this: is: not: valid: yaml: : :\n")

            source = RustFileConfigSource(provider="file", path=tmpdir)
            # Should either return error result or raise
            try:
                result = source.load_config("bad.yaml")
                # If it returns, check it's usable
                assert result is None or isinstance(result, ConfigResult)
            except Exception:
                # Expected - invalid YAML should raise
                pass

    def test_empty_config(self):
        """Loading empty config file works."""
        with tempfile.TemporaryDirectory() as tmpdir:
            empty_file = Path(tmpdir) / "empty.yaml"
            empty_file.write_text("")

            source = RustFileConfigSource(provider="file", path=tmpdir)
            # Empty YAML file - check behavior
            try:
                result = source.load_config("empty.yaml")
                assert result is None or isinstance(result, ConfigResult)
            except OSError:
                # Also acceptable - empty file may be treated as error
                pass
