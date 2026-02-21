# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved

from pathlib import Path

from lerna.test_utils.test_utils import normalize_path_for_override, run_python_script


def test_rerun(tmpdir: Path) -> None:
    cmd = [
        "examples/experimental/rerun/my_app.py",
        f'hydra.run.dir="{normalize_path_for_override(tmpdir)}"',
        "hydra.job.chdir=True",
        "hydra.hydra_logging.formatters.simple.format='[HYDRA] %(message)s'",
        "hydra.job_logging.formatters.simple.format='[JOB] %(message)s'",
    ]

    result, _err = run_python_script(cmd)
    assert "[JOB] cfg.foo=bar" in result
