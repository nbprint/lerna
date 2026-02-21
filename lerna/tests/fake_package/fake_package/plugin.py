# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from hydra.core.config_search_path import ConfigSearchPath
from hydra.core.config_store import ConfigStore
from hydra.plugins.search_path_plugin import SearchPathPlugin


class FakePackageSearchPathPlugin(SearchPathPlugin):
    def manipulate_search_path(self, search_path: ConfigSearchPath) -> None:
        inst = ConfigStore.instance()
        inst.store(name="fake-package", node=None, group="fake_package", provider="fake-package")
        search_path.append(provider="fake-package", path="pkg://fake_package/hydra")
