# Copyright (c) Lerna Contributors. All Rights Reserved
"""Tests for list operations from CLI (append, prepend, insert, remove_at, remove_value, list_clear).

These tests verify the fix for Hydra issues:
- #1547: Append to list from CLI
- #2477: Delete item from ListConfig by index
"""

import pytest

from lerna import compose, initialize_config_dir
from lerna.core.global_hydra import GlobalHydra
from lerna.core.override_parser.overrides_parser import OverridesParser
from lerna.core.override_parser.types import ListOperationType, OverrideType


class TestListOperationParsing:
    """Test parsing of list operation functions."""

    @pytest.fixture
    def parser(self):
        return OverridesParser.create()

    def test_append_parsing(self, parser):
        result = parser.parse_override("tags=append(new_tag)")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.APPEND
        assert result.list_index is None
        assert result._value == ["new_tag"]

    def test_append_multiple(self, parser):
        result = parser.parse_override("tags=append(a,b,c)")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.APPEND
        assert result._value == ["a", "b", "c"]

    def test_prepend_parsing(self, parser):
        result = parser.parse_override("tags=prepend(first)")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.PREPEND
        assert result.list_index is None
        assert result._value == ["first"]

    def test_prepend_multiple(self, parser):
        result = parser.parse_override("tags=prepend(a,b,c)")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.PREPEND
        assert result._value == ["a", "b", "c"]

    def test_insert_parsing(self, parser):
        result = parser.parse_override("tags=insert(2,middle)")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.INSERT
        assert result.list_index == 2
        assert result._value == ["middle"]

    def test_insert_at_beginning(self, parser):
        result = parser.parse_override("tags=insert(0,first)")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.INSERT
        assert result.list_index == 0
        assert result._value == ["first"]

    def test_remove_at_parsing(self, parser):
        result = parser.parse_override("tags=remove_at(0)")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.REMOVE_AT
        assert result.list_index == 0
        assert result._value == []

    def test_remove_at_negative_index(self, parser):
        result = parser.parse_override("tags=remove_at(-1)")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.REMOVE_AT
        assert result.list_index == -1

    def test_remove_value_parsing(self, parser):
        result = parser.parse_override("tags=remove_value(old_tag)")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.REMOVE_VALUE
        assert result.list_index is None
        assert result._value == ["old_tag"]

    def test_list_clear_parsing(self, parser):
        result = parser.parse_override("tags=list_clear()")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.CLEAR
        assert result.list_index is None
        assert result._value == []


class TestListOperationsIntegration:
    """Test list operations in full compose workflow."""

    @pytest.fixture(autouse=True)
    def cleanup(self):
        """Clean up GlobalHydra before and after each test."""
        GlobalHydra.instance().clear()
        yield
        GlobalHydra.instance().clear()

    @pytest.fixture
    def config_dir(self, tmp_path):
        """Create a temporary config directory with a test config."""
        conf_dir = tmp_path / "conf"
        conf_dir.mkdir()
        config_file = conf_dir / "config.yaml"
        config_file.write_text(
            """
tags:
  - one
  - two
  - three
items:
  - a
  - b
  - c
"""
        )
        return str(conf_dir)

    def test_append_single(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=append(four)"])
            assert list(cfg["tags"]) == ["one", "two", "three", "four"]

    def test_append_multiple(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=append(four,five)"])
            assert list(cfg["tags"]) == ["one", "two", "three", "four", "five"]

    def test_prepend_single(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=prepend(zero)"])
            assert list(cfg["tags"]) == ["zero", "one", "two", "three"]

    def test_prepend_multiple(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=prepend(zero,half)"])
            assert list(cfg["tags"]) == ["zero", "half", "one", "two", "three"]

    def test_insert_at_beginning(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=insert(0,zero)"])
            assert list(cfg["tags"]) == ["zero", "one", "two", "three"]

    def test_insert_in_middle(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=insert(1,one-half)"])
            assert list(cfg["tags"]) == ["one", "one-half", "two", "three"]

    def test_insert_at_end(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=insert(3,four)"])
            assert list(cfg["tags"]) == ["one", "two", "three", "four"]

    def test_remove_at_first(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=remove_at(0)"])
            assert list(cfg["tags"]) == ["two", "three"]

    def test_remove_at_middle(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=remove_at(1)"])
            assert list(cfg["tags"]) == ["one", "three"]

    def test_remove_at_last(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=remove_at(2)"])
            assert list(cfg["tags"]) == ["one", "two"]

    def test_remove_at_negative_index(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=remove_at(-1)"])
            assert list(cfg["tags"]) == ["one", "two"]

    def test_remove_value(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=remove_value(two)"])
            assert list(cfg["tags"]) == ["one", "three"]

    def test_list_clear(self, config_dir):
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=list_clear()"])
            assert list(cfg["tags"]) == []

    def test_multiple_operations_sequential(self, config_dir):
        """Test that multiple list operations work sequentially."""
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            cfg = compose(
                config_name="config",
                overrides=[
                    "tags=prepend(zero)",
                    "tags=append(four)",
                ],
            )
            assert list(cfg["tags"]) == ["zero", "one", "two", "three", "four"]

    def test_nested_list_path(self, tmp_path):
        """Test list operations on nested paths."""
        conf_dir = tmp_path / "conf"
        conf_dir.mkdir()
        config_file = conf_dir / "config.yaml"
        config_file.write_text(
            """
db:
  hosts:
    - localhost
    - replica1
"""
        )
        with initialize_config_dir(version_base=None, config_dir=str(conf_dir)):
            cfg = compose(config_name="config", overrides=["db.hosts=append(replica2)"])
            assert list(cfg["db"]["hosts"]) == ["localhost", "replica1", "replica2"]


class TestListOperationErrors:
    """Test error handling for list operations."""

    @pytest.fixture(autouse=True)
    def cleanup(self):
        GlobalHydra.instance().clear()
        yield
        GlobalHydra.instance().clear()

    @pytest.fixture
    def config_dir(self, tmp_path):
        conf_dir = tmp_path / "conf"
        conf_dir.mkdir()
        config_file = conf_dir / "config.yaml"
        config_file.write_text(
            """
tags:
  - one
  - two
name: not_a_list
"""
        )
        return str(conf_dir)

    def test_append_to_non_list_fails(self, config_dir):
        """Cannot append to a non-list value."""
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            with pytest.raises(Exception, match="not a list"):
                compose(config_name="config", overrides=["name=append(new)"])

    def test_remove_at_out_of_bounds(self, config_dir):
        """Remove at out-of-bounds index should fail."""
        with initialize_config_dir(version_base=None, config_dir=config_dir):
            with pytest.raises(Exception, match="Cannot remove item"):
                compose(config_name="config", overrides=["tags=remove_at(10)"])
