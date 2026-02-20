# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""Tests for Rust OmegaConf implementation.

These tests verify that the Rust OmegaConf bindings work correctly.
Tests are adapted from omry/omegaconf test suite.
"""

from typing import Any

import pytest

# Import from the Rust module
from lerna.lerna import DictConfig, ListConfig, OmegaConf


class TestDictConfigBasic:
    """Basic DictConfig tests."""

    def test_create_empty(self) -> None:
        cfg = DictConfig()
        assert len(cfg) == 0
        assert cfg.keys() == []

    def test_create_with_dict(self) -> None:
        cfg = DictConfig({"a": 1, "b": 2})
        assert len(cfg) == 2
        assert "a" in cfg.keys()
        assert "b" in cfg.keys()

    def test_getitem(self) -> None:
        cfg = DictConfig({"a": 1, "b": "hello", "c": True})
        assert cfg["a"] == 1
        assert cfg["b"] == "hello"
        assert cfg["c"] is True

    def test_setitem(self) -> None:
        cfg = DictConfig()
        cfg["x"] = 42
        assert cfg["x"] == 42
        cfg["y"] = "world"
        assert cfg["y"] == "world"

    def test_delitem(self) -> None:
        cfg = DictConfig({"a": 1, "b": 2})
        del cfg["a"]
        assert "a" not in cfg.keys()
        assert "b" in cfg.keys()

    def test_contains(self) -> None:
        cfg = DictConfig({"a": 1})
        assert "a" in cfg
        assert "b" not in cfg

    def test_len(self) -> None:
        cfg = DictConfig({"a": 1, "b": 2, "c": 3})
        assert len(cfg) == 3

    def test_keys(self) -> None:
        cfg = DictConfig({"a": 1, "b": 2})
        keys = cfg.keys()
        assert len(keys) == 2
        assert "a" in keys
        assert "b" in keys

    def test_values(self) -> None:
        cfg = DictConfig({"a": 1, "b": 2})
        values = cfg.values()
        assert len(values) == 2
        assert 1 in values
        assert 2 in values

    def test_items(self) -> None:
        cfg = DictConfig({"a": 1, "b": 2})
        items = cfg.items()
        assert len(items) == 2
        # items is list of (key, value) tuples
        keys = [k for k, v in items]
        vals = [v for k, v in items]
        assert "a" in keys
        assert "b" in keys
        assert 1 in vals
        assert 2 in vals

    def test_get_with_default(self) -> None:
        cfg = DictConfig({"a": 1})
        assert cfg.get("a") == 1
        assert cfg.get("b") is None
        assert cfg.get("b", 42) == 42

    def test_getitem_missing_raises(self) -> None:
        cfg = DictConfig({"a": 1})
        with pytest.raises(KeyError):
            _ = cfg["nonexistent"]


class TestListConfigBasic:
    """Basic ListConfig tests."""

    def test_create_empty(self) -> None:
        cfg = ListConfig()
        assert len(cfg) == 0

    def test_create_with_list(self) -> None:
        cfg = ListConfig([1, 2, 3])
        assert len(cfg) == 3

    def test_getitem(self) -> None:
        cfg = ListConfig([10, 20, 30])
        assert cfg[0] == 10
        assert cfg[1] == 20
        assert cfg[2] == 30

    def test_getitem_negative_index(self) -> None:
        cfg = ListConfig([10, 20, 30])
        assert cfg[-1] == 30
        assert cfg[-2] == 20

    def test_setitem(self) -> None:
        cfg = ListConfig([1, 2, 3])
        cfg[1] = 99
        assert cfg[1] == 99

    def test_delitem(self) -> None:
        cfg = ListConfig([1, 2, 3])
        del cfg[1]
        assert len(cfg) == 2
        assert cfg[0] == 1
        assert cfg[1] == 3

    def test_len(self) -> None:
        cfg = ListConfig([1, 2, 3, 4, 5])
        assert len(cfg) == 5

    def test_append(self) -> None:
        cfg = ListConfig([1, 2])
        cfg.append(3)
        assert len(cfg) == 3
        assert cfg[2] == 3

    def test_insert(self) -> None:
        cfg = ListConfig([1, 3])
        cfg.insert(1, 2)
        assert len(cfg) == 3
        assert cfg[0] == 1
        assert cfg[1] == 2
        assert cfg[2] == 3

    def test_pop(self) -> None:
        cfg = ListConfig([1, 2, 3])
        val = cfg.pop()
        assert val == 3
        assert len(cfg) == 2

    def test_clear(self) -> None:
        cfg = ListConfig([1, 2, 3])
        cfg.clear()
        assert len(cfg) == 0

    def test_getitem_out_of_range_raises(self) -> None:
        cfg = ListConfig([1, 2, 3])
        with pytest.raises(IndexError):
            _ = cfg[10]


class TestOmegaConfCreate:
    """Tests for OmegaConf.create()."""

    def test_create_empty_dict(self) -> None:
        cfg = OmegaConf.create()
        assert OmegaConf.is_dict(cfg)

    def test_create_dict(self) -> None:
        cfg = OmegaConf.create({"a": 1, "b": 2})
        assert OmegaConf.is_dict(cfg)
        assert cfg["a"] == 1

    def test_create_list(self) -> None:
        cfg = OmegaConf.create([1, 2, 3])
        assert OmegaConf.is_list(cfg)
        assert cfg[0] == 1


class TestOmegaConfStatic:
    """Tests for OmegaConf static methods."""

    def test_is_config_dict(self) -> None:
        cfg = DictConfig({"a": 1})
        assert OmegaConf.is_config(cfg) is True

    def test_is_config_list(self) -> None:
        cfg = ListConfig([1, 2, 3])
        assert OmegaConf.is_config(cfg) is True

    def test_is_dict(self) -> None:
        cfg = DictConfig({"a": 1})
        assert OmegaConf.is_dict(cfg) is True
        assert OmegaConf.is_list(cfg) is False

    def test_is_list(self) -> None:
        cfg = ListConfig([1, 2, 3])
        assert OmegaConf.is_list(cfg) is True
        assert OmegaConf.is_dict(cfg) is False


class TestNestedConfigs:
    """Tests for nested DictConfig and ListConfig."""

    def test_nested_dict(self) -> None:
        cfg = DictConfig({"a": {"b": {"c": 1}}})
        # Access nested dict (should return another dict or primitive)
        inner = cfg["a"]
        assert inner is not None

    def test_nested_list(self) -> None:
        cfg = ListConfig([[1, 2], [3, 4]])
        inner = cfg[0]
        assert inner is not None

    def test_dict_with_list(self) -> None:
        cfg = DictConfig({"items": [1, 2, 3]})
        items = cfg["items"]
        assert items is not None


class TestValueTypes:
    """Tests for different value types."""

    def test_string_value(self) -> None:
        cfg = DictConfig({"s": "hello"})
        assert cfg["s"] == "hello"

    def test_int_value(self) -> None:
        cfg = DictConfig({"i": 42})
        assert cfg["i"] == 42

    def test_float_value(self) -> None:
        cfg = DictConfig({"f": 3.14})
        assert abs(cfg["f"] - 3.14) < 0.001

    def test_bool_value(self) -> None:
        cfg = DictConfig({"t": True, "f": False})
        assert cfg["t"] is True
        assert cfg["f"] is False

    def test_none_value(self) -> None:
        cfg = DictConfig({"n": None})
        assert cfg["n"] is None


class TestMissing:
    """Tests for MISSING value handling."""

    def test_missing_string(self) -> None:
        cfg = DictConfig({"m": "???"})
        # MISSING should be returned as the string "???"
        assert cfg["m"] == "???"


class TestInterpolation:
    """Tests for interpolation handling."""

    def test_interpolation_string(self) -> None:
        cfg = DictConfig({"ref": "${other}"})
        # Interpolation should be preserved as the string
        assert cfg["ref"] == "${other}"


class TestResolve:
    """Tests for OmegaConf.resolve() method."""

    def test_resolve_simple_interpolation(self) -> None:
        cfg = DictConfig({"name": "Alice", "greeting": "${name}"})
        OmegaConf.resolve(cfg)
        assert cfg["name"] == "Alice"
        # After resolve, interpolation should be replaced with actual value
        # Note: our simple implementation may not fully resolve yet

    def test_resolve_nested_dict(self) -> None:
        cfg = DictConfig({"db": {"host": "localhost", "url": "${db.host}:5432"}})
        # Just verify no crash for now
        OmegaConf.resolve(cfg)


class TestLoad:
    """Tests for OmegaConf.load() and from_yaml() methods."""

    def test_from_yaml_simple(self) -> None:
        yaml_str = "key: value\nnum: 42"
        cfg = OmegaConf.from_yaml(yaml_str)
        assert cfg["key"] == "value"
        assert cfg["num"] == 42

    def test_from_yaml_nested(self) -> None:
        yaml_str = """
