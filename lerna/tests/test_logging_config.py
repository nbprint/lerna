# Copyright (c) Lerna Contributors. All Rights Reserved

from omegaconf import OmegaConf
from pytest import raises

from lerna.core.utils import configure_log
from lerna.errors import InstantiationException


def test_logging_rejects_unsafe_factory() -> None:
    cfg = OmegaConf.create(
        {
            "version": 1,
            "handlers": {
                "blocked": {
                    "()": "posix.system",
                    "command": "true",
                }
            },
            "root": {"level": "INFO", "handlers": ["blocked"]},
        }
    )
    with raises(ValueError, match="Unable to configure handler") as exc_info:
        configure_log(cfg)

    cause: BaseException = exc_info.value
    while cause.__cause__ is not None:
        cause = cause.__cause__
    assert isinstance(cause, InstantiationException)
    assert "os.system" in str(cause)
