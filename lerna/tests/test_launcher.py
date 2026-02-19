# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""Tests for Launcher Rust bindings."""

import pytest

from lerna import JobReturn, LauncherManager, RustBasicLauncher


class TestRustBasicLauncher:
    """Test RustBasicLauncher - Rust BasicLauncher exposed to Python."""

    def test_create_basic_launcher(self):
        """BasicLauncher can be created."""
        launcher = RustBasicLauncher()
        assert launcher is not None
        assert launcher.name() == "basic"

    def test_basic_launcher_setup(self):
        """BasicLauncher can be setup with config."""
        launcher = RustBasicLauncher()
        config = {"key": "value", "number": 42}
        launcher.setup(config, "my_task")
        # Should not raise
        assert True

    def test_basic_launcher_launch_single_job(self):
        """BasicLauncher can launch a single job."""
        launcher = RustBasicLauncher()
        launcher.setup({}, "test_task")

        # Launch one job with overrides
        job_overrides = [["db=mysql", "server.port=8080"]]
        results = launcher.launch(job_overrides, 0)

        assert len(results) == 1
        assert isinstance(results[0], JobReturn)
        assert results[0].job_name == "job_0"
        assert results[0].is_success()

    def test_basic_launcher_launch_multiple_jobs(self):
        """BasicLauncher can launch multiple jobs."""
        launcher = RustBasicLauncher()
        launcher.setup({}, "multi_task")

        # Launch multiple jobs
        job_overrides = [
            ["db=mysql"],
            ["db=postgres"],
            ["db=sqlite"],
        ]
        results = launcher.launch(job_overrides, 0)

        assert len(results) == 3
        assert results[0].job_name == "job_0"
        assert results[1].job_name == "job_1"
        assert results[2].job_name == "job_2"

    def test_basic_launcher_initial_job_idx(self):
        """BasicLauncher respects initial_job_idx."""
        launcher = RustBasicLauncher()
        launcher.setup({}, "indexed_task")

        job_overrides = [["key=value"]]
        results = launcher.launch(job_overrides, 10)

        assert results[0].job_name == "job_10"


class TestLauncherManager:
    """Test LauncherManager - manages launcher instances."""

    def test_create_manager(self):
        """LauncherManager can be created."""
        manager = LauncherManager()
        assert manager is not None
        assert not manager.has_launcher()

    def test_set_basic_launcher(self):
        """Manager can set BasicLauncher."""
        manager = LauncherManager()
        manager.set_basic_launcher()

        assert manager.has_launcher()
        assert manager.launcher_name() == "basic"

    def test_manager_launch_without_launcher(self):
        """Manager raises error when launching without launcher."""
        manager = LauncherManager()

        with pytest.raises(RuntimeError, match="No launcher configured"):
            manager.launch([[]], 0)

    def test_manager_launch_with_basic_launcher(self):
        """Manager can launch jobs with BasicLauncher."""
        manager = LauncherManager()
        manager.set_basic_launcher()

        job_overrides = [["db=mysql"], ["db=postgres"]]
        results = manager.launch(job_overrides, 0)

        assert len(results) == 2

    def test_set_python_launcher(self):
        """Manager can set a Python launcher."""

        class PyLauncher:
            def launch(self, job_overrides, initial_job_idx):
                # Return mock JobReturn objects
                results = []
                for idx, overrides in enumerate(job_overrides):
                    job_return = JobReturn(
                        job_name=f"py_job_{initial_job_idx + idx}",
                        task_name="py_task",
                        working_dir="/tmp",
                        output_dir=f"/outputs/{initial_job_idx + idx}",
                        status_code=0,
                    )
                    results.append(job_return)
                return results

        manager = LauncherManager()
        manager.set_python_launcher(PyLauncher())

        assert manager.has_launcher()


class TestPythonLauncherIntegration:
    """Test Python launcher integration with Rust manager."""

    def test_python_launcher_launch(self):
        """Python launcher's launch method is called correctly."""

        launched_jobs = []

        class TracingLauncher:
            def launch(self, job_overrides, initial_job_idx):
                launched_jobs.extend(job_overrides)
                results = []
                for idx, overrides in enumerate(job_overrides):
                    # Create a simple JobReturn-like object
                    results.append(
                        JobReturn(
                            job_name=f"traced_{idx}",
                            task_name="trace_task",
                            working_dir="/",
                            output_dir="/out",
                        )
                    )
                return results

        manager = LauncherManager()
        manager.set_python_launcher(TracingLauncher())

        job_overrides = [["a=1"], ["a=2", "b=3"]]
        results = manager.launch(job_overrides, 0)

        assert len(results) == 2
        assert len(launched_jobs) == 2
        assert launched_jobs[0] == ["a=1"]
        assert launched_jobs[1] == ["a=2", "b=3"]


class TestJobReturnFromLauncher:
    """Test JobReturn objects returned from launchers."""

    def test_job_return_fields(self):
        """JobReturn has expected fields."""
        launcher = RustBasicLauncher()
        launcher.setup({}, "field_test")

        results = launcher.launch([["key=value"]], 0)
        job_return = results[0]

        assert hasattr(job_return, "job_name")
        assert hasattr(job_return, "task_name")
        assert hasattr(job_return, "working_dir")
        assert hasattr(job_return, "output_dir")
        assert hasattr(job_return, "status_code")
        assert hasattr(job_return, "return_value")

    def test_job_return_success_status(self):
        """JobReturn.is_success() works correctly."""
        launcher = RustBasicLauncher()
        launcher.setup({}, "status_test")

        results = launcher.launch([["key=value"]], 0)

        assert results[0].is_success() is True
        assert results[0].status_code == 0
