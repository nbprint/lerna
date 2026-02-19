# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""Tests for Sweeper Rust bindings."""

import pytest

from lerna import JobReturn, RustBasicSweeper, SweeperManager


class TestRustBasicSweeper:
    """Test RustBasicSweeper - Rust BasicSweeper exposed to Python."""

    def test_create_basic_sweeper(self):
        """BasicSweeper can be created."""
        sweeper = RustBasicSweeper()
        assert sweeper is not None
        assert sweeper.name() == "basic"

    def test_create_sweeper_with_batch_size(self):
        """BasicSweeper can be created with max_batch_size."""
        sweeper = RustBasicSweeper(max_batch_size=10)
        assert sweeper is not None

    def test_basic_sweeper_setup(self):
        """BasicSweeper can be setup with config."""
        sweeper = RustBasicSweeper()
        config = {"key": "value", "number": 42}
        sweeper.setup(config, "my_task")
        # Should not raise
        assert True

    def test_basic_sweeper_single_value(self):
        """BasicSweeper sweeps single value."""
        sweeper = RustBasicSweeper()
        sweeper.setup({}, "test_task")

        results = sweeper.sweep(["key=value"])

        assert len(results) == 1
        assert isinstance(results[0], JobReturn)

    def test_basic_sweeper_comma_separated(self):
        """BasicSweeper expands comma-separated values."""
        sweeper = RustBasicSweeper()
        sweeper.setup({}, "test_task")

        # Single parameter with 3 values
        results = sweeper.sweep(["db=mysql,postgres,sqlite"])

        assert len(results) == 3

    def test_basic_sweeper_cartesian_product(self):
        """BasicSweeper generates cartesian product."""
        sweeper = RustBasicSweeper()
        sweeper.setup({}, "test_task")

        # 2 params x 2 values each = 4 combinations
        results = sweeper.sweep(["a=1,2", "b=x,y"])

        assert len(results) == 4

    def test_basic_sweeper_three_params(self):
        """BasicSweeper handles multiple parameters."""
        sweeper = RustBasicSweeper()
        sweeper.setup({}, "test_task")

        # 2 x 2 x 2 = 8 combinations
        results = sweeper.sweep(["a=1,2", "b=x,y", "c=true,false"])

        assert len(results) == 8

    def test_basic_sweeper_mixed_params(self):
        """BasicSweeper handles mix of single and multiple values."""
        sweeper = RustBasicSweeper()
        sweeper.setup({}, "test_task")

        # 1 x 2 x 1 = 2 combinations
        results = sweeper.sweep(["fixed=value", "sweep=a,b", "another=100"])

        assert len(results) == 2

    def test_basic_sweeper_empty_args(self):
        """BasicSweeper handles empty arguments."""
        sweeper = RustBasicSweeper()
        sweeper.setup({}, "test_task")

        results = sweeper.sweep([])

        # Empty sweep should still produce one job
        assert len(results) == 1


class TestSweeperManager:
    """Test SweeperManager - manages sweeper instances."""

    def test_create_manager(self):
        """SweeperManager can be created."""
        manager = SweeperManager()
        assert manager is not None
        assert not manager.has_sweeper()

    def test_set_basic_sweeper(self):
        """Manager can set BasicSweeper."""
        manager = SweeperManager()
        manager.set_basic_sweeper()

        assert manager.has_sweeper()
        assert manager.sweeper_name() == "basic"

    def test_set_basic_sweeper_with_batch_size(self):
        """Manager can set BasicSweeper with batch size."""
        manager = SweeperManager()
        manager.set_basic_sweeper(max_batch_size=5)

        assert manager.has_sweeper()

    def test_manager_sweep_without_sweeper(self):
        """Manager raises error when sweeping without sweeper."""
        manager = SweeperManager()

        with pytest.raises(RuntimeError, match="No sweeper configured"):
            manager.sweep([])

    def test_set_python_sweeper(self):
        """Manager can set a Python sweeper."""

        class PySweeper:
            def sweep(self, arguments):
                # Return mock JobReturn objects
                return [
                    JobReturn(
                        job_name="py_job_0",
                        task_name="py_task",
                        working_dir="/tmp",
                        output_dir="/outputs/0",
                        status_code=0,
                    )
                ]

        manager = SweeperManager()
        manager.set_python_sweeper(PySweeper())

        assert manager.has_sweeper()


class TestPythonSweeperIntegration:
    """Test Python sweeper integration with Rust manager."""

    def test_python_sweeper_sweep(self):
        """Python sweeper's sweep method is called correctly."""

        received_args = []

        class TracingSweeper:
            def sweep(self, arguments):
                received_args.extend(arguments)
                return [
                    JobReturn(
                        job_name=f"traced_{i}",
                        task_name="trace_task",
                        working_dir="/",
                        output_dir="/out",
                    )
                    for i in range(len(arguments) if arguments else 1)
                ]

        manager = SweeperManager()
        manager.set_python_sweeper(TracingSweeper())

        _ = manager.sweep(["db=mysql", "port=8080"])

        assert len(received_args) == 2
        assert "db=mysql" in received_args
        assert "port=8080" in received_args


class TestSweeperJobResults:
    """Test job results from sweeper."""

    def test_sweeper_job_fields(self):
        """Sweeper results have expected JobReturn fields."""
        sweeper = RustBasicSweeper()
        sweeper.setup({}, "field_test")

        results = sweeper.sweep(["key=value"])
        job_return = results[0]

        assert hasattr(job_return, "job_name")
        assert hasattr(job_return, "task_name")
        assert hasattr(job_return, "working_dir")
        assert hasattr(job_return, "output_dir")
        assert hasattr(job_return, "status_code")
        assert hasattr(job_return, "return_value")

    def test_sweeper_job_names_unique(self):
        """Sweeper generates unique job names."""
        sweeper = RustBasicSweeper()
        sweeper.setup({}, "naming_test")

        results = sweeper.sweep(["a=1,2,3"])

        job_names = [r.job_name for r in results]
        assert len(set(job_names)) == 3  # All unique

    def test_sweeper_all_jobs_success(self):
        """Sweeper jobs report success status."""
        sweeper = RustBasicSweeper()
        sweeper.setup({}, "status_test")

        results = sweeper.sweep(["key=a,b"])

        assert all(r.is_success() for r in results)
        assert all(r.status_code == 0 for r in results)
