# Copyright (c) Lerna Contributors. All Rights Reserved
from pathlib import Path

from omegaconf import OmegaConf

import lerna.lerna as rs
from lerna import compose, initialize, initialize_config_dir
from lerna.core.override_parser.overrides_parser import OverridesParser
from lerna.utils import instantiate


def test_rust_parser_and_sweeps() -> None:
    parser = OverridesParser.create()
    assert parser.parse_override("values=[1,2,3]").value() == [1, 2, 3]
    assert sorted(parser.parse_override("values=shuffle([1,2,3])").value()) == [1, 2, 3]
    assert rs.expand_sweeps(["db=mysql,postgres", "port=range(1,3)"]) == [
        ["db=mysql", "port=1"],
        ["db=mysql", "port=2"],
        ["db=postgres", "port=1"],
        ["db=postgres", "port=2"],
    ]


def test_compose_without_config_files() -> None:
    with initialize(config_path=None):
        cfg = compose(overrides=["+name=pyodide", "+count=3"])
    assert OmegaConf.to_container(cfg) == {"name": "pyodide", "count": 3}


def test_compose_config_files(tmp_path: Path) -> None:
    (tmp_path / "db").mkdir()
    (tmp_path / "db" / "mysql.yaml").write_text("host: localhost\nport: 3306\n")
    (tmp_path / "config.yaml").write_text("defaults:\n  - db: mysql\n  - _self_\ntags: [base]\nurl: ${db.host}:${db.port}\n")

    with initialize_config_dir(config_dir=str(tmp_path)):
        cfg = compose(config_name="config", overrides=["db.port=5432", "tags=append(browser)"])

    assert cfg.db.port == 5432
    assert cfg.tags == ["base", "browser"]
    assert cfg.url == "localhost:5432"


def test_instantiate() -> None:
    cfg = {"_target_": "builtins.dict", "name": "pyodide", "count": 3}
    assert instantiate(cfg, _execution_whitelist_=["builtins.dict"]) == {"name": "pyodide", "count": 3}
