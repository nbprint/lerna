# Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
"""Test Rust integration for lerna"""

import pytest


class TestRustSweepIntegration:
    """Test Rust sweep expansion integration"""

    def test_import_rust_module(self):
        """Test that the Rust module can be imported"""
        import lerna.lerna as rs

        assert hasattr(rs, "expand_sweeps")
        assert hasattr(rs, "count_sweep_combinations")

    def test_simple_choice_sweep(self):
        """Test simple choice sweep expansion"""
        import lerna.lerna as rs

        combos = rs.expand_sweeps(["db=mysql,postgres"])
        assert len(combos) == 2
        assert ["db=mysql"] in combos
        assert ["db=postgres"] in combos

    def test_multi_dimension_sweep(self):
        """Test multi-dimensional sweep expansion"""
        import lerna.lerna as rs

        combos = rs.expand_sweeps(["db=mysql,postgres", "port=3306,5432"])
        assert len(combos) == 4
        assert ["db=mysql", "port=3306"] in combos
        assert ["db=mysql", "port=5432"] in combos
        assert ["db=postgres", "port=3306"] in combos
        assert ["db=postgres", "port=5432"] in combos

    def test_range_sweep(self):
        """Test range sweep expansion"""
        import lerna.lerna as rs

        combos = rs.expand_sweeps(["x=range(1,4)"])
        assert len(combos) == 3
        assert ["x=1"] in combos
        assert ["x=2"] in combos
        assert ["x=3"] in combos

    def test_range_with_step_sweep(self):
        """Test range with step sweep expansion"""
        import lerna.lerna as rs

        combos = rs.expand_sweeps(["x=range(0,10,2)"])
        assert len(combos) == 5
        for i, val in enumerate([0, 2, 4, 6, 8]):
            assert [f"x={val}"] in combos

    def test_mixed_sweep_and_static(self):
        """Test mixed sweep and static overrides"""
        import lerna.lerna as rs

        combos = rs.expand_sweeps(["db=mysql,postgres", "port=3306"])
        assert len(combos) == 2
        assert ["db=mysql", "port=3306"] in combos
        assert ["db=postgres", "port=3306"] in combos

    def test_count_combinations(self):
        """Test combination counting"""
        import lerna.lerna as rs

        assert rs.count_sweep_combinations(["a=1,2,3"]) == 3
        assert rs.count_sweep_combinations(["a=1,2", "b=1,2"]) == 4
        assert rs.count_sweep_combinations(["x=range(1,10)"]) == 9

    def test_empty_overrides(self):
        """Test empty override list"""
        import lerna.lerna as rs

        combos = rs.expand_sweeps([])
        assert len(combos) == 1
        assert combos[0] == []


class TestRustDefaultsListIntegration:
    """Test Rust defaults list integration"""

    def test_parse_simple_overrides(self):
        """Test parsing simple overrides"""
        import lerna.lerna as rs

        ovr = rs.defaults_list.parse_overrides(["db=postgres", "server=nginx"])
        choices = ovr.get_choices()
        assert "db" in choices
        assert choices["db"] == "postgres"
        assert "server" in choices
        assert choices["server"] == "nginx"

    def test_parse_deletion(self):
        """Test parsing deletion override"""
        import lerna.lerna as rs

        ovr = rs.defaults_list.parse_overrides(["~db"])
        deletions = ovr.get_deletions()
        assert "db" in deletions

    def test_parse_append(self):
        """Test parsing append override"""
        import lerna.lerna as rs

        ovr = rs.defaults_list.parse_overrides(["+cache=redis"])
        appends = ovr.get_appends()
        assert len(appends) == 1
        assert appends[0] == ("cache", "redis")

    def test_mixed_overrides(self):
        """Test parsing mixed override types"""
        import lerna.lerna as rs

        ovr = rs.defaults_list.parse_overrides(
            [
                "db=postgres",
                "~server",
                "+cache=redis",
            ]
        )
        assert ovr.get_choices()["db"] == "postgres"
        assert "server" in ovr.get_deletions()
        assert ("cache", "redis") in ovr.get_appends()

    def test_overrides_methods(self):
        """Test Overrides helper methods"""
        import lerna.lerna as rs

        ovr = rs.defaults_list.parse_overrides(["db=postgres", "~server"])

        assert ovr.is_overridden("db")
        assert not ovr.is_overridden("unknown")
        assert ovr.is_deleted("server")
        assert not ovr.is_deleted("db")
        assert ovr.get_choice("db") == "postgres"
        assert ovr.get_choice("unknown") is None


