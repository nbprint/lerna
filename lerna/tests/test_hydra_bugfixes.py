# Copyright (c) Lerna Contributors. All Rights Reserved
"""Tests for Hydra bug fixes implemented in Lerna.

These tests verify fixes for:
- #2935: Subfolder config append bug
- #2878: Relative path empty strings in defaults
- #2870: importlib-resources 6.2+ compatibility
"""

from typing import Any

import pytest

from lerna.core.default_element import (
    ConfigDefault,
    GroupDefault,
    _normalize_path,
)
from omegaconf import OmegaConf


class TestNormalizePath:
    """Test the _normalize_path helper function for Hydra #2878."""

    def test_simple_path(self):
        """Normal paths should be unchanged."""
        assert _normalize_path("a/b/c") == "a/b/c"

    def test_empty_path(self):
        """Empty paths should remain empty."""
        assert _normalize_path("") == ""

    def test_single_dotdot(self):
        """Single .. should resolve correctly."""
        assert _normalize_path("dir1/../dir2") == "dir2"

    def test_multiple_dotdot(self):
        """Multiple .. should resolve correctly."""
        assert _normalize_path("a/b/c/../../d") == "a/d"

    def test_dotdot_at_start(self):
        """.. at start should be ignored (can't go above root)."""
        assert _normalize_path("../foo") == "foo"
        assert _normalize_path("../../foo/bar") == "foo/bar"

    def test_dot_segments(self):
        """Single dots should be removed."""
        assert _normalize_path("./dir/child") == "dir/child"
        assert _normalize_path("a/./b/./c") == "a/b/c"

    def test_complex_path(self):
        """Complex paths with mixed . and .. should resolve correctly."""
        assert _normalize_path("dir1/../dir2/./child") == "dir2/child"
        assert _normalize_path("a/b/../c/./d/../e") == "a/c/e"

    def test_empty_segments(self):
        """Empty segments (double slashes) should be handled."""
        assert _normalize_path("a//b/c") == "a/b/c"


class TestConfigDefaultRelativePaths:
    """Test ConfigDefault with relative paths for Hydra #2878."""

    def test_relative_path_with_dotdot(self):
        """ConfigDefault with .. should resolve correctly."""
        cfg = ConfigDefault(path="../dir2/child")
        cfg.update_parent("dir1", "")
        # dir1 + ../dir2/child -> dir2/child
        assert cfg.get_config_path() == "dir2/child"

    def test_relative_group_path_with_dotdot(self):
        """Group path with .. should resolve correctly."""
        cfg = ConfigDefault(path="../dir2/child")
        cfg.update_parent("dir1", "")
        # Group path should be dir2 (parent of child)
        assert cfg.get_group_path() == "dir2"

    def test_deep_relative_path(self):
        """Deep relative path should resolve correctly."""
        cfg = ConfigDefault(path="../../other/config")
        cfg.update_parent("a/b/c", "")
        # a/b/c + ../../other/config -> a/other/config
        assert cfg.get_config_path() == "a/other/config"

    def test_absolute_path_unaffected(self):
        """Absolute paths should not be affected by normalization."""
        cfg = ConfigDefault(path="/absolute/path")
        cfg.update_parent("any/parent", "")
        assert cfg.get_config_path() == "absolute/path"


class TestGroupDefaultRelativePaths:
    """Test GroupDefault with relative paths for Hydra #2878."""

    def test_relative_group_with_dotdot(self):
        """GroupDefault with .. in group should resolve correctly."""
        gd = GroupDefault(group="../other", value="config")
        gd.update_parent("dir1/dir2", "")
        # dir1/dir2 + ../other -> dir1/other
        assert gd.get_group_path() == "dir1/other"

    def test_group_config_path_with_dotdot(self):
        """GroupDefault config path with .. should resolve correctly."""
        gd = GroupDefault(group="../db", value="mysql")
        gd.update_parent("server/configs", "")
        # server/configs + ../db/mysql -> server/db/mysql
        assert gd.get_config_path() == "server/db/mysql"


