# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
from pathlib import Path
from typing import Any, List

from pytest import mark, param

from lerna.test_utils.test_utils import (
    chdir_hydra_root,
    normalize_newlines,
    run_with_error,
)

chdir_hydra_root()


@mark.parametrize(
    "override,expected_substrings",
    [
        param(
            "+key=int(",
            [
                "Error when parsing index: 1, string: +key=int( out of [",
                "Expected ',' or ')' in function arguments",
            ],
            id="parse_error_in_function",
        ),
        param(
            "+key=sort()",
            [
                """Error when parsing index: 1, string: +key=sort() out of [""",
                "sort() requires at least 1 argument",
            ],
            id="empty_sort",
        ),
        param(
            "key=sort(interval(1,10))",
            [
                """Error when parsing index: 1, string: key=sort(interval(1,10)) out of [""",
                "Function 'interval' returns a sweep, which cannot be used here",
            ],
            id="sort_interval",
        ),
        param(
            "+key=choice()",
            [
                """Error when parsing index: 1, string: +key=choice() out of [""",
                "choice() requires at least one argument",
            ],
            id="empty choice",
        ),
        param(
            "+key=extend_list(1, 2, 3)",
            [
                """Error when parsing index: 1, string: +key=extend_list(1, 2, 3) out of [""",
                "Trying to use override symbols when extending a list",
            ],
            id="plus key extend_list",
        ),
        param(
            "key={inner_key=extend_list(1, 2, 3)}",
            [
                """Error when parsing index: 1, string: key={inner_key=extend_list(1, 2, 3)} out of [""",
                "Expected ':' or '='",
            ],
            id="embedded extend_list",
        ),
        param(
            ["+key=choice(choice(a,b))", "-m"],
            [
                """Error when parsing index: 1, string: +key=choice(choice(a,b)) out of [""",
                "Function 'choice' returns a sweep, which cannot be used here",
            ],
            id="nested_choice",
        ),
        param(
            "--config-dir=/dir/not/found",
            [
                f"""Additional config directory '{Path("/dir/not/found").absolute()}' not found

Set the environment variable HYDRA_FULL_ERROR=1 for a complete stack trace.
"""
            ],
            id="config_dir_not_found",
        ),
    ],
)
def test_cli_error(
    tmpdir: Any,
    monkeypatch: Any,
    override: Any,
    expected_substrings: List[str],
) -> None:
    monkeypatch.chdir("lerna/tests/test_apps/app_without_config/")
    if isinstance(override, str):
        override = [override]
    cmd = ["my_app.py", f'hydra.sweep.dir="{str(tmpdir).replace(chr(92), chr(47))}"'] + override
    ret = normalize_newlines(run_with_error(cmd))
    missing_substrings = [s for s in expected_substrings if s.strip() not in ret]
    assert not missing_substrings, (
        f"Result:"
        f"\n---"
        f"\n{ret}"
        f"\n---"
        f"\nDoes not contain the following expected substrings:" + "\n---\n".join(f"{s}" for s in missing_substrings) + "\n---"
    )