class TestRustConfigIntegration:
    """Test Rust config loading integration"""

    def test_parse_yaml(self):
        """Test YAML parsing"""
        import lerna.lerna as rs

        result = rs.parse_yaml("key: value\nnum: 42")
        assert result["key"] == "value"
        assert result["num"] == 42

    def test_parse_nested_yaml(self):
        """Test nested YAML parsing"""
        import lerna.lerna as rs

        result = rs.parse_yaml("""
db:
  host: localhost
  port: 3306
""")
        assert result["db"]["host"] == "localhost"
        assert result["db"]["port"] == 3306

    def test_parse_list_yaml(self):
        """Test list YAML parsing"""
        import lerna.lerna as rs

        result = rs.parse_yaml("items:\n  - a\n  - b\n  - c")
        assert result["items"] == ["a", "b", "c"]


class TestRustGlobIntegration:
    """Test Rust glob pattern integration"""

    def test_glob_match(self):
        """Test glob pattern matching"""
        import lerna.lerna as rs

        # Glob requires a list of patterns
        g = rs.Glob(["*.yaml"])
        # Glob class exists and is usable
        assert g is not None


class TestRustOverrideParserIntegration:
    """Test Rust override parser integration"""

    def test_parse_simple_override(self):
        """Test parsing simple override"""
        import lerna.lerna as rs

        parser = rs.OverrideParser()
        ovr = parser.parse("db=postgres")
        assert ovr.key_or_group == "db"
        # Value is a quoted string type

    def test_parse_add_override(self):
        """Test parsing add override"""
        import lerna.lerna as rs

        parser = rs.OverrideParser()
        ovr = parser.parse("+db=postgres")
        assert ovr.key_or_group == "db"
        assert ovr.is_add()

    def test_parse_delete_override(self):
        """Test parsing delete override"""
        import lerna.lerna as rs

        parser = rs.OverrideParser()
        ovr = parser.parse("~db")
        assert ovr.key_or_group == "db"
        assert ovr.is_delete()


class TestRustValidationIntegration:
    """Test Rust validation integration"""

    def test_type_spec_parse(self):
        """Test TypeSpec parsing"""
        import lerna.lerna as rs

        ts = rs.validation.TypeSpec.parse("int")
        assert ts is not None

        ts_list = rs.validation.TypeSpec.parse("List[str]")
        assert ts_list is not None

        ts_opt = rs.validation.TypeSpec.parse("Optional[int]")
        assert ts_opt is not None

    def test_type_spec_constructors(self):
        """Test TypeSpec factory methods"""
        import lerna.lerna as rs

        assert rs.validation.TypeSpec.int() is not None
        assert rs.validation.TypeSpec.string() is not None
        assert rs.validation.TypeSpec.bool() is not None
        assert rs.validation.TypeSpec.float() is not None
        assert rs.validation.TypeSpec.any() is not None

    def test_type_spec_composite(self):
        """Test composite TypeSpec creation"""
        import lerna.lerna as rs

        int_type = rs.validation.TypeSpec.int()
        list_int = rs.validation.TypeSpec.list(int_type)
        assert list_int is not None

        opt_int = rs.validation.TypeSpec.optional(int_type)
        assert opt_int is not None

    def test_validate_type(self):
        """Test type validation"""
        import lerna.lerna as rs

        int_type = rs.validation.TypeSpec.int()

        assert rs.validation.validate_type(42, int_type)
        assert not rs.validation.validate_type("hello", int_type)

        str_type = rs.validation.TypeSpec.string()
        assert rs.validation.validate_type("hello", str_type)
        assert not rs.validation.validate_type(42, str_type)

    def test_config_schema_basic(self):
        """Test ConfigSchema basic usage"""
        import lerna.lerna as rs

        schema = rs.validation.ConfigSchema()
        schema.required("name", rs.validation.TypeSpec.string())
        schema.required("port", rs.validation.TypeSpec.int())

        # Valid config
        errors = schema.validate({"name": "test", "port": 8080})
        assert len(errors) == 0
        assert schema.is_valid({"name": "test", "port": 8080})

    def test_config_schema_missing_field(self):
        """Test ConfigSchema with missing required field"""
        import lerna.lerna as rs

        schema = rs.validation.ConfigSchema()
        schema.required("name", rs.validation.TypeSpec.string())
        schema.required("port", rs.validation.TypeSpec.int())

        # Missing port
        errors = schema.validate({"name": "test"})
        assert len(errors) == 1
        assert errors[0].path == "port"
        assert not schema.is_valid({"name": "test"})

    def test_config_schema_type_mismatch(self):
        """Test ConfigSchema with type mismatch"""
        import lerna.lerna as rs

        schema = rs.validation.ConfigSchema()
        schema.required("port", rs.validation.TypeSpec.int())

        # port should be int, not string
        errors = schema.validate({"port": "not-a-number"})
        assert len(errors) == 1
        assert "Type mismatch" in errors[0].message

    def test_config_schema_optional(self):
        """Test ConfigSchema with optional fields"""
        import lerna.lerna as rs

        schema = rs.validation.ConfigSchema()
        schema.required("name", rs.validation.TypeSpec.string())
        schema.optional("host", rs.validation.TypeSpec.string(), "localhost")

        # Valid without optional field
        assert schema.is_valid({"name": "test"})


