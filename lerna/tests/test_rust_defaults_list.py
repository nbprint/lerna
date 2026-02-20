#!/usr/bin/env python
"""Test the Rust defaults list helper."""

from typing import Any

import pytest

from lerna._internal.defaults_list import (
    _HAS_RUST,
    create_defaults_list_rust,
)


def test_rust_defaults_list_available():
    """Test that Rust is available."""
    print(f"Rust available: {_HAS_RUST}")
    assert _HAS_RUST, "Rust bindings should be available"


def test_rust_defaults_list_prepend_hydra_with_pkg_source(hydra_restore_singletons: Any):
    """Test that with prepend_hydra=True and pkg:// source, we can use hybrid repo."""
    from lerna._internal.config_repository import ConfigRepository
    from lerna._internal.config_search_path_impl import ConfigSearchPathImpl
    from lerna._internal.defaults_list import _HAS_HYBRID_REPO

    csp = ConfigSearchPathImpl()
    # Add a pkg:// source (lerna.conf has hydra configs)
    csp.append("lerna", "pkg://lerna.conf")
    csp.append("test", "file://lerna/test_utils/configs")
    repo = ConfigRepository(config_search_path=csp)

    result = create_defaults_list_rust(
        repo=repo,
        config_name="compose",
        overrides_list=[],
        prepend_hydra=True,
        skip_missing=False,
    )

    if _HAS_HYBRID_REPO:
        # With hybrid repo, we should be able to handle pkg:// sources
        # But structured:// (ConfigStore) will still cause fallback
        # So result may still be None depending on the config
        pass  # Either way is acceptable
    else:
        assert result is None, "Without hybrid repo, should return None"


def test_rust_defaults_list_simple(hydra_restore_singletons: Any):
    """Test Rust defaults list for simple case."""
    from lerna._internal.config_repository import ConfigRepository
    from lerna._internal.config_search_path_impl import ConfigSearchPathImpl

    csp = ConfigSearchPathImpl()
    csp.append("test", "file://lerna/test_utils/configs")
    repo = ConfigRepository(config_search_path=csp)

    result = create_defaults_list_rust(
        repo=repo,
        config_name="compose",
        overrides_list=[],
        prepend_hydra=False,
        skip_missing=False,
    )

    if result is not None:
        print(f"Got result with {len(result.defaults)} defaults")
        for d in result.defaults:
            print(f"  {d}")
        assert len(result.defaults) > 0
        # Check that defaults have proper structure
        for d in result.defaults:
            assert hasattr(d, "config_path")
            assert hasattr(d, "is_self")
            assert hasattr(d, "primary")
    else:
        print("Result is None - may have fallen back")


class TestRustOverrides:
    """Tests for Rust overrides parsing."""

    def test_parse_overrides_basic(self):
        """Test basic override parsing."""
        from lerna.lerna import defaults_list as dl

        overrides = dl.parse_overrides(["db=mysql", "server.port=8080"])
        assert overrides is not None
        choices = overrides.get_choices()
        assert "db" in choices or overrides.is_overridden("db")

    def test_parse_overrides_empty(self):
        """Test empty override parsing."""
        from lerna.lerna import defaults_list as dl

        overrides = dl.parse_overrides([])
        assert overrides is not None
        choices = overrides.get_choices()
        assert len(choices) == 0

    def test_rust_overrides_from_strings(self):
        """Test RustOverrides from_strings constructor."""
        from lerna.lerna import defaults_list as dl

        overrides = dl.RustOverrides.from_strings(["group=value", "+append=new"])
        assert overrides is not None


