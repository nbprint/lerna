# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from lerna.core.plugins import Plugins
from lerna.plugins.plugin import Plugin


class ExampleRegisteredPlugin(Plugin):
    def __init__(self, v: int) -> None:
        self.v = v

    def add(self, x: int) -> int:
        return self.v + x


def register_example_plugin() -> None:
    """The Hydra user should call this function before invoking @lerna.main"""
    Plugins.instance().register(ExampleRegisteredPlugin)