class TestRustJobIntegration:
    """Test Rust job configuration integration"""

    def test_job_config_basic(self):
        """Test basic JobConfig creation"""
        import lerna.lerna as rs

        job = rs.job.JobConfig("myapp", 0, ["db=mysql"])
        assert job.name == "myapp"
        assert job.idx == 0
        assert job.overrides == ["db=mysql"]

    def test_job_config_override_dirname(self):
        """Test JobConfig override dirname generation"""
        import lerna.lerna as rs

        job = rs.job.JobConfig("myapp", 0, ["db=mysql", "port=3306"])
        dirname = job.get_override_dirname("_", ",")
        assert "db_mysql" in dirname
        assert "port_3306" in dirname

    def test_job_config_exclude_keys(self):
        """Test JobConfig override dirname with excluded keys"""
        import lerna.lerna as rs

        job = rs.job.JobConfig("myapp", 0, ["db=mysql", "port=3306"])
        dirname = job.get_override_dirname("_", ",", ["port"])
        assert "db_mysql" in dirname
        assert "port" not in dirname

    def test_generate_sweep_jobs(self):
        """Test generating sweep jobs"""
        import lerna.lerna as rs

        sweep_overrides = [["db=mysql"], ["db=postgres"]]
        jobs = rs.job.generate_jobs("myapp", sweep_overrides, "/output")

        assert len(jobs) == 2
        assert jobs[0].idx == 0
        assert jobs[1].idx == 1
        assert jobs[0].num_jobs == 2
        assert jobs[1].num_jobs == 2

    def test_compute_output_dir(self):
        """Test computing output directory"""
        import lerna.lerna as rs

        # Without override dirname
        dir1 = rs.job.compute_job_output_dir("/output", 0, [], False)
        # Normalize separators for cross-platform comparison
        assert dir1.replace("\\", "/") == "/output/0"

        # With override dirname
        dir2 = rs.job.compute_job_output_dir("/output", 0, ["db=mysql"], True)
        assert "db_mysql" in dir2


class TestRustInterpolationIntegration:
    """Test Rust interpolation resolution integration"""

    def test_has_interpolations(self):
        """Test checking if string has interpolations"""
        import lerna.lerna as rs

        assert rs.interpolation.has_interpolations("${key}")
        assert rs.interpolation.has_interpolations("Hello ${name}")
        assert not rs.interpolation.has_interpolations("no interpolation")
        assert not rs.interpolation.has_interpolations("just text")

    def test_find_interpolations(self):
        """Test finding interpolations in a string"""
        import lerna.lerna as rs

        interps = rs.interpolation.find_interpolations_in_string("host=${db.host}, port=${db.port}")
        assert len(interps) == 2
        assert interps[0][2] == "${db.host}"
        assert interps[1][2] == "${db.port}"

    def test_get_interpolation_type_simple(self):
        """Test getting interpolation type for simple key"""
        import lerna.lerna as rs

        result = rs.interpolation.get_interpolation_type("${key}")
        assert result == "key:key"

    def test_get_interpolation_type_nested(self):
        """Test getting interpolation type for nested key"""
        import lerna.lerna as rs

        result = rs.interpolation.get_interpolation_type("${db.host}")
        assert result == "nested:db.host"

    def test_get_interpolation_type_env(self):
        """Test getting interpolation type for env var"""
        import lerna.lerna as rs

        result = rs.interpolation.get_interpolation_type("${oc.env:HOME}")
        assert result == "env:HOME"

        result_default = rs.interpolation.get_interpolation_type("${oc.env:MISSING,fallback}")
        assert result_default == "env:MISSING,fallback"

    def test_resolve_string_simple(self):
        """Test resolving simple interpolations in string"""
        import lerna.lerna as rs

        config = {"host": "localhost", "port": 3306}
        result = rs.interpolation.resolve_string_interpolations("mysql://${host}:${port}", config)
        assert result == "mysql://localhost:3306"

    def test_resolve_string_nested(self):
        """Test resolving nested interpolations"""
        import lerna.lerna as rs

        config = {"db": {"host": "localhost", "port": 3306}}
        result = rs.interpolation.resolve_string_interpolations("mysql://${db.host}:${db.port}", config)
        assert result == "mysql://localhost:3306"

    def test_resolve_config(self):
        """Test resolving all interpolations in a config"""
        import lerna.lerna as rs

        config = {"db": {"host": "localhost", "port": 3306}, "url": "mysql://${db.host}:${db.port}"}
        resolved = rs.interpolation.resolve_config_interpolations(config)
        assert resolved["url"] == "mysql://localhost:3306"


