# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""
Override parser using Rust implementation.

This module provides the OverridesParser class which parses Hydra override
strings using a Rust-based parser (via PyO3). The Rust parser handles all
override syntax including values, sweeps, and functions.

Note: The `functions` parameter is kept for backward compatibility but is
not used - the Rust parser has built-in functions (choice, range, sort, etc.).
"""

import sys
from typing import Any, List, Optional

from lerna._internal.grammar.functions import Functions
from lerna.core.config_loader import ConfigLoader
from lerna.core.override_parser.types import (
    ChoiceSweep,
    Glob,
    IntervalSweep,
    Key,
    ListExtensionOverrideValue,
    ListOperationType,
    Override,
    OverrideType,
    Quote,
    QuotedString,
    RangeSweep,
    ValueType,
)
from lerna.errors import HydraException, OverrideParseException

# Import Rust parser (required)
try:
    import lerna.lerna as _rs
except ImportError:
    print("Error: Rust parser extension not found. Please build the Rust extension with 'maturin build --release'.")
    sys.exit(1)


class OverridesParser:
    """
    Parser for Hydra override strings.

    Uses a Rust-based parser for performance. User-defined functions can be
    passed via the `functions` parameter - they will be called from Rust when
    an unknown function is encountered.
    """

    _rust_parser: Any = None
    _functions: Optional[Functions] = None

    @classmethod
    def create(
        cls,
        config_loader: Optional[ConfigLoader] = None,
        searchpath: Optional[List[str]] = None,
    ) -> "OverridesParser":
        """Create an OverridesParser instance.

        Args:
            config_loader: ConfigLoader for resolving glob sweeps
            searchpath: Optional searchpath from hydra.searchpath config,
                       used to ensure pkg:// sources are available for glob sweeps
        """
        return cls(functions=None, config_loader=config_loader, searchpath=searchpath)

    def __init__(
        self,
        functions: Optional[Functions] = None,
        config_loader: Optional[ConfigLoader] = None,
        searchpath: Optional[List[str]] = None,
    ):
        self.config_loader = config_loader
        self.searchpath = searchpath
        self._functions = functions
        # Pass functions to Rust parser if provided
        if functions is not None:
            self._rust_parser = _rs.OverrideParser(functions)
        else:
            self._rust_parser = _rs.OverrideParser()

    def _parse_with_rust(self, s: str) -> Override:
        """Parse using Rust parser and convert to Python Override."""
        data = self._rust_parser.parse_to_dict(s)
        return _rust_dict_to_override(data, self.config_loader, self.searchpath)

    def parse_rule(self, s: str, rule_name: str) -> Any:
        """Parse a rule using the Rust parser.

        For backward compatibility with tests, we wrap the input and
        parse as an override, then extract the value.
        """
        if rule_name == "override":
            try:
                return self._parse_with_rust(s)
            except Exception as e:
                raise HydraException(f"Parse error: {e}") from e
        elif rule_name in ("listContainer", "dictContainer"):
            # These rules should preserve QuotedString objects
            wrapped = f"_tmp_key_={s}"
            try:
                override = self._parse_with_rust(wrapped)
            except Exception as e:
                raise HydraException(f"Parse error: {e}") from e
            return override._value
        elif rule_name in ("function", "value", "element", "simpleChoiceSweep"):
            # Wrap input as an override value and extract the result
            wrapped = f"_tmp_key_={s}"
            try:
                override = self._parse_with_rust(wrapped)
            except Exception as e:
                # Convert specific evaluation errors to HydraException
                err_msg = str(e)
                if (
                    "Error while evaluating" in err_msg
                    or "OverflowError while evaluating" in err_msg
                    or "ValueError while evaluating" in err_msg
                    or "TypeError while evaluating" in err_msg
                ):
                    # Extract the actual error message from the parse error
                    import re

                    match = re.search(r"((?:OverflowError|ValueError|TypeError|Error) while evaluating.*)", err_msg)
                    if match:
                        raise HydraException(match.group(1)) from None
                # Re-raise with cleaner message
                raise HydraException(f"Parse error: {e}") from e
            # For extend_list, wrap the values back in ListExtensionOverrideValue
            # since parse_rule("value") should return the function result, not the unwrapped value
            if override.type == OverrideType.EXTEND_LIST:
                return ListExtensionOverrideValue(values=override._value)
            return override.value()
        elif rule_name == "primitive":
            # For primitive rule, return the raw value without QuotedString conversion
            # Check for all-whitespace input which should be an error
            if s.strip() == "" and s != "":
                raise HydraException("Trying to parse a primitive that is all whitespaces")
            wrapped = f"_tmp_key_={s}"
            try:
                override = self._parse_with_rust(wrapped)
            except Exception as e:
                raise HydraException(f"Parse error: {e}") from e
            # Access the raw value to preserve QuotedString
            return override._value
        elif rule_name == "key":
            # Parse a key (possibly with @package)
            # Format: key_or_group[@package]
            if "@" in s:
                parts = s.split("@", 1)
                return Key(key_or_group=parts[0], package=parts[1])
            else:
                return Key(key_or_group=s)
        elif rule_name in ("package", "packageOrGroup"):
            # Return the raw string for package rules
            return s
        else:
            raise ValueError(
                f"Unknown rule: {rule_name}. Supported rules: override, function, value, primitive, element, listContainer, dictContainer, simpleChoiceSweep, key, package, packageOrGroup."
            )

    def parse_override(self, s: str) -> Override:
        ret = self.parse_rule(s, "override")
        assert isinstance(ret, Override)
        return ret

    def parse_overrides(self, overrides: List[str]) -> List[Override]:
        ret: List[Override] = []
        for idx, override in enumerate(overrides):
            try:
                parsed = self.parse_rule(override, "override")
            except (HydraException, ValueError) as e:
                msg = f"Error parsing override '{override}'\n{e}"
                raise OverrideParseException(
                    override=override,
                    message=f"Error when parsing index: {idx}, string: {override} out of {overrides}."
                    f"\n{msg}"
                    f"\nSee https://hydra.cc/docs/1.2/advanced/override_grammar/basic for details",
                ) from e.__cause__ if isinstance(e, HydraException) else e
            assert isinstance(parsed, Override)
            parsed.config_loader = self.config_loader
            ret.append(parsed)
        return ret


def create_functions() -> Functions:
    """
    Create a Functions registry with built-in grammar functions.

    Note: This is kept for backward compatibility with tests that pass
    a Functions object to OverridesParser. The Rust parser has its own
    built-in implementations and does not use these Python functions.
    """
    from lerna._internal.grammar import grammar_functions

    functions = Functions()
    # casts - marked as builtin so they don't count as user overrides
    functions.register(name="int", func=grammar_functions.cast_int, _builtin=True)
    functions.register(name="str", func=grammar_functions.cast_str, _builtin=True)
    functions.register(name="bool", func=grammar_functions.cast_bool, _builtin=True)
    functions.register(name="float", func=grammar_functions.cast_float, _builtin=True)
    functions.register(name="json_str", func=grammar_functions.cast_json_str, _builtin=True)
    # sweeps
    functions.register(name="choice", func=grammar_functions.choice, _builtin=True)
    functions.register(name="range", func=grammar_functions.range, _builtin=True)
    functions.register(name="interval", func=grammar_functions.interval, _builtin=True)
    # misc
    functions.register(name="tag", func=grammar_functions.tag, _builtin=True)
    functions.register(name="sort", func=grammar_functions.sort, _builtin=True)
    functions.register(name="shuffle", func=grammar_functions.shuffle, _builtin=True)
    functions.register(name="glob", func=grammar_functions.glob, _builtin=True)
    functions.register(name="extend_list", func=grammar_functions.extend_list, _builtin=True)
    return functions


def _convert_dict_key(key: str) -> Any:
    """Convert dict key string to appropriate Python type."""
    key_lower = key.lower()
    if key_lower == "null":
        return None
    if key_lower == "true":
        return True
    if key_lower == "false":
        return False
    # Try integer
    try:
        return int(key)
    except ValueError:
        pass
    # Try float
    try:
        return float(key)
    except ValueError:
        pass
    return key


def _convert_rust_value(value: Any) -> Any:
    """Convert Rust types to Python types recursively."""
    # Already a Python QuotedString - don't convert again
    if isinstance(value, QuotedString):
        return value
    # Check if it's a Rust QuotedString (has text/quote/with_quotes but not our type)
    if hasattr(value, "text") and hasattr(value, "quote") and hasattr(value, "with_quotes"):
        # It's a Rust QuotedString, convert to Python QuotedString
        quote_str = str(value.quote).lower()  # "single" or "double"
        py_quote = Quote.single if quote_str == "single" else Quote.double
        return QuotedString(text=value.text, quote=py_quote)
    elif isinstance(value, list):
        return [_convert_rust_value(item) for item in value]
    elif isinstance(value, dict):
        return {_convert_dict_key(k): _convert_rust_value(v) for k, v in value.items()}
    else:
        return value


def _parse_list_operation(operation_str: str) -> ListOperationType:
    """Convert list operation string from Rust to Python enum."""
    operation_map = {
        "APPEND": ListOperationType.APPEND,
        "PREPEND": ListOperationType.PREPEND,
        "INSERT": ListOperationType.INSERT,
        "REMOVE_AT": ListOperationType.REMOVE_AT,
        "REMOVE_VALUE": ListOperationType.REMOVE_VALUE,
        "CLEAR": ListOperationType.CLEAR,
    }
    return operation_map.get(operation_str, ListOperationType.APPEND)


def _rust_dict_to_override(
    data: dict,
    config_loader: Optional[ConfigLoader] = None,
    searchpath: Optional[List[str]] = None,
) -> Override:
    """Convert Rust parser output dict to Python Override object.

    Args:
        data: Dict from Rust parser
        config_loader: ConfigLoader for resolving glob sweeps
        searchpath: Optional searchpath from hydra.searchpath config,
                   used to ensure pkg:// sources are available for glob sweeps
    """
    # Map override type strings to enum
    type_map = {
        "CHANGE": OverrideType.CHANGE,
        "ADD": OverrideType.ADD,
        "FORCE_ADD": OverrideType.FORCE_ADD,
        "DEL": OverrideType.DEL,
        "EXTEND_LIST": OverrideType.EXTEND_LIST,
    }
    override_type = type_map[data["type"]]

    # Map value type strings to enum
    value_type_map = {
        "ELEMENT": ValueType.ELEMENT,
        "CHOICE_SWEEP": ValueType.CHOICE_SWEEP,
        "SIMPLE_CHOICE_SWEEP": ValueType.SIMPLE_CHOICE_SWEEP,
        "GLOB_CHOICE_SWEEP": ValueType.GLOB_CHOICE_SWEEP,
        "RANGE_SWEEP": ValueType.RANGE_SWEEP,
        "INTERVAL_SWEEP": ValueType.INTERVAL_SWEEP,
    }

    # For DEL overrides without value, value_type should be None
    if data["value"] is None:
        value_type: Optional[ValueType] = None
    else:
        value_type = value_type_map.get(data["value_type"])

    # Convert value - first convert any Rust types to Python types
    raw_value = _convert_rust_value(data["value"])

    # Initialize list operation fields (only used for EXTEND_LIST type)
    list_operation: Optional[ListOperationType] = None
    list_index: Optional[int] = None

    if raw_value is None:
        value: Any = None
    elif isinstance(raw_value, dict):
        sweep_type = raw_value.get("type")
        if sweep_type == "choice_sweep":
            # Convert choice list items from Rust to Python types
            # At the top level of choice, convert QuotedStrings to plain strings
            # But keep QuotedStrings inside nested structures for correct serialization
            choice_list = []
            for item in raw_value.get("list", []):
                converted_item = _convert_rust_value(item)
                # Convert top-level QuotedStrings to plain strings
                if isinstance(converted_item, QuotedString):
                    converted_item = converted_item.text
                choice_list.append(converted_item)
            value = ChoiceSweep(
                tags=raw_value.get("tags", set()),
                list=choice_list,
                simple_form=raw_value.get("simple_form", False),
                shuffle=raw_value.get("shuffle", False),
            )
        elif sweep_type == "range_sweep":
            # For range sweep, use is_int flag to determine integer vs float values
            start = raw_value.get("start")
            stop = raw_value.get("stop")
            step = raw_value.get("step", 1)
            is_int = raw_value.get("is_int", False)
            # Convert to int if is_int flag is set
            if is_int:
                if start is not None:
                    start = int(start)
                if stop is not None:
                    stop = int(stop)
                if step is not None:
                    step = int(step)
            value = RangeSweep(
                tags=raw_value.get("tags", set()),
                start=start,
                stop=stop,
                step=step,
                shuffle=raw_value.get("shuffle", False),
            )
        elif sweep_type == "interval_sweep":
            # For interval sweep, convert floats to ints only if is_int flag is set
            start = raw_value.get("start")
            end = raw_value.get("end")
            is_int = raw_value.get("is_int", False)
            if is_int:
                if start is not None:
                    start = int(start)
                if end is not None:
                    end = int(end)
            value = IntervalSweep(
                tags=raw_value.get("tags", set()),
                start=start,
                end=end,
            )
        elif sweep_type == "glob_choice_sweep" or raw_value.get("_type") == "glob":
            # Handle both glob_choice_sweep type and _type:glob format
            value = Glob(
                include=raw_value.get("include", []),
                exclude=raw_value.get("exclude", []),
            )
            # Glob is a sweep type
            value_type = ValueType.GLOB_CHOICE_SWEEP
        elif sweep_type == "list_extension":
            # extend_list() can only be used with CHANGE type (no prefix)
            # Using +key=extend_list() or ++key=extend_list() is invalid
            if override_type in (OverrideType.ADD, OverrideType.FORCE_ADD):
                from lerna.errors import OverrideParseException

                raise OverrideParseException(override=data.get("input_line", ""), message="Trying to use override symbols when extending a list")
            # Convert list extension values and change override type
            # Unquote top-level QuotedStrings (similar to choice handling)
            ext_values = []
            for v in raw_value.get("values", []):
                converted = _convert_rust_value(v)
                if isinstance(converted, QuotedString):
                    converted = converted.text
                ext_values.append(converted)
            # Set _value to the raw list (unwrapped), not the ListExtensionOverrideValue wrapper
            # This matches Hydra's behavior in overrides_visitor.py
            value = ext_values
            override_type = OverrideType.EXTEND_LIST
            value_type = ValueType.ELEMENT  # ListExtension uses ELEMENT value_type

            # Extract list operation and index
            operation_str = raw_value.get("operation", "APPEND")
            list_operation = _parse_list_operation(operation_str)
            list_index = raw_value.get("index")
        else:
            # Regular dict value
            value = raw_value
    else:
        value = raw_value

    override = Override(
        type=override_type,
        key_or_group=data["key_or_group"],
        value_type=value_type,
        _value=value,
        package=data.get("package"),
        input_line=data.get("input_line"),
        config_loader=config_loader,
        list_operation=list_operation,
        list_index=list_index,
        searchpath=searchpath,
    )
    override.validate()
    return override
