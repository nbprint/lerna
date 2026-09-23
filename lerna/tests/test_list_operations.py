# Copyright (c) Lerna Contributors. All Rights Reserved
"""Tests for list operations from CLI.

These tests verify the fix for Hydra issues:
- #1547: Append to list from CLI
- #2477: Delete item from ListConfig by index
"""

import pytest
from omegaconf import OmegaConf

from lerna import compose, initialize_config_dir
from lerna.core.global_hydra import GlobalHydra
from lerna.core.override_parser.overrides_parser import OverridesParser
from lerna.core.override_parser.types import ListOperationType, OverrideType
from lerna.errors import HydraException


class TestListOperationParsing:
    """Test parsing of list operation functions."""

    @pytest.fixture
    def parser(self):
        return OverridesParser.create()

    def test_legacy_enum_names_alias_canonical_names(self):
        assert ListOperationType.REMOVE_AT is ListOperationType.POP
        assert ListOperationType.REMOVE_VALUE is ListOperationType.REMOVE
        assert ListOperationType.EXTEND_FROM is ListOperationType.EXTEND

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

    def test_append_unique_parsing(self, parser):
        result = parser.parse_override("tags=append_unique(a,b)")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.APPEND_UNIQUE
        assert result.list_index is None
        assert result._value == ["a", "b"]

    def test_remove_all_parsing(self, parser):
        result = parser.parse_override("tags=remove_all(a,b)")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.REMOVE_ALL
        assert result.list_index is None
        assert result._value == ["a", "b"]

    def test_extend_from_parsing(self, parser):
        result = parser.parse_override("tags=extend_from(${other.tags})")
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == ListOperationType.EXTEND_FROM
        assert result.list_index is None
        assert result._value == ["${other.tags}"]

    @pytest.mark.parametrize(
        ("override", "operation", "index", "end_index", "value"),
        [
            ("tags=extend(${other.tags})", ListOperationType.EXTEND_FROM, None, None, ["${other.tags}"]),
            ("tags=pop(-1)", ListOperationType.REMOVE_AT, -1, None, []),
            ("tags=remove(old_tag)", ListOperationType.REMOVE_VALUE, None, None, ["old_tag"]),
            ("tags=clear()", ListOperationType.CLEAR, None, None, []),
            ("tags=delete_slice(1)", ListOperationType.DELETE_SLICE, 1, None, []),
            ("tags=delete_slice(1,3)", ListOperationType.DELETE_SLICE, 1, 3, []),
        ],
    )
    def test_python_style_operation_parsing(self, parser, override, operation, index, end_index, value):
        result = parser.parse_override(override)
        assert result.type == OverrideType.EXTEND_LIST
        assert result.list_operation == operation
        assert result.list_index == index
        assert result.list_end_index == end_index
        assert result._value == value

    @pytest.mark.parametrize(
        "override",
        [
            "tags=pop()",
            "tags=pop(0,1)",
            "tags=remove()",
            "tags=remove(a,b)",
            "tags=clear(value)",
            "tags=delete_slice()",
            "tags=delete_slice(0,1,2)",
            "tags=delete_slice(start)",
            "tags=delete_slice(0,stop)",
        ],
    )
    def test_python_style_operations_validate_arguments(self, parser, override):
        with pytest.raises(HydraException):
            parser.parse_override(override)

    @pytest.mark.parametrize("override", ["tags=extend_from()", "tags=extend_from(${one},${two})", "tags=extend_from(value)"])
    def test_extend_from_requires_one_interpolation(self, parser, override):
        with pytest.raises(Exception, match="extend_from.*exactly one interpolation"):
            parser.parse_override(override)


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
duplicates:
    - rest
    - outputs
    - rest
source:
    label: source-label
    values:
        - trigger
        - [market_data, quotes]
        - name: ${...label}