class TestRustPackageIntegration:
    """Test Rust package resolution integration"""

    def test_package_resolver_basic(self):
        """Test basic PackageResolver"""
        import lerna.lerna as rs

        resolver = rs.package.PackageResolver()
        resolver = resolver.with_config_group("db/mysql")
        assert resolver.resolve() == "db/mysql"

    def test_package_resolver_override(self):
        """Test PackageResolver with package override"""
        import lerna.lerna as rs

        resolver = rs.package.PackageResolver()
        resolver = resolver.with_config_group("db/mysql")
        resolver = resolver.with_package_override("database")
        assert resolver.resolve() == "database"

    def test_package_resolver_global(self):
        """Test PackageResolver with _global_ package"""
        import lerna.lerna as rs

        resolver = rs.package.PackageResolver()
        resolver = resolver.with_config_group("db/mysql")
        resolver = resolver.with_package_override("_global_")
        assert resolver.resolve() == ""

    def test_package_resolver_group(self):
        """Test PackageResolver with _group_ package"""
        import lerna.lerna as rs

        resolver = rs.package.PackageResolver()
        resolver = resolver.with_config_group("db/mysql")
        resolver = resolver.with_package_override("_group_")
        assert resolver.resolve() == "db/mysql"

    def test_package_resolver_name(self):
        """Test PackageResolver with _name_ package"""
        import lerna.lerna as rs

        resolver = rs.package.PackageResolver()
        resolver = resolver.with_config_group("db/mysql")
        resolver = resolver.with_package_override("_name_")
        assert resolver.resolve() == "mysql"

    def test_parse_package_header(self):
        """Test parsing @package directive from header"""
        import lerna.lerna as rs

        content = "# @package _global_\ndb:\n  host: localhost"
        result = rs.package.parse_package_from_header(content)
        assert result == "_global_"

    def test_parse_package_header_none(self):
        """Test parsing config with no @package directive"""
        import lerna.lerna as rs

        content = "db:\n  host: localhost"
        result = rs.package.parse_package_from_header(content)
        assert result is None

    def test_compute_target_path(self):
        """Test computing target path"""
        import lerna.lerna as rs

        assert rs.package.compute_config_target_path("db", "host") == "db.host"
        assert rs.package.compute_config_target_path("", "host") == "host"
        assert rs.package.compute_config_target_path("db", "") == "db"

    def test_split_and_join_path(self):
        """Test splitting and joining dotted paths"""
        import lerna.lerna as rs

        parts = rs.package.split_dotted_path("db.host.port")
        assert parts == ["db", "host", "port"]

        joined = rs.package.join_dotted_path(["db", "host", "port"])
        assert joined == "db.host.port"