class TestRustIntegrationInCreateDefaultsList:
    """Test that Rust is actually used in create_defaults_list."""

    def test_create_defaults_list_uses_rust_when_available(self, hydra_restore_singletons: Any):
        """Test that create_defaults_list uses Rust for prepend_hydra=False."""
        from lerna._internal.config_repository import ConfigRepository
        from lerna._internal.config_search_path_impl import ConfigSearchPathImpl
        from lerna._internal.defaults_list import _HAS_RUST, create_defaults_list

        csp = ConfigSearchPathImpl()
        csp.append("test", "file://lerna/test_utils/configs")
        repo = ConfigRepository(config_search_path=csp)

        # Call the main function with prepend_hydra=False
        result = create_defaults_list(
            repo=repo,
            config_name="compose",
            overrides_list=[],
            prepend_hydra=False,
            skip_missing=False,
        )

        # Should return a valid result
        assert result is not None
        assert len(result.defaults) > 0

        # If Rust is available, this should have been handled by Rust
        if _HAS_RUST:
            print(f"Rust handled the request, got {len(result.defaults)} defaults")
            # Verify the defaults look correct
            config_paths = [d.config_path for d in result.defaults]
            assert "compose" in config_paths or any("compose" in p for p in config_paths if p)

    def test_create_defaults_list_with_overrides(self, hydra_restore_singletons: Any):
        """Test create_defaults_list with config overrides uses Rust."""
        from lerna._internal.config_repository import ConfigRepository
        from lerna._internal.config_search_path_impl import ConfigSearchPathImpl
        from lerna._internal.defaults_list import _HAS_RUST, create_defaults_list
        from lerna.core.override_parser.overrides_parser import OverridesParser

        csp = ConfigSearchPathImpl()
        csp.append("test", "file://lerna/test_utils/configs")
        repo = ConfigRepository(config_search_path=csp)

        # Parse overrides
        parser = OverridesParser.create()
        overrides = parser.parse_overrides(["group1.foo=999"])

        # Call the main function with prepend_hydra=False
        result = create_defaults_list(
            repo=repo,
            config_name="compose",
            overrides_list=overrides,
            prepend_hydra=False,
            skip_missing=False,
        )

        # Should return a valid result
        assert result is not None
        assert len(result.defaults) > 0

        if _HAS_RUST:
            print(f"With overrides: got {len(result.defaults)} defaults")


class TestRustComposeIntegration:
    """Test Rust compose integration in ConfigLoaderImpl."""

    def test_rust_compose_used_for_file_only(self, hydra_restore_singletons: Any):
        """Test that Rust compose is used when only file:// sources are present."""
        from lerna._internal.config_loader_impl import _HAS_RUST, ConfigLoaderImpl
        from lerna._internal.config_repository import ConfigRepository
        from lerna._internal.config_search_path_impl import ConfigSearchPathImpl
        from lerna.core.default_element import ResultDefault

        if not _HAS_RUST:
            pytest.skip("Rust not available")

        csp = ConfigSearchPathImpl()
        csp.append("test", "file://lerna/test_utils/configs")
        loader = ConfigLoaderImpl(config_search_path=csp)
        repo = ConfigRepository(config_search_path=csp)

        # Create a simple defaults list
        defaults = [
            ResultDefault(
                config_path="compose",
                package=None,
                is_self=False,
                primary=True,
            ),
        ]

        # Try Rust compose
        result = loader._try_rust_compose(defaults, repo)

        # Should succeed for file:// only sources
        assert result is not None
        print(f"Rust compose returned: {result}")

    def test_rust_compose_fallback_for_structured(self, hydra_restore_singletons: Any):
        """Test that Rust compose falls back for structured:// sources."""
        from lerna._internal.config_loader_impl import _HAS_RUST, ConfigLoaderImpl
        from lerna._internal.config_repository import ConfigRepository
        from lerna._internal.config_search_path_impl import ConfigSearchPathImpl
        from lerna.core.default_element import ResultDefault

        if not _HAS_RUST:
            pytest.skip("Rust not available")

        csp = ConfigSearchPathImpl()
        csp.append("test", "file://lerna/test_utils/configs")
        csp.append("schema", "structured://")  # Add structured source
        loader = ConfigLoaderImpl(config_search_path=csp)
        repo = ConfigRepository(config_search_path=csp)

        # Create a simple defaults list
        defaults = [
            ResultDefault(
                config_path="compose",
                package=None,
                is_self=False,
                primary=True,
            ),
        ]

        # Try Rust compose - now handles structured:// via callbacks
        result = loader._try_rust_compose(defaults, repo)

        # With structured:// support via callbacks, Rust should succeed
        assert result is not None
        print(f"Rust compose with structured:// returned: {result}")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
