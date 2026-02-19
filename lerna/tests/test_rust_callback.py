"""Tests for Rust callback extension point bindings."""

from lerna import CallbackManager, JobReturn


class TestCallbackManager:
    """Test CallbackManager functionality."""

    def test_create_empty_manager(self):
        """Test creating an empty callback manager."""
        cm = CallbackManager()
        assert cm.is_empty()
        assert cm.len() == 0

    def test_add_logging_callback(self):
        """Test adding built-in logging callback."""
        cm = CallbackManager()
        cm.add_logging_callback()
        assert not cm.is_empty()
        assert cm.len() == 1

    def test_add_noop_callback(self):
        """Test adding built-in no-op callback."""
        cm = CallbackManager()
        cm.add_noop_callback()
        assert cm.len() == 1

    def test_clear_callbacks(self):
        """Test clearing callbacks."""
        cm = CallbackManager()
        cm.add_logging_callback()
        cm.add_noop_callback()
        assert cm.len() == 2
        cm.clear()
        assert cm.len() == 0

    def test_add_python_callback(self):
        """Test adding a Python callback."""

        class MyCallback:
            def __init__(self):
                self.calls = []

            def on_run_start(self, config, kwargs):
                self.calls.append(("on_run_start", config))

        callback = MyCallback()
        cm = CallbackManager()
        cm.add_callback(callback)
        assert cm.len() == 1

        # Trigger callback
        cm.on_run_start({"key": "value"})
        assert len(callback.calls) == 1
        assert callback.calls[0][0] == "on_run_start"
        assert "key" in callback.calls[0][1]

    def test_multiple_callbacks(self):
        """Test multiple callbacks are all called."""
        calls = []

        class Callback1:
            def on_run_start(self, config, kwargs):
                calls.append(1)

        class Callback2:
            def on_run_start(self, config, kwargs):
                calls.append(2)

        cm = CallbackManager()
        cm.add_callback(Callback1())
        cm.add_callback(Callback2())

        cm.on_run_start({})
        assert calls == [1, 2]

    def test_lifecycle_methods(self):
        """Test all lifecycle methods are callable."""
        cm = CallbackManager()
        cm.add_noop_callback()

        # These should not raise
        cm.on_run_start({})
        cm.on_run_end({})
        cm.on_job_start({})


class TestJobReturn:
    """Test JobReturn functionality."""

    def test_create_job_return(self):
        """Test creating a JobReturn."""
        jr = JobReturn("my_job", "my_task", "/work", "/output", 0, None)
        assert jr.job_name == "my_job"
        assert jr.task_name == "my_task"
        assert jr.working_dir == "/work"
        assert jr.output_dir == "/output"
        assert jr.status_code == 0
        assert jr.return_value is None

    def test_job_return_with_return_value(self):
        """Test JobReturn with return value."""
        jr = JobReturn("job", "task", "/w", "/o", 0, "result")
        assert jr.return_value == "result"

    def test_is_success(self):
        """Test is_success method."""
        jr_success = JobReturn("job", "task", "/w", "/o", 0)
        jr_failure = JobReturn("job", "task", "/w", "/o", 1)

        assert jr_success.is_success()
        assert not jr_failure.is_success()

    def test_setters(self):
        """Test that fields can be modified."""
        jr = JobReturn("job", "task", "/w", "/o", 0)
        jr.status_code = 42
        assert jr.status_code == 42


class TestCallbackWithJobEnd:
    """Test on_job_end callback with JobReturn."""

    def test_on_job_end(self):
        """Test on_job_end receives JobReturn."""
        received = []

        class MyCallback:
            def on_job_end(self, config, job_return, kwargs):
                received.append(job_return)

        cm = CallbackManager()
        cm.add_callback(MyCallback())

        jr = JobReturn("job", "task", "/w", "/o", 0)
        cm.on_job_end({}, jr)

        assert len(received) == 1
