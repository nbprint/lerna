# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from collections.abc import Callable
from typing import Any

from omegaconf import DictConfig, OmegaConf
from pytest import mark, param

from lerna._internal import utils
from lerna.tests import data


def test_get_args_accepts_overrides_around_flags() -> None:
    args = utils.get_args(["task=1", "--multirun", "db=mysql"])

    assert args.multirun
    assert args.overrides == ["task=1", "db=mysql"]


@mark.parametrize(
    "matrix,expected",
    [
        ([["a"]], [1]),
        ([["a", "bb"]], [1, 2]),
        ([["a", "bb"], ["aa", "b"]], [2, 2]),
        ([["a"], ["aa", "b"]], [2, 1]),
        ([["a", "aa"], ["bb"]], [2, 2]),
    ],
)
def test_get_column_widths(matrix: Any, expected: Any) -> None:
    assert utils.get_column_widths(matrix) == expected


@mark.parametrize(
    "config, expected",
    [
        param(OmegaConf.create({"_target_": "foo"}), "foo", id="ObjectConf:target"),
    ],
)
def test_get_class_name(config: DictConfig, expected: Any) -> None:
    assert utils._get_cls_name(config) == expected


@mark.parametrize(
    "task_function, expected_file, expected_module",
    [
        param(data.foo, None, "lerna.tests.data", id="function"),
        param(data.foo_main_module, data.__file__, None, id="function-main-module"),
        param(data.Bar, None, "lerna.tests.data", id="class"),
        param(data.bar_instance, None, "lerna.tests.data", id="class_inst"),
        param(data.bar_instance_main_module, None, None, id="class_inst-main-module"),
    ],
)
def test_detect_calling_file_or_module_from_task_function(
    task_function: Callable[..., None],
    expected_file: str | None,
    expected_module: str | None,
) -> None:
    file, module = utils.detect_calling_file_or_module_from_task_function(task_function)
    assert file == expected_file
    assert module == expected_module