class TestRustMergeIntegration:
    """Test Rust config merge integration"""

    def test_merge_simple_dicts(self):
        """Test merging two simple dictionaries"""
        import lerna.lerna as rs

        base = {"a": 1, "b": 2}
        other = {"b": 20, "c": 3}
        result = rs.merge.merge_config_dicts(base, other)

        assert result["a"] == 1
        assert result["b"] == 20
        assert result["c"] == 3

    def test_merge_nested_dicts(self):
        """Test merging nested dictionaries"""
        import lerna.lerna as rs

        base = {"db": {"host": "localhost", "port": 3306}}
        other = {"db": {"port": 5432}}
        result = rs.merge.merge_config_dicts(base, other)

        assert result["db"]["host"] == "localhost"
        assert result["db"]["port"] == 5432

    def test_merge_multiple_configs(self):
        """Test merging multiple configs in order"""
        import lerna.lerna as rs

        cfg1 = {"a": 1}
        cfg2 = {"b": 2}
        cfg3 = {"a": 10, "c": 3}

        result = rs.merge.merge_multiple_configs([cfg1, cfg2, cfg3])

        assert result["a"] == 10
        assert result["b"] == 2
        assert result["c"] == 3

    def test_apply_deletions(self):
        """Test applying deletions to a config"""
        import lerna.lerna as rs

        config = {"a": 1, "b": 2, "c": 3}
        result = rs.merge.apply_config_deletions(config, ["~a", "~c"])

        assert "a" not in result
        assert result["b"] == 2
        assert "c" not in result

    def test_apply_override(self):
        """Test applying an override at a nested path"""
        import lerna.lerna as rs

        config = {"a": 1}
        result = rs.merge.apply_config_override(config, "b.c.d", 42)

        assert result["b"]["c"]["d"] == 42

    def test_get_nested_value(self):
        """Test getting a value at a nested path"""
        import lerna.lerna as rs

        config = {"db": {"connection": {"host": "localhost"}}}
        result = rs.merge.get_nested_value(config, "db.connection.host")

        assert result == "localhost"

    def test_get_nested_value_missing(self):
        """Test getting a missing nested value"""
        import lerna.lerna as rs

        config = {"a": 1}
        result = rs.merge.get_nested_value(config, "b.c.d")

        assert result is None

    def test_get_all_keys(self):
        """Test collecting all keys from a config"""
        import lerna.lerna as rs

        config = {"db": {"host": "localhost"}, "port": 3306}
        keys = rs.merge.get_all_keys(config)

        assert "db" in keys
        assert "db.host" in keys
        assert "port" in keys

    def test_get_diff_keys(self):
        """Test finding differing keys between configs"""
        import lerna.lerna as rs

        config1 = {"a": 1, "b": 2}
        config2 = {"a": 1, "c": 3}

        diff = rs.merge.get_diff_keys(config1, config2)

        assert "b" in diff
        assert "c" in diff


class TestRustSearchPathIntegration:
    """Test Rust search path integration"""

    def test_import_search_path_types(self):
        """Test that search path types can be imported"""
        import lerna.lerna as rs

        assert hasattr(rs, "SearchPathElement")
        assert hasattr(rs, "SearchPathQuery")
        assert hasattr(rs, "RustConfigSearchPath")

    def test_search_path_element_creation(self):
        """Test creating a search path element"""
        import lerna.lerna as rs

        elem = rs.SearchPathElement("hydra", "file://conf")

        assert elem.provider == "hydra"
        assert elem.path == "file://conf"
        assert elem.scheme() == "file"
        assert elem.path_without_scheme() == "conf"

    def test_search_path_element_no_scheme(self):
        """Test element without scheme"""
        import lerna.lerna as rs

        elem = rs.SearchPathElement("main", "conf/db")

        assert elem.scheme() is None
        assert elem.path_without_scheme() == "conf/db"

    def test_search_path_query_by_provider(self):
        """Test query matching by provider"""
        import lerna.lerna as rs

        elem = rs.SearchPathElement("hydra", "file://conf")
        query = rs.SearchPathQuery.by_provider("hydra")

        assert query.matches(elem)

        query2 = rs.SearchPathQuery.by_provider("other")
        assert not query2.matches(elem)

    def test_search_path_query_by_path(self):
        """Test query matching by path"""
        import lerna.lerna as rs

        elem = rs.SearchPathElement("hydra", "file://conf")
        query = rs.SearchPathQuery.by_path("file://conf")

        assert query.matches(elem)

    def test_config_search_path_append(self):
        """Test appending to search path"""
        import lerna.lerna as rs

        sp = rs.RustConfigSearchPath()
        sp.append("hydra", "file://conf1")
        sp.append("main", "file://conf2")

        assert len(sp) == 2
        assert sp.get(0).provider == "hydra"
        assert sp.get(1).provider == "main"

    def test_config_search_path_prepend(self):
        """Test prepending to search path"""
        import lerna.lerna as rs

        sp = rs.RustConfigSearchPath()
        sp.append("hydra", "file://conf1")
        sp.prepend("main", "file://conf2")

        assert len(sp) == 2
        assert sp.get(0).provider == "main"
        assert sp.get(1).provider == "hydra"

    def test_config_search_path_find(self):
        """Test finding elements in search path"""
        import lerna.lerna as rs

        sp = rs.RustConfigSearchPath()
        sp.append("hydra", "file://conf1")
        sp.append("main", "file://conf2")
        sp.append("hydra", "file://conf3")

        query = rs.SearchPathQuery.by_provider("hydra")

        assert sp.find_first_match(query) == 0
        assert sp.find_last_match(query) == 2

    def test_config_search_path_remove(self):
        """Test removing elements from search path"""
        import lerna.lerna as rs

        sp = rs.RustConfigSearchPath()
        sp.append("hydra", "file://conf1")
        sp.append("main", "file://conf2")
        sp.append("hydra", "file://conf3")

        query = rs.SearchPathQuery.by_provider("hydra")
        removed = sp.remove(query)

        assert removed == 2
        assert len(sp) == 1
        assert sp.get(0).provider == "main"

    def test_config_search_path_append_after(self):
        """Test appending after an anchor"""
        import lerna.lerna as rs

        sp = rs.RustConfigSearchPath()
        sp.append("hydra", "file://conf1")
        sp.append("main", "file://conf2")

        anchor = rs.SearchPathQuery.by_provider("hydra")
        sp.append_after("plugin", "file://plugin_conf", anchor)

        assert len(sp) == 3
        assert sp.get(0).provider == "hydra"
        assert sp.get(1).provider == "plugin"
        assert sp.get(2).provider == "main"

    def test_config_search_path_from_tuples(self):
        """Test creating search path from tuples"""
        import lerna.lerna as rs

        sp = rs.RustConfigSearchPath.from_tuples(
            [
                ("hydra", "file://conf1"),
                ("main", "file://conf2"),
            ]
        )

        assert len(sp) == 2
        path = sp.get_path()
        assert path[0].provider == "hydra"
        assert path[1].provider == "main"