class TestExternalAppendPaths:
    """Test that external appends (CLI +group=value) use absolute paths.

    This tests the fix for Hydra #2935.
    """

    def test_external_append_flag_set(self):
        """GroupDefault with external_append should have flag set."""
        gd = GroupDefault(group="db", value="postgresql", external_append=True)
        assert gd.external_append is True

    def test_external_append_still_resolves_group(self):
        """external_append GroupDefault still resolves via update_parent.

        Note: The actual fix is in defaults_list.py which passes "" for
        parent_base_dir when external_append=True. This test verifies
        the GroupDefault itself works correctly.
        """
        gd = GroupDefault(group="db", value="postgresql", external_append=True)
        # When fix is applied, parent_base_dir will be "" for external appends
        gd.update_parent("", "")  # This is what the fix does
        assert gd.get_config_path() == "db/postgresql"

    def test_non_external_append_uses_parent(self):
        """Non-external append uses parent_base_dir normally."""
        gd = GroupDefault(group="db", value="postgresql", external_append=False)
        gd.update_parent("server", "")
        assert gd.get_config_path() == "server/db/postgresql"


class TestImportlibResourcesSafeChecks:
    """Test safe is_file/is_dir methods for Hydra #2870."""

    def test_safe_is_file_with_orphan_path(self):
        """_safe_is_file should return False for objects without is_file method."""
        from lerna._internal.core_plugins.importlib_resources_config_source import (
            ImportlibResourcesConfigSource,
        )

        # Simulate OrphanPath-like object without is_file method
        class OrphanPath:
            pass

        result = ImportlibResourcesConfigSource._safe_is_file(OrphanPath())
        assert result is False

    def test_safe_is_file_with_normal_path(self):
        """_safe_is_file should work normally for objects with is_file method."""
        from lerna._internal.core_plugins.importlib_resources_config_source import (
            ImportlibResourcesConfigSource,
        )

        class NormalPath:
            def is_file(self):
                return True

        result = ImportlibResourcesConfigSource._safe_is_file(NormalPath())
        assert result is True

    def test_safe_is_dir_with_orphan_path(self):
        """_safe_is_dir should return False for objects without is_dir method."""
        from lerna._internal.core_plugins.importlib_resources_config_source import (
            ImportlibResourcesConfigSource,
        )

        class OrphanPath:
            pass

        result = ImportlibResourcesConfigSource._safe_is_dir(OrphanPath())
        assert result is False

    def test_safe_is_dir_with_normal_path(self):
        """_safe_is_dir should work normally for objects with is_dir method."""
        from lerna._internal.core_plugins.importlib_resources_config_source import (
            ImportlibResourcesConfigSource,
        )

        class NormalPath:
            def is_dir(self):
                return True

        result = ImportlibResourcesConfigSource._safe_is_dir(NormalPath())
        assert result is True


class TestSubfolderAppendIntegration:
    """Integration tests for subfolder append fix (Hydra #2935).

    Tests the full flow where a config in a subfolder has defaults
    appended from CLI.

    The bug: when config is at server/alpha.yaml and has defaults like:
        defaults:
          - db: mysql

    And you run: python app.py --config-name=server/alpha +db@db_2=postgresql

    Hydra incorrectly looks for server/db/postgresql instead of db/postgresql.
    The fix makes CLI appends use absolute paths from root.
    """

    @pytest.fixture
    def config_dir(self, tmp_path):
        """Create a config structure similar to Hydra #2935 repro."""
        # Create directory structure exactly matching the bug report:
        # conf/
        #   db/
        #     mysql.yaml
        #     postgresql.yaml
        #   server/
        #     alpha.yaml (with relative defaults: - db: mysql)

        conf_dir = tmp_path / "conf"
        db_dir = conf_dir / "db"
        server_dir = conf_dir / "server"

        db_dir.mkdir(parents=True)
        server_dir.mkdir(parents=True)

        # db/mysql.yaml
        (db_dir / "mysql.yaml").write_text("""
driver: mysql
user: root
password: secret
""")

        # db/postgresql.yaml
        (db_dir / "postgresql.yaml").write_text("""
driver: postgresql
user: postgres
password: pg_secret
timeout: 10
""")

        # server/alpha.yaml - relative reference to /db (absolute)
        # Note: We use /db (absolute) because relative db would look in server/db/
        # which doesn't exist in our structure
        (server_dir / "alpha.yaml").write_text("""
defaults:
  - /db: mysql
  - _self_

name: alpha
""")

        return conf_dir

    def test_append_works_with_subfolder_config(self, config_dir):
        """Appending db@db_2 with subfolder config should work (fix for #2935).

        The key test: when appending +db@db_2=postgresql from CLI with a config
        in a subfolder, the appended config should be looked up from root (db/)
        not from the config's directory (server/db/).

        Note: The config ends up under the server package because of how
        package inheritance works with configs in subfolders.
        """
        from lerna import compose, initialize_config_dir
        from lerna.core.global_hydra import GlobalHydra

        GlobalHydra.instance().clear()

        try:
            with initialize_config_dir(config_dir=str(config_dir), version_base=None):
                # This is the test case from Hydra #2935:
                # +db@db_2=postgresql should look for db/postgresql, not server/db/postgresql
                # If the bug existed, this would fail with "Could not find server/db/postgresql"
                cfg = compose(config_name="server/alpha", overrides=["+db@db_2=postgresql"])
                # Config is structured under server package
                assert cfg.server.db.driver == "mysql"
                # Appended db_2 should be postgresql (this is the fix verification)
                assert cfg.server.db_2.driver == "postgresql"
                assert cfg.server.name == "alpha"
        finally:
            GlobalHydra.instance().clear()


