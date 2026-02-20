# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""Tests for the Rust ConfigRepository implementation."""

import os
import tempfile


class TestRustConfigRepository:
    """Tests for RustConfigRepository from lerna.lerna."""

    def test_import(self):
        """Test that RustConfigRepository can be imported."""
        from lerna.lerna import RustConfigRepository

        assert RustConfigRepository is not None

    def test_create_repository(self):
        """Test creating a repository."""
        from lerna.lerna import RustConfigRepository

        with tempfile.TemporaryDirectory() as td:
            # Create a simple config file
            with open(os.path.join(td, "config.yaml"), "w") as f:
                f.write("key: value\n")

            repo = RustConfigRepository([("main", td)])
            assert repo.num_sources() == 1

    def test_config_exists(self):
        """Test config_exists method."""
        from lerna.lerna import RustConfigRepository

        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "config.yaml"), "w") as f:
                f.write("key: value\n")

            repo = RustConfigRepository([("main", td)])

            assert repo.config_exists("config")
            assert not repo.config_exists("nonexistent")

    def test_group_exists(self):
        """Test group_exists method."""
        from lerna.lerna import RustConfigRepository

        with tempfile.TemporaryDirectory() as td:
            os.makedirs(os.path.join(td, "db"))
            with open(os.path.join(td, "db", "mysql.yaml"), "w") as f:
                f.write("driver: mysql\n")

            repo = RustConfigRepository([("main", td)])

            assert repo.group_exists("db")
            assert not repo.group_exists("server")

    def test_load_config(self):
        """Test load_config method."""
        from lerna.lerna import RustConfigRepository

        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "config.yaml"), "w") as f:
                f.write("db:\n  host: localhost\n  port: 3306\n")

            repo = RustConfigRepository([("main", td)])

            config = repo.load_config("config")
            assert config is not None
            assert config["db"]["host"] == "localhost"
            assert config["db"]["port"] == 3306

    def test_load_config_nonexistent(self):
        """Test load_config returns None for nonexistent config."""
        from lerna.lerna import RustConfigRepository

        with tempfile.TemporaryDirectory() as td:
            repo = RustConfigRepository([("main", td)])

            config = repo.load_config("nonexistent")
            assert config is None

    def test_load_group_config(self):
        """Test loading a config from a group."""
        from lerna.lerna import RustConfigRepository

        with tempfile.TemporaryDirectory() as td:
            os.makedirs(os.path.join(td, "db"))
            with open(os.path.join(td, "db", "mysql.yaml"), "w") as f:
                f.write("driver: mysql\nport: 3306\n")

            repo = RustConfigRepository([("main", td)])

            config = repo.load_config("db/mysql")
            assert config is not None
            assert config["driver"] == "mysql"
            assert config["port"] == 3306

    def test_get_group_options(self):
        """Test get_group_options method."""
        from lerna.lerna import RustConfigRepository

        with tempfile.TemporaryDirectory() as td:
            os.makedirs(os.path.join(td, "db"))
            with open(os.path.join(td, "db", "mysql.yaml"), "w") as f:
                f.write("driver: mysql\n")
            with open(os.path.join(td, "db", "postgres.yaml"), "w") as f:
                f.write("driver: postgres\n")

            repo = RustConfigRepository([("main", td)])

            options = repo.get_group_options("db")
            assert "mysql" in options
            assert "postgres" in options

    def test_load_config_full(self):
        """Test load_config_full returns full result with header."""
        from lerna.lerna import RustConfigRepository

        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "config.yaml"), "w") as f:
                f.write("# @package _global_\nkey: value\n")

            repo = RustConfigRepository([("main", td)])

            result = repo.load_config_full("config")
            assert result is not None
            assert "config" in result
            assert "header" in result
            assert "provider" in result
            assert result["config"]["key"] == "value"

    def test_multiple_sources(self):
        """Test repository with multiple sources."""
        from lerna.lerna import RustConfigRepository

        with tempfile.TemporaryDirectory() as td1, tempfile.TemporaryDirectory() as td2:
            # Create config in first source
            with open(os.path.join(td1, "config.yaml"), "w") as f:
                f.write("source: first\n")

            # Create different config in second source
            os.makedirs(os.path.join(td2, "db"))
            with open(os.path.join(td2, "db", "mysql.yaml"), "w") as f:
                f.write("driver: mysql\n")

            repo = RustConfigRepository([("first", td1), ("second", td2)])

            assert repo.num_sources() == 2
            assert repo.config_exists("config")
            assert repo.config_exists("db/mysql")
            assert repo.group_exists("db")

    def test_nested_groups(self):
        """Test nested config groups."""
        from lerna.lerna import RustConfigRepository

        with tempfile.TemporaryDirectory() as td:
            os.makedirs(os.path.join(td, "db", "connection"))
            with open(os.path.join(td, "db", "connection", "pool.yaml"), "w") as f:
                f.write("size: 10\n")

            repo = RustConfigRepository([("main", td)])

            assert repo.group_exists("db")
            assert repo.group_exists("db/connection")
            assert repo.config_exists("db/connection/pool")

            config = repo.load_config("db/connection/pool")
            assert config is not None
            assert config["size"] == 10

    def test_complex_config(self):
        """Test loading complex nested config."""
        from lerna.lerna import RustConfigRepository

        with tempfile.TemporaryDirectory() as td:
            yaml_content = """
database:
  host: localhost
  port: 5432
  credentials:
    username: admin
    password: secret
  options:
    - pool_size: 10
    - timeout: 30
"""
            with open(os.path.join(td, "config.yaml"), "w") as f:
                f.write(yaml_content)

            repo = RustConfigRepository([("main", td)])

            config = repo.load_config("config")
            assert config is not None
            assert config["database"]["host"] == "localhost"
            assert config["database"]["credentials"]["username"] == "admin"
            assert len(config["database"]["options"]) == 2