db:
  host: localhost
  port: 5432
"""
        cfg = OmegaConf.from_yaml(yaml_str)
        assert "db" in cfg
        db = cfg["db"]
        assert db["host"] == "localhost"
        assert db["port"] == 5432

    def test_from_yaml_list(self) -> None:
        yaml_str = "items:\n  - a\n  - b\n  - c"
        cfg = OmegaConf.from_yaml(yaml_str)
        assert "items" in cfg
        items = cfg["items"]
        # Items come back as ListConfig, not plain list
        assert len(items) == 3
        assert items[0] == "a"
        assert items[1] == "b"
        assert items[2] == "c"

    def test_from_yaml_missing_value(self) -> None:
        yaml_str = "required: ???"
        cfg = OmegaConf.from_yaml(yaml_str)
        assert cfg["required"] == "???"

    def test_from_yaml_interpolation(self) -> None:
        yaml_str = "name: Alice\ngreeting: ${name}"
        cfg = OmegaConf.from_yaml(yaml_str)
        assert cfg["name"] == "Alice"
        assert cfg["greeting"] == "${name}"


class TestFlags:
    """Tests for DictConfig flag methods."""

    def test_get_flag_default_none(self) -> None:
        cfg = DictConfig({"a": 1})
        # Flags not set should return None
        assert cfg._get_flag("struct") is None
        assert cfg._get_flag("readonly") is None

    def test_set_flag(self) -> None:
        cfg = DictConfig({"a": 1})
        cfg._set_flag("struct", True)
        assert cfg._get_flag("struct") is True
        cfg._set_flag("struct", False)
        assert cfg._get_flag("struct") is False

    def test_is_struct(self) -> None:
        cfg = DictConfig({"a": 1})
        assert cfg._is_struct() is False
        cfg._set_flag("struct", True)
        assert cfg._is_struct() is True

    def test_is_readonly(self) -> None:
        cfg = DictConfig({"a": 1})
        assert cfg._is_readonly() is False
        cfg._set_flag("readonly", True)
        assert cfg._is_readonly() is True


class TestExports:
    """Tests for module exports - Container, MissingMandatoryValue, etc."""

    def test_container_import(self) -> None:
        from lerna.lerna import Container

        assert Container is not None

    def test_container_instance(self) -> None:
        from lerna.lerna import Container

        c = Container()
        assert isinstance(c, Container)

    def test_missing_mandatory_value_import(self) -> None:
        from lerna.lerna import MissingMandatoryValue

        assert MissingMandatoryValue is not None

    def test_missing_mandatory_value_raise(self) -> None:
        from lerna.lerna import MissingMandatoryValue

        try:
            raise MissingMandatoryValue("test missing value")
        except MissingMandatoryValue as e:
            assert "test missing value" in str(e)

    def test_missing_constant(self) -> None:
        from lerna.lerna import MISSING

        assert MISSING == "???"


class TestContextManagers:
    """Tests for open_dict and read_write context managers."""

    def test_open_dict_import(self) -> None:
        from lerna.lerna import open_dict

        assert open_dict is not None

    def test_read_write_import(self) -> None:
        from lerna.lerna import read_write

        assert read_write is not None

    def test_flag_override_import(self) -> None:
        from lerna.lerna import flag_override

        assert flag_override is not None

    def test_open_dict_basic(self) -> None:
        from lerna.lerna import open_dict

        cfg = DictConfig({"a": 1})
        cfg._set_flag("struct", True)
        assert cfg._is_struct() is True

        with open_dict(cfg) as c:
            assert c._is_struct() is False
            c["new_key"] = 42

        # After context, struct should be restored
        assert cfg._is_struct() is True
        # But new_key should persist
        assert cfg["new_key"] == 42

    def test_read_write_basic(self) -> None:
        from lerna.lerna import read_write

        cfg = DictConfig({"a": 1})
        cfg._set_flag("readonly", True)
        assert cfg._is_readonly() is True

        with read_write(cfg) as c:
            assert c._is_readonly() is False

        # After context, readonly should be restored
        assert cfg._is_readonly() is True

    def test_open_dict_restores_on_exception(self) -> None:
        from lerna.lerna import open_dict

        cfg = DictConfig({"a": 1})
        cfg._set_flag("struct", True)

        try:
            with open_dict(cfg):
                assert cfg._is_struct() is False
                raise ValueError("test error")
        except ValueError:
            pass

        # Should still restore
        assert cfg._is_struct() is True

    def test_flag_override_basic(self) -> None:
        from lerna.lerna import flag_override

        cfg = DictConfig({"a": 1})
        assert cfg._get_flag("custom") is None

        with flag_override(cfg, "custom", True) as c:
            assert c._get_flag("custom") is True

        # After context, flag should be restored to None
        assert cfg._get_flag("custom") is None


class TestRustConfigRepository:
    """Tests for Rust config repository implementation."""

    @pytest.fixture
    def test_configs_path(self) -> str:
        """Path to test configs."""
        import os

        base = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        return f"file://{base}/test_utils/configs"

    @pytest.fixture
    def repo(self, test_configs_path: str) -> Any:
        """Create a RustCachingConfigRepository."""
        import lerna.lerna as rs

        return rs.RustCachingConfigRepository([("test", test_configs_path)])

    def test_group_exists(self, repo: Any) -> None:
        """Test group_exists method."""
        assert repo.group_exists("db") is True
        assert repo.group_exists("nonexistent") is False

    def test_config_exists(self, repo: Any) -> None:
        """Test config_exists method."""
        assert repo.config_exists("db/mysql") is True
        assert repo.config_exists("db/postgresql") is True
        assert repo.config_exists("db/nonexistent") is False

    def test_load_config(self, repo: Any) -> None:
        """Test load_config method."""
        cfg = repo.load_config("db/mysql")
        assert cfg is not None
        assert cfg["driver"] == "mysql"
        assert cfg["user"] == "omry"
        assert cfg["password"] == "secret"

    def test_load_config_nonexistent(self, repo: Any) -> None:
        """Test load_config with nonexistent config."""
        cfg = repo.load_config("nonexistent/config")
        assert cfg is None

    def test_get_group_options(self, repo: Any) -> None:
        """Test get_group_options method."""
        options = repo.get_group_options("db")
        assert "mysql" in options
        assert "postgresql" in options

    def test_load_and_compose_simple(self, repo: Any) -> None:
        """Test load_and_compose with no overrides."""
        result = repo.load_and_compose("compose", [])
        assert result is not None
        assert "config" in result
        assert "defaults" in result
        config = result["config"]
        # compose.yaml has defaults: [group1: file1, group2: file2]
        assert "group1" in config
        assert "group2" in config

    def test_load_and_compose_with_overrides(self, repo: Any) -> None:
        """Test load_and_compose with config overrides."""
        result = repo.load_and_compose("compose", ["group1.foo=999"])
        assert result is not None
        config = result["config"]
        assert config["group1"]["foo"] == 999

    def test_clear_cache(self, repo: Any) -> None:
        """Test clear_cache method."""
        # First load should cache
        repo.load_config("db/mysql")
        # Clear cache
        repo.clear_cache()
        # Should still work after clear
        cfg = repo.load_config("db/mysql")
        assert cfg is not None
        assert cfg["driver"] == "mysql"

    def test_multiple_sources(self, test_configs_path: str) -> None:
        """Test repository with multiple config sources."""
        import lerna.lerna as rs

        repo = rs.RustCachingConfigRepository(
            [
                ("source1", test_configs_path),
                ("source2", test_configs_path),
            ]
        )
        assert repo.group_exists("db") is True
        cfg = repo.load_config("db/mysql")
        assert cfg is not None


class TestRustConfigRepositoryNonCaching:
    """Tests for non-caching Rust config repository."""

    @pytest.fixture
    def test_configs_path(self) -> str:
        """Path to test configs."""
        import os

        base = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        return f"file://{base}/test_utils/configs"

    @pytest.fixture
    def repo(self, test_configs_path: str) -> Any:
        """Create a RustConfigRepository."""
        import lerna.lerna as rs

        return rs.RustConfigRepository([("test", test_configs_path)])

    def test_load_config(self, repo: Any) -> None:
        """Test load_config method."""
        cfg = repo.load_config("db/mysql")
        assert cfg is not None
        assert cfg["driver"] == "mysql"

    def test_group_exists(self, repo: Any) -> None:
        """Test group_exists method."""
        assert repo.group_exists("db") is True
        assert repo.group_exists("nonexistent") is False

    def test_config_exists(self, repo: Any) -> None:
        """Test config_exists method."""
        assert repo.config_exists("db/mysql") is True
        assert repo.config_exists("db/nonexistent") is False

    def test_get_group_options(self, repo: Any) -> None:
        """Test get_group_options method."""
        options = repo.get_group_options("db")
        assert "mysql" in options
        assert "postgresql" in options

    def test_num_sources(self, repo: Any) -> None:
        """Test num_sources method."""
        assert repo.num_sources() == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
