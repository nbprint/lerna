# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""
Conftest for lerna tests.

This file ensures that the structured config test module is imported before
tests that need it run, so that its ConfigStore entries are available.

It also explicitly imports lerna's pytest fixtures to ensure they take
precedence over hydra-core's fixtures (if hydra-core is installed).
"""

import copy

from lerna.core.singleton import Singleton

# Explicitly import lerna's fixtures to override hydra-core's fixtures
# This ensures our fixtures are used even when hydra-core is installed
from lerna.extra.pytest_plugin import (  # noqa: F401
    hydra_restore_singletons,
    hydra_sweep_runner,
    hydra_task_runner,
)

# Store the initial state BEFORE importing structured configs
_initial_state_without_structured = copy.deepcopy(Singleton.get_state())

# Import the structured config test module to register its configs
import lerna.tests.test_apps.config_source_test.structured  # noqa: F401, E402

# Store the state with the structured configs registered
_initial_state_with_configs = copy.deepcopy(Singleton.get_state())


def pytest_runtest_setup(item):
    """
    Ensure structured configs are available for tests that need them,
    and NOT available for tests that don't expect them.

    The autouse hydra_restore_singletons fixture saves state at the start of
    each test, but it may not include the structured configs if those were
    loaded after the fixture's initial state capture.
    """
    # Tests that need the structured configs
    if "StructuredConfigSource" in item.name or "test_config_repository" in str(item.fspath):
        Singleton.set_state(copy.deepcopy(_initial_state_with_configs))
    else:
        # For other tests, restore to state without the extra structured configs
        # This prevents test_list_groups and similar tests from seeing unexpected groups
        Singleton.set_state(copy.deepcopy(_initial_state_without_structured))
