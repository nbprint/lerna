# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from abc import abstractmethod

from lerna.core.config_search_path import ConfigSearchPath

from .plugin import Plugin


class SearchPathPlugin(Plugin):
    @abstractmethod
    def manipulate_search_path(self, search_path: ConfigSearchPath) -> None: ...