class TestRustCachingConfigRepository:
    """Tests for RustCachingConfigRepository with load_and_compose."""

    def test_import(self):
        """Test that RustCachingConfigRepository can be imported."""
        from lerna.lerna import RustCachingConfigRepository

        assert RustCachingConfigRepository is not None

    def test_create_caching_repository(self):
        """Test creating a caching repository."""
        from lerna.lerna import RustCachingConfigRepository

        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "config.yaml"), "w") as f:
                f.write("key: value\n")

            repo = RustCachingConfigRepository([("main", td)])
            assert repo is not None

    def test_load_config(self):
        """Test basic load_config method."""
        from lerna.lerna import RustCachingConfigRepository

        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "config.yaml"), "w") as f:
                f.write("db:\n  host: localhost\n")

            repo = RustCachingConfigRepository([("main", td)])
            config = repo.load_config("config")
            assert config is not None
            assert config["db"]["host"] == "localhost"

    def test_load_and_compose_simple(self):
        """Test load_and_compose with a simple config."""
        from lerna.lerna import RustCachingConfigRepository

        with tempfile.TemporaryDirectory() as td:
            # Create a simple config with no defaults
            with open(os.path.join(td, "config.yaml"), "w") as f:
                f.write("key: value\nnum: 42\n")

            repo = RustCachingConfigRepository([("main", td)])
            result = repo.load_and_compose("config")

            assert "config" in result
            assert "defaults" in result
            assert result["config"]["key"] == "value"
            assert result["config"]["num"] == 42

    def test_load_and_compose_with_defaults(self):
        """Test load_and_compose with defaults list."""
        from lerna.lerna import RustCachingConfigRepository

        with tempfile.TemporaryDirectory() as td:
            # Create db group
            os.makedirs(os.path.join(td, "db"))
            with open(os.path.join(td, "db", "mysql.yaml"), "w") as f:
                f.write("driver: mysql\nport: 3306\n")

            # Create main config with defaults
            with open(os.path.join(td, "config.yaml"), "w") as f:
                f.write("defaults:\n  - db: mysql\n\napp_name: myapp\n")

            repo = RustCachingConfigRepository([("main", td)])
            result = repo.load_and_compose("config")

            assert "config" in result
            config = result["config"]
            # The db config should be merged
            assert config["app_name"] == "myapp"

    def test_load_and_compose_with_overrides(self):
        """Test load_and_compose with command line overrides."""
        from lerna.lerna import RustCachingConfigRepository

        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "config.yaml"), "w") as f:
                f.write("db:\n  host: localhost\n  port: 3306\n")

            repo = RustCachingConfigRepository([("main", td)])
            result = repo.load_and_compose("config", ["db.port=5432"])

            config = result["config"]
            assert config["db"]["port"] == 5432

    def test_clear_cache(self):
        """Test that clear_cache works."""
        from lerna.lerna import RustCachingConfigRepository

        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "config.yaml"), "w") as f:
                f.write("key: value\n")

            repo = RustCachingConfigRepository([("main", td)])
            repo.load_config("config")
            repo.clear_cache()  # Should not raise
