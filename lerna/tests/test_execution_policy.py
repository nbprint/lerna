# SPDX-FileCopyrightText: Contributors to Hydra
# SPDX-License-Identifier: MIT

import re
from pathlib import Path
from typing import Any

from pytest import mark, param, raises

from lerna._internal import logging_config
from lerna._internal.execution_policy import _capture_execution_policy, _execution_policy_digest
from lerna._internal.instantiate import _instantiate2
from lerna.errors import InstantiationException
from lerna.utils import instantiate

_DIGEST_CALL = re.compile(r"_validated_execution_policy\(\s*\"([0-9a-f]{64})\"", re.MULTILINE)


@mark.parametrize("module", [param(_instantiate2, id="instantiate"), param(logging_config, id="logging_config")])
def test_embedded_policy_digest_is_current(module: Any) -> None:
    """The digest pinned at each call site must match the policy tables.

    Editing the tables in execution_policy.py changes the digest and must be
    accompanied by updating the constants these call sites pass.
    """
    embedded = _DIGEST_CALL.findall(Path(module.__file__).read_text())
    assert embedded, f"no policy digest found in {module.__name__}"
    expected = _execution_policy_digest(_capture_execution_policy())
    for digest in embedded:
        assert digest == expected


@mark.parametrize(
    "value",
    [
        param("???", id="missing"),
        param("${configured}", id="interpolation"),
        param({"nested": "???"}, id="nested_missing"),
        param({"nested": "${configured}"}, id="nested_interpolation"),
        param(["???"], id="list_missing"),
        param(("${configured}",), id="tuple_interpolation"),
    ],
)
def test_callsite_override_rejects_omegaconf_syntax(value: Any) -> None:
    config = {"_target_": "lerna.tests.instantiate.AClass", "configured": 10}
    with raises(InstantiationException, match="Call-site override"):
        instantiate(config, a=value, b=20, c=30)


@mark.parametrize("value", [param("???", id="missing"), param("${value}", id="interpolation")])
def test_callsite_positional_override_rejects_omegaconf_syntax(value: str) -> None:
    config = {"_target_": "lerna.tests.instantiate.ArgsClass"}
    with raises(InstantiationException, match=re.escape("Call-site override '_args_.0'")):
        instantiate(config, value)