class TestRustEnvIntegration:
    """Test Rust environment variable resolver integration"""

    def test_import_env_module(self):
        """Test that the env module can be imported"""
        import lerna.lerna as rs

        assert hasattr(rs, "env")
        assert hasattr(rs.env, "EnvResolver")
        assert hasattr(rs.env, "parse_env_reference")
        assert hasattr(rs.env, "resolve_env_string")

    def test_env_resolver_get(self):
        """Test basic env resolver get"""
        import os

        import lerna.lerna as rs

        # Set a test variable
        os.environ["TEST_RUST_ENV"] = "test_value"

        resolver = rs.env.EnvResolver()
        result = resolver.get("TEST_RUST_ENV")
        assert result == "test_value"

        del os.environ["TEST_RUST_ENV"]

    def test_env_resolver_get_or_default(self):
        """Test env resolver with default value"""
        import lerna.lerna as rs

        resolver = rs.env.EnvResolver()
        result = resolver.get_or_default("NONEXISTENT_VAR_12345", "fallback")
        assert result == "fallback"

    def test_env_resolver_get_required(self):
        """Test env resolver get_required raises on missing"""
        import lerna.lerna as rs

        resolver = rs.env.EnvResolver()
        with pytest.raises(Exception):
            resolver.get_required("NONEXISTENT_VAR_12345")

    def test_env_resolver_with_override(self):
        """Test env resolver with overrides"""
        import lerna.lerna as rs

        resolver = rs.env.EnvResolver()
        resolver.set_override("MY_OVERRIDE", "override_value")

        result = resolver.get("MY_OVERRIDE")
        assert result == "override_value"

    def test_env_resolver_caching(self):
        """Test env resolver caching"""
        import os

        import lerna.lerna as rs

        os.environ["CACHE_TEST_VAR"] = "initial"

        resolver = rs.env.EnvResolver()
        resolver.enable_caching(True)

        result1 = resolver.get("CACHE_TEST_VAR")
        assert result1 == "initial"

        os.environ["CACHE_TEST_VAR"] = "changed"
        result2 = resolver.get("CACHE_TEST_VAR")
        # Should still be initial due to caching
        assert result2 == "initial"

        resolver.clear_cache()
        result3 = resolver.get("CACHE_TEST_VAR")
        assert result3 == "changed"

        del os.environ["CACHE_TEST_VAR"]

    def test_env_resolver_resolve_string(self):
        """Test resolving env references in a string"""
        import os

        import lerna.lerna as rs

        os.environ["USER_NAME"] = "alice"
        os.environ["USER_HOST"] = "localhost"

        resolver = rs.env.EnvResolver()
        result = resolver.resolve_string("Hello ${oc.env:USER_NAME} at ${oc.env:USER_HOST}")
        assert result == "Hello alice at localhost"

        del os.environ["USER_NAME"]
        del os.environ["USER_HOST"]

    def test_parse_env_reference(self):
        """Test parsing env references"""
        import lerna.lerna as rs

        result = rs.env.parse_env_reference("${oc.env:HOME}")
        assert result is not None
        assert result[0] == "HOME"
        assert result[1] is None

    def test_parse_env_reference_with_default(self):
        """Test parsing env references with default"""
        import lerna.lerna as rs

        result = rs.env.parse_env_reference("${oc.env:MISSING,fallback}")
        assert result is not None
        assert result[0] == "MISSING"
        assert result[1] == "fallback"

    def test_parse_env_reference_short_form(self):
        """Test parsing short form env references"""
        import lerna.lerna as rs

        result = rs.env.parse_env_reference("${env:PATH}")
        assert result is not None
        assert result[0] == "PATH"

    def test_find_env_references(self):
        """Test finding env references in a string"""
        import lerna.lerna as rs

        refs = rs.env.find_env_references("Start ${oc.env:A} middle ${env:B,default} end")
        assert len(refs) == 2
        # Refs are (start, end, var_name, default_value) tuples
        var_names = [r[2] for r in refs]
        assert "A" in var_names
        assert "B" in var_names

    def test_resolve_env_string_module_function(self):
        """Test resolve_env_string as module function"""
        import os

        import lerna.lerna as rs

        os.environ["MODULE_TEST"] = "module_value"

        result = rs.env.resolve_env_string("Value: ${oc.env:MODULE_TEST}")
        assert result == "Value: module_value"

        del os.environ["MODULE_TEST"]

    def test_get_env_function(self):
        """Test get_env module function"""
        import os

        import lerna.lerna as rs

        os.environ["GET_ENV_TEST"] = "test123"

        result = rs.env.get_env("GET_ENV_TEST")
        assert result == "test123"

        result = rs.env.get_env("NONEXISTENT_12345")
        assert result is None

        del os.environ["GET_ENV_TEST"]

    def test_is_env_set_function(self):
        """Test is_env_set module function"""
        import os

        import lerna.lerna as rs

        os.environ["IS_SET_TEST"] = "yes"

        assert rs.env.is_env_set("IS_SET_TEST") is True
        assert rs.env.is_env_set("DEFINITELY_NOT_SET_12345") is False

        del os.environ["IS_SET_TEST"]

    def test_get_many_env(self):
        """Test getting multiple env vars at once"""
        import os

        import lerna.lerna as rs

        os.environ["MULTI_A"] = "value_a"
        os.environ["MULTI_B"] = "value_b"

        result = rs.env.get_many_env(["MULTI_A", "MULTI_B", "NONEXISTENT"])
        assert result["MULTI_A"] == "value_a"
        assert result["MULTI_B"] == "value_b"
        assert result["NONEXISTENT"] is None

        del os.environ["MULTI_A"]
        del os.environ["MULTI_B"]

    def test_resolve_with_default_when_missing(self):
        """Test that defaults are used when env var is missing"""
        import lerna.lerna as rs

        result = rs.env.resolve_env_string("Value: ${oc.env:MISSING_VAR,default_val}")
        assert result == "Value: default_val"