empty: []
"""
        )
        return str(conf_dir)

    def test_append_single(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=append(four)"])
            assert list(cfg["tags"]) == ["one", "two", "three", "four"]

    def test_append_multiple(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=append(four,five)"])
            assert list(cfg["tags"]) == ["one", "two", "three", "four", "five"]

    def test_prepend_single(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=prepend(zero)"])
            assert list(cfg["tags"]) == ["zero", "one", "two", "three"]

    def test_prepend_multiple(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=prepend(zero,half)"])
            assert list(cfg["tags"]) == ["zero", "half", "one", "two", "three"]

    def test_insert_at_beginning(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=insert(0,zero)"])
            assert list(cfg["tags"]) == ["zero", "one", "two", "three"]

    def test_insert_in_middle(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=insert(1,one-half)"])
            assert list(cfg["tags"]) == ["one", "one-half", "two", "three"]

    def test_insert_at_end(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=insert(3,four)"])
            assert list(cfg["tags"]) == ["one", "two", "three", "four"]

    def test_remove_at_first(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=remove_at(0)"])
            assert list(cfg["tags"]) == ["two", "three"]

    def test_remove_at_middle(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=remove_at(1)"])
            assert list(cfg["tags"]) == ["one", "three"]

    def test_remove_at_last(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=remove_at(2)"])
            assert list(cfg["tags"]) == ["one", "two"]

    def test_remove_at_negative_index(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=remove_at(-1)"])
            assert list(cfg["tags"]) == ["one", "two"]

    def test_remove_value(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=remove_value(two)"])
            assert list(cfg["tags"]) == ["one", "three"]

    def test_list_clear(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=list_clear()"])
            assert list(cfg["tags"]) == []

    def test_multiple_operations_sequential(self, config_dir):
        """Test that multiple list operations work sequentially."""
        with initialize_config_dir(config_dir=config_dir):
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
        with initialize_config_dir(config_dir=str(conf_dir)):
            cfg = compose(config_name="config", overrides=["db.hosts=append(replica2)"])
            assert list(cfg["db"]["hosts"]) == ["localhost", "replica1", "replica2"]

    def test_append_unique_preserves_existing_duplicates(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["duplicates=append_unique(rest,metrics,metrics)"])
            assert list(cfg.duplicates) == ["rest", "outputs", "rest", "metrics"]

    def test_append_unique_compares_resolved_structures(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(
                config_name="config",
                overrides=[
                    "+candidate={name:source-label}",
                    "items=append({name:${candidate.name}})",
                    "items=append_unique({name:source-label},[nested,list])",
                ],
            )
            assert list(cfg["items"]) == ["a", "b", "c", {"name": "source-label"}, ["nested", "list"]]
            assert OmegaConf.to_container(cfg["items"], resolve=False)[3] == {"name": "${candidate.name}"}

    def test_remove_all_removes_every_resolved_match(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["duplicates=remove_all(rest,missing)"])
            assert list(cfg.duplicates) == ["outputs"]

    def test_extend_from_splices_only_source_list(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=["tags=extend_from(${source.values})"])
            assert OmegaConf.to_container(cfg.tags, resolve=True) == [
                "one",
                "two",
                "three",
                "trigger",
                ["market_data", "quotes"],
                {"name": "source-label"},
            ]
            assert OmegaConf.to_container(cfg.tags, resolve=False)[-1] == {"name": "${source.label}"}

    def test_extend_from_empty_and_self_sources(self, config_dir):
        with initialize_config_dir(config_dir=config_dir):
            empty = compose(config_name="config", overrides=["tags=extend_from(${empty})"])
            assert list(empty.tags) == ["one", "two", "three"]

        GlobalHydra.instance().clear()
        with initialize_config_dir(config_dir=config_dir):
            same = compose(config_name="config", overrides=["tags=extend_from(${tags})"])
            assert list(same.tags) == ["one", "two", "three", "one", "two", "three"]

    @pytest.mark.parametrize(
        ("override", "expected"),
        [
            ("tags=pop(-1)", ["one", "two"]),
            ("tags=remove(two)", ["one", "three"]),
            ("tags=clear()", []),
            ("tags=extend(${source.values})", ["one", "two", "three", "trigger", ["market_data", "quotes"], {"name": "source-label"}]),
            ("tags=delete_slice(1,2)", ["one", "three"]),
            ("tags=delete_slice(1)", ["one"]),
            ("tags=delete_slice(-2,-1)", ["one", "three"]),
            ("tags=delete_slice(10,20)", ["one", "two", "three"]),
        ],
    )
    def test_python_style_operations(self, config_dir, override, expected):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=[override])
            assert OmegaConf.to_container(cfg.tags, resolve=True) == expected

    @pytest.mark.parametrize(
        ("overrides", "expected"),
        [
            (["tags=append(four)", "tags=append_unique(four,five)"], ["one", "two", "three", "four", "five"]),
            (["tags=prepend(zero)", "tags=append_unique(zero,four)"], ["zero", "one", "two", "three", "four"]),
            (["tags=insert(1,one-half)", "tags=append_unique(one-half,four)"], ["one", "one-half", "two", "three", "four"]),
            (["tags=remove_value(two)", "tags=append_unique(two,four)"], ["one", "three", "two", "four"]),
            (["tags=list_clear()", "tags=append_unique(one,two)"], ["one", "two"]),
            (["tags=append_unique(four)", "tags=append(four)"], ["one", "two", "three", "four", "four"]),
            (["tags=append_unique(four)", "tags=prepend(four)"], ["four", "one", "two", "three", "four"]),
            (["tags=append_unique(four)", "tags=insert(0,four)"], ["four", "one", "two", "three", "four"]),
            (["tags=append_unique(four)", "tags=remove_value(four)"], ["one", "two", "three"]),
            (["tags=append_unique(four)", "tags=list_clear()"], []),
        ],
    )
    def test_append_unique_sequential_behavior(self, config_dir, overrides, expected):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=overrides)
            assert list(cfg.tags) == expected

    @pytest.mark.parametrize(
        ("overrides", "expected"),
        [
            (["tags=append(two)", "tags=remove_all(two)"], ["one", "three"]),
            (["tags=prepend(two)", "tags=remove_all(two)"], ["one", "three"]),
            (["tags=insert(1,two)", "tags=remove_all(two)"], ["one", "three"]),
            (["tags=remove_value(two)", "tags=remove_all(two)"], ["one", "three"]),
            (["tags=list_clear()", "tags=remove_all(two)"], []),
            (["tags=remove_all(two)", "tags=append(two)"], ["one", "three", "two"]),
            (["tags=remove_all(two)", "tags=prepend(two)"], ["two", "one", "three"]),
            (["tags=remove_all(two)", "tags=insert(1,two)"], ["one", "two", "three"]),
            (["tags=remove_all(two)", "tags=remove_value(one)"], ["three"]),
            (["tags=remove_all(two)", "tags=list_clear()"], []),
        ],
    )
    def test_remove_all_sequential_behavior(self, config_dir, overrides, expected):
        with initialize_config_dir(config_dir=config_dir):
            cfg = compose(config_name="config", overrides=overrides)
            assert list(cfg.tags) == expected


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
        with initialize_config_dir(config_dir=config_dir), pytest.raises(Exception, match="not a list"):
            compose(config_name="config", overrides=["name=append(new)"])

    def test_remove_at_out_of_bounds(self, config_dir):
        """Remove at out-of-bounds index should fail."""
        with initialize_config_dir(config_dir=config_dir), pytest.raises(Exception, match="Cannot remove item"):
            compose(config_name="config", overrides=["tags=remove_at(10)"])

    @pytest.mark.parametrize(
        ("override", "message"),
        [
            ("name=extend_from(${tags})", "destination 'name'.*not a list"),
            ("tags=extend_from(${missing})", "source 'missing'.*destination 'tags'.*does not exist"),
            ("tags=extend_from(${name})", "source 'name'.*destination 'tags'.*not a list"),
        ],
    )
    def test_extend_from_reports_source_and_destination(self, config_dir, override, message):
        with initialize_config_dir(config_dir=config_dir), pytest.raises(Exception, match=message):
            compose(config_name="config", overrides=[override])
