# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""Benchmarks comparing Python vs Rust implementations."""

import tempfile
from pathlib import Path
from typing import Any

import pytest

# Check if Rust is available
try:
    from lerna import lerna as _rs

    HAS_RUST = True
except ImportError:
    HAS_RUST = False
    _rs = None


@pytest.fixture(scope="module")
def temp_config_dir():
    """Create a temporary directory with config files for benchmarking."""
    with tempfile.TemporaryDirectory() as tmpdir:
        config_dir = Path(tmpdir) / "conf"
        config_dir.mkdir()

        # Create main config
        (config_dir / "config.yaml").write_text("""
defaults:
  - db: mysql
  - server: apache
  - _self_

app:
  name: myapp
  version: 1.0.0
  debug: true
""")

        # Create db group
        db_dir = config_dir / "db"
        db_dir.mkdir()
        (db_dir / "mysql.yaml").write_text("""
driver: mysql
host: localhost
port: 3306
user: root
password: secret
database: mydb
pool_size: 10
timeout: 30
""")
        (db_dir / "postgres.yaml").write_text("""
driver: postgres
host: localhost
port: 5432
user: postgres
password: secret
database: mydb
pool_size: 20
timeout: 60
""")

        # Create server group
        server_dir = config_dir / "server"
        server_dir.mkdir()
        (server_dir / "apache.yaml").write_text("""
name: apache
host: 0.0.0.0
port: 80
workers: 4
max_connections: 1000
keepalive: true
""")
        (server_dir / "nginx.yaml").write_text("""
name: nginx
host: 0.0.0.0
port: 80
workers: auto
max_connections: 10000
keepalive: true
""")

        yield str(config_dir)


@pytest.mark.skipif(not HAS_RUST, reason="Rust not available")
class TestRustBenchmarks:
    """Benchmarks for Rust implementations."""

    def test_rust_yaml_parse(self, benchmark: Any) -> None:
        """Benchmark YAML parsing with Rust."""
        yaml_content = """
key1: value1
key2: 42
nested:
  a: 1
  b: 2
  c: 3
list:
  - item1
  - item2
  - item3
"""
        result = benchmark(_rs.parse_yaml, yaml_content)
        assert result is not None
        assert result["key1"] == "value1"

    def test_rust_config_repo_load(self, temp_config_dir: str, benchmark: Any) -> None:
        """Benchmark config loading with Rust ConfigRepository."""
        repo = _rs.RustCachingConfigRepository([("test", f"file://{temp_config_dir}")])

        def load_config():
            repo.clear_cache()
            return repo.load_config("config.yaml")

        result = benchmark(load_config)
        assert result is not None

    def test_rust_config_repo_load_cached(self, temp_config_dir: str, benchmark: Any) -> None:
        """Benchmark config loading with Rust ConfigRepository (cached)."""
        repo = _rs.RustCachingConfigRepository([("test", f"file://{temp_config_dir}")])
        # Prime the cache
        repo.load_config("config.yaml")

        result = benchmark(repo.load_config, "config.yaml")
        assert result is not None

    def test_rust_compose(self, temp_config_dir: str, benchmark: Any) -> None:
        """Benchmark full config compose with Rust."""
        repo = _rs.RustCachingConfigRepository([("test", f"file://{temp_config_dir}")])

        def compose():
            repo.clear_cache()
            return repo.load_and_compose("config.yaml", [])

        result = benchmark(compose)
        assert result is not None
        assert "defaults" in result

    def test_rust_compose_with_overrides(self, temp_config_dir: str, benchmark: Any) -> None:
        """Benchmark config compose with overrides."""
        repo = _rs.RustCachingConfigRepository([("test", f"file://{temp_config_dir}")])

        def compose():
            repo.clear_cache()
            return repo.load_and_compose("config.yaml", ["db=postgres", "server=nginx"])

        result = benchmark(compose)
        assert result is not None


class TestPythonBenchmarks:
    """Benchmarks for Python implementations."""

    def test_python_yaml_parse(self, benchmark: Any) -> None:
        """Benchmark YAML parsing with Python."""
        import yaml

        yaml_content = """
key1: value1
key2: 42
nested:
  a: 1
  b: 2
  c: 3
list:
  - item1
  - item2
  - item3
"""
        result = benchmark(yaml.safe_load, yaml_content)
        assert result is not None
        assert result["key1"] == "value1"

    def test_python_config_loader(self, benchmark: Any) -> None:
        """Benchmark config loading with Python ConfigLoader."""
        from lerna._internal.config_loader_impl import ConfigLoaderImpl
        from lerna._internal.utils import create_config_search_path
        from lerna.types import RunMode

        loader = ConfigLoaderImpl(config_search_path=create_config_search_path("pkg://lerna.test_utils.configs"))

        result = benchmark(
            loader.load_configuration,
            config_name="config",
            overrides=[],
            run_mode=RunMode.RUN,
        )
        assert result is not None
