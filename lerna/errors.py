# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from collections.abc import Sequence


class HydraException(Exception): ...


class CompactHydraException(HydraException): ...


class OverrideParseException(CompactHydraException):
    def __init__(self, override: str, message: str) -> None:
        super().__init__(message)
        self.override = override
        self.message = message


class InstantiationException(CompactHydraException): ...


class ConfigCompositionException(CompactHydraException): ...


class SearchPathException(CompactHydraException): ...


class MissingConfigException(IOError, ConfigCompositionException):
    def __init__(
        self,
        message: str,
        missing_cfg_file: str | None = None,
        options: Sequence[str] | None = None,
    ) -> None:
        super().__init__(message)
        self.missing_cfg_file = missing_cfg_file
        self.options = options


class HydraDeprecationError(HydraException): ...
