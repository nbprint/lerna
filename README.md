# lerna

`lerna` is a collection of plugins and extensions for [hydra](https://hydra.cc)

[![Build Status](https://github.com/timkpaine/lerna/actions/workflows/build.yaml/badge.svg?branch=main&event=push)](https://github.com/timkpaine/lerna/actions/workflows/build.yaml)
[![codecov](https://codecov.io/gh/timkpaine/lerna/branch/main/graph/badge.svg)](https://codecov.io/gh/timkpaine/lerna)
[![License](https://img.shields.io/github/license/timkpaine/lerna)](https://github.com/timkpaine/lerna)
[![PyPI](https://img.shields.io/pypi/v/lerna.svg)](https://pypi.python.org/pypi/lerna)

## Overview

`lerna` extends `hydra` with added functionality.
Due to the long release intervals of `hydra`, some of this functionality may move to `hydra`, or remain in `lerna` indefinitely.

### SearchPath Plugins via Entrypoints

This is a standalone implementation of [this open Pull Request](https://github.com/facebookresearch/hydra/pull/3052)

```toml
[project.entry-points."hydra.lernaplugins"]
fake-package = "fake_package.plugin"
```

This will check `fake_package.plugin` for `hydra` search path plugins, without the need to deal with `hydra`'s `hydra_plugins` installation.


> [!NOTE]
> This library was generated using [copier](https://copier.readthedocs.io/en/stable/) from the [Base Python Project Template repository](https://github.com/python-project-templates/base).