class TestRustConfigUtilsIntegration:
    """Test Rust config utility functions integration"""

    def test_extract_header_dict_package(self):
        """Test extracting package from header"""
        import lerna.lerna as rs

        content = "# @package db\nhost: localhost\n"
        header = rs.extract_header_dict(content)
        assert header.get("package") == "db"

    def test_extract_header_dict_multiple(self):
        """Test extracting multiple headers"""
        import lerna.lerna as rs

        content = "# @package _global_\n# @mode strict\ndb:\n  host: localhost\n"
        header = rs.extract_header_dict(content)
        assert header.get("package") == "_global_"
        assert header.get("mode") == "strict"

    def test_extract_header_dict_none(self):
        """Test header when no package directive"""
        import lerna.lerna as rs

        content = "# Just a comment\ndb:\n  host: localhost\n"
        header = rs.extract_header_dict(content)
        assert header.get("package") is None

    def test_normalize_file_name(self):
        """Test normalizing config file names"""
        import lerna.lerna as rs

        assert rs.normalize_file_name("config") == "config.yaml"
        assert rs.normalize_file_name("config.yaml") == "config.yaml"
        assert rs.normalize_file_name("config.yml") == "config.yml"

    def test_get_valid_filename(self):
        """Test getting valid filenames"""
        import lerna.lerna as rs

        assert rs.get_valid_filename("my app") == "my_app"
        assert rs.get_valid_filename("file@123") == "file123"
        assert rs.get_valid_filename("test-file.py") == "test-file.py"

    def test_sanitize_path_component(self):
        """Test sanitizing path components"""
        import lerna.lerna as rs

        assert rs.sanitize_path_component("file") == "file"
        assert rs.sanitize_path_component("path/to/file") == "path_to_file"
        assert rs.sanitize_path_component("file:name") == "file_name"


