# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
import inspect
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List

from omegaconf._utils import type_str

from lerna._internal.grammar.utils import is_type_matching
from lerna.core.override_parser.types import QuotedString
from lerna.errors import HydraException


@dataclass
class FunctionCall:
    name: str
    args: List[Any]
    kwargs: Dict[str, Any]


@dataclass
class Functions:
    # Names of functions that the Rust parser handles natively
    RUST_NATIVE_FUNCTIONS: set = field(
        default_factory=lambda: {
            "choice",
            "range",
            "interval",
            "shuffle",
            "sort",
            "tag",
            "glob",
            "int",
            "float",
            "str",
            "bool",
            "json_str",
            "extend_list",
        }
    )

    definitions: Dict[str, inspect.Signature] = field(default_factory=dict)
    functions: Dict[str, Callable[..., Any]] = field(default_factory=dict)
    # Tracks which Rust-native functions have been overridden by user registrations
    user_overrides: set = field(default_factory=set)

    def register(self, name: str, func: Callable[..., Any], _builtin: bool = False) -> None:
        if name in self.definitions:
            raise HydraException(f"Function named '{name}' is already registered")

        self.definitions[name] = inspect.signature(func)
        self.functions[name] = func

        # Track if this is a user override of a Rust-native function
        # _builtin=True is used by create_functions() to avoid marking defaults as overrides
        if not _builtin and name in self.RUST_NATIVE_FUNCTIONS:
            self.user_overrides.add(name)

    def eval(self, func: FunctionCall) -> Any:
        if func.name not in self.definitions:
            raise HydraException(f"Unknown function '{func.name}'\nAvailable: {','.join(sorted(self.definitions.keys()))}\n")
        sig = self.definitions[func.name]

        # unquote strings in args
        # Handle both Python QuotedString dataclass and Rust PyQuotedString class
        args = []
        for arg in func.args:
            if isinstance(arg, QuotedString):
                arg = arg.text
            elif hasattr(arg, "text") and hasattr(arg, "quote"):
                # Rust PyQuotedString has text and quote attributes
                arg = arg.text
            args.append(arg)

        # Unquote strings in kwargs values
        kwargs = {}
        for key, val in func.kwargs.items():
            if isinstance(val, QuotedString):
                val = val.text
            elif hasattr(val, "text") and hasattr(val, "quote"):
                # Rust PyQuotedString has text and quote attributes
                val = val.text
            kwargs[key] = val

        bound = sig.bind(*args, **kwargs)

        for idx, arg in enumerate(bound.arguments.items()):
            name = arg[0]
            value = arg[1]
            expected_type = sig.parameters[name].annotation
            if sig.parameters[name].kind == inspect.Parameter.VAR_POSITIONAL:
                for iidx, v in enumerate(value):
                    if not is_type_matching(v, expected_type):
                        raise TypeError(f"mismatch type argument {name}[{iidx}]: {type_str(type(v))} is incompatible with {type_str(expected_type)}")

            else:
                if not is_type_matching(value, expected_type):
                    raise TypeError(f"mismatch type argument {name}: {type_str(type(value))} is incompatible with {type_str(expected_type)}")

        return self.functions[func.name](*bound.args, **bound.kwargs)