class TestGlobPkgSearchpathFix:
    """Tests for Hydra #1942: glob(*) sweep over pkg:// config groups.

    The bug: glob(*) sweep fails to discover options in pkg:// searchpath
    config groups because get_group_options doesn't include searchpath sources.

    The fix: Pass hydra.searchpath to get_group_options when enumerating
    glob sweep options so pkg:// sources are available.
    """

    def test_glob_sweep_uses_searchpath(self):
        """Verify that glob sweep passes searchpath to get_group_options.

        This tests the OverridesParser correctly propagates searchpath
        to Override objects for glob sweep enumeration.
        """
        from lerna.core.override_parser.overrides_parser import OverridesParser
        from lerna.core.override_parser.types import Glob

        # Create parser with a mock searchpath
        searchpath = ["pkg://some_package/conf"]
        parser = OverridesParser.create(config_loader=None, searchpath=searchpath)

        # Parse a glob sweep
        override = parser.parse_override("db=glob(*)")

        # Verify the override has the searchpath
        assert override.searchpath == searchpath
        assert isinstance(override._value, Glob)

    def test_glob_sweep_without_searchpath(self):
        """Verify glob sweep works without searchpath (backward compatibility)."""
        from lerna.core.override_parser.overrides_parser import OverridesParser
        from lerna.core.override_parser.types import Glob

        # Create parser without searchpath (default behavior)
        parser = OverridesParser.create(config_loader=None)

        # Parse a glob sweep
        override = parser.parse_override("db=glob(*)")

        # searchpath should be None
        assert override.searchpath is None
        assert isinstance(override._value, Glob)


class TestInstantiateErrorContext:
    """Tests for Hydra #2235: instantiate error messages missing context.

    The bug: Errors during instantiate lose full_key and object_type
    context that OmegaConf normally provides on direct attribute access.

    The fix: Wrap OmegaConf.resolve() to catch errors and enhance them
    with the config's full_key and object_type context.
    """

    def test_interpolation_error_has_full_key(self):
        """Verify interpolation errors during instantiate include full_key."""
        from dataclasses import dataclass

        from omegaconf.errors import InterpolationKeyError

        from lerna.utils import instantiate

        @dataclass
        class TestClass:
            bad_interp: Any

        cfg = OmegaConf.structured(TestClass(bad_interp="${foo}"))

        with pytest.raises(InterpolationKeyError) as exc_info:
            instantiate(cfg)

        # Verify full_key is set (not None)
        assert exc_info.value.full_key is not None
        assert exc_info.value.full_key == "bad_interp"

    def test_interpolation_error_has_object_type(self):
        """Verify interpolation errors during instantiate include object_type."""
        from dataclasses import dataclass

        from omegaconf.errors import InterpolationKeyError

        from lerna.utils import instantiate

        @dataclass
        class TestClass:
            bad_interp: Any

        cfg = OmegaConf.structured(TestClass(bad_interp="${foo}"))

        with pytest.raises(InterpolationKeyError) as exc_info:
            instantiate(cfg)

        # Verify object_type is set (not None)
        assert exc_info.value.object_type is not None
        assert exc_info.value.object_type == TestClass

    def test_error_message_includes_context(self):
        """Verify the error message string includes full_key and object_type."""
        from dataclasses import dataclass

        from omegaconf.errors import InterpolationKeyError

        from lerna.utils import instantiate

        @dataclass
        class MyClass:
            bad_interp: Any

        cfg = OmegaConf.structured(MyClass(bad_interp="${foo}"))

        with pytest.raises(InterpolationKeyError) as exc_info:
            instantiate(cfg)

        # Verify the error message string includes context
        error_msg = str(exc_info.value)
        assert "full_key: bad_interp" in error_msg
        assert "object_type=MyClass" in error_msg