class TestRustPythonIntegration:
    """Test that Python modules correctly use Rust acceleration."""

    def test_config_source_normalize_uses_rust(self):
        """Test that ConfigSource._normalize_file_name uses Rust"""
        from lerna.plugins.config_source import ConfigSource

        # Test normalization
        assert ConfigSource._normalize_file_name("config") == "config.yaml"
        assert ConfigSource._normalize_file_name("config.yaml") == "config.yaml"
        assert ConfigSource._normalize_file_name("test") == "test.yaml"

    def test_core_utils_get_valid_filename_uses_rust(self):
        """Test that core.utils.get_valid_filename uses Rust"""
        from lerna.core.utils import get_valid_filename

        assert get_valid_filename("my app") == "my_app"
        assert get_valid_filename("file@123") == "file123"
        assert get_valid_filename("test-file.py") == "test-file.py"

    def test_grammar_escape_special_chars_uses_rust(self):
        """Test that grammar.utils.escape_special_characters uses Rust"""
        from lerna._internal.grammar.utils import escape_special_characters

        assert escape_special_characters("hello") == "hello"
        assert escape_special_characters("a b") == "a\\ b"
        assert escape_special_characters("a=b") == "a\\=b"
        assert escape_special_characters("a[0]") == "a\\[0\\]"

    def test_glob_filter_uses_rust(self):
        """Test that Glob.filter uses Rust"""
        from lerna.core.override_parser.types import Glob

        g = Glob(include=["*.yaml", "*.py"], exclude=["test*"])
        names = ["config.yaml", "test.yaml", "app.py", "test.py", "readme.md"]
        result = g.filter(names)

        assert "config.yaml" in result
        assert "app.py" in result
        assert "test.yaml" not in result  # excluded
        assert "test.py" not in result  # excluded
        assert "readme.md" not in result  # not included

    def test_config_repository_get_scheme_uses_rust(self):
        """Test that ConfigRepository._get_scheme uses Rust"""
        from lerna._internal.config_repository import ConfigRepository

        assert ConfigRepository._get_scheme("file://path") == "file"
        assert ConfigRepository._get_scheme("pkg://module") == "pkg"
        assert ConfigRepository._get_scheme("/absolute/path") == "file"
        assert ConfigRepository._get_scheme("structured://") == "structured"

    def test_importlib_resources_config_source_uses_rust(self):
        """Test that ImportlibResourcesConfigSource uses Rust for header extraction."""
        from lerna._internal.core_plugins.importlib_resources_config_source import _RUST_AVAILABLE, ImportlibResourcesConfigSource

        # Verify Rust is available
        assert _RUST_AVAILABLE, "Rust module should be available"

        # Create a source pointing to the lerna conf package
        source = ImportlibResourcesConfigSource(provider="test", path="pkg://lerna.conf")
        assert source.available(), "lerna.conf should be available"

    def test_file_config_source_uses_rust(self):
        """Test that FileConfigSource uses Rust for YAML parsing and header extraction."""
        import os
        import tempfile

        from lerna._internal.core_plugins.file_config_source import _RUST_AVAILABLE, FileConfigSource

        # Verify Rust is available
        assert _RUST_AVAILABLE, "Rust module should be available"

        # Create a test config file
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = os.path.join(tmpdir, "test.yaml")
            with open(config_path, "w") as f:
                f.write("# @package _global_\ndb:\n  host: localhost\n  port: 3306\n")

            source = FileConfigSource(provider="test", path=f"file://{tmpdir}")
            result = source.load_config("test")

            assert result.header.get("package") == "_global_"
            assert result.config.db.host == "localhost"
            assert result.config.db.port == 3306

    def test_file_config_source_empty_file(self):
        """Test that FileConfigSource handles empty files correctly."""
        import os
        import tempfile

        from lerna._internal.core_plugins.file_config_source import FileConfigSource

        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = os.path.join(tmpdir, "empty.yaml")
            with open(config_path, "w") as f:
                f.write("")  # Empty file

            source = FileConfigSource(provider="test", path=f"file://{tmpdir}")
            result = source.load_config("empty")

            # Should return empty config, not None
            assert result.config is not None
            assert len(result.config) == 0
