// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for default element types

use pyo3::prelude::*;
use pyo3::types::PyList;
use lerna::{
    ResultDefault, ConfigDefault, GroupDefault, GroupValue
};

/// Result of resolving a default
#[pyclass(name = "ResultDefault")]
#[derive(Clone)]
pub struct PyResultDefault {
    inner: ResultDefault,
}

#[pymethods]
impl PyResultDefault {
    #[new]
    #[pyo3(signature = (config_path=None, parent=None, package=None, is_self=false, primary=false, override_key=None))]
    fn new(
        config_path: Option<String>,
        parent: Option<String>,
        package: Option<String>,
        is_self: bool,
        primary: bool,
        override_key: Option<String>,
    ) -> Self {
        Self {
            inner: ResultDefault {
                config_path,
                parent,
                package,
                is_self,
                primary,
                override_key,
            },
        }
    }

    #[getter]
    fn config_path(&self) -> Option<String> {
        self.inner.config_path.clone()
    }

    #[setter]
    fn set_config_path(&mut self, value: Option<String>) {
        self.inner.config_path = value;
    }

    #[getter]
    fn parent(&self) -> Option<String> {
        self.inner.parent.clone()
    }

    #[setter]
    fn set_parent(&mut self, value: Option<String>) {
        self.inner.parent = value;
    }

    #[getter]
    fn package(&self) -> Option<String> {
        self.inner.package.clone()
    }

    #[setter]
    fn set_package(&mut self, value: Option<String>) {
        self.inner.package = value;
    }

    #[getter]
    fn is_self(&self) -> bool {
        self.inner.is_self
    }

    #[setter]
    fn set_is_self(&mut self, value: bool) {
        self.inner.is_self = value;
    }

    #[getter]
    fn primary(&self) -> bool {
        self.inner.primary
    }

    #[setter]
    fn set_primary(&mut self, value: bool) {
        self.inner.primary = value;
    }

    #[getter]
    fn override_key(&self) -> Option<String> {
        self.inner.override_key.clone()
    }

    #[setter]
    fn set_override_key(&mut self, value: Option<String>) {
        self.inner.override_key = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "ResultDefault(config_path={:?}, parent={:?}, package={:?}, is_self={}, primary={}, override_key={:?})",
            self.inner.config_path,
            self.inner.parent,
            self.inner.package,
            self.inner.is_self,
            self.inner.primary,
            self.inner.override_key,
        )
    }
}

/// A config file default
#[pyclass(name = "ConfigDefault")]
#[derive(Clone)]
pub struct PyConfigDefault {
    inner: ConfigDefault,
}

#[pymethods]
impl PyConfigDefault {
    #[new]
    #[pyo3(signature = (path=None, package=None, optional=false, deleted=false))]
    fn new(path: Option<String>, package: Option<String>, optional: bool, deleted: bool) -> Self {
        let mut cd = match path {
            Some(p) => ConfigDefault::new(p),
            None => ConfigDefault::default(),
        };
        cd.base.package = package;
        cd.optional = optional;
        cd.deleted = deleted;
        Self { inner: cd }
    }

    #[getter]
    fn path(&self) -> Option<String> {
        self.inner.path.clone()
    }

    #[setter]
    fn set_path(&mut self, value: Option<String>) {
        self.inner.path = value;
    }

    #[getter]
    fn package(&self) -> Option<String> {
        self.inner.base.package.clone()
    }

    #[setter]
    fn set_package(&mut self, value: Option<String>) {
        self.inner.base.package = value;
    }

    #[getter]
    fn optional(&self) -> bool {
        self.inner.optional
    }

    #[setter]
    fn set_optional(&mut self, value: bool) {
        self.inner.optional = value;
    }

    #[getter]
    fn deleted(&self) -> bool {
        self.inner.deleted
    }

    #[setter]
    fn set_deleted(&mut self, value: bool) {
        self.inner.deleted = value;
    }

    #[getter]
    fn parent_base_dir(&self) -> Option<String> {
        self.inner.base.parent_base_dir.clone()
    }

    #[setter]
    fn set_parent_base_dir(&mut self, value: Option<String>) {
        self.inner.base.parent_base_dir = value;
    }

    #[getter]
    fn parent_package(&self) -> Option<String> {
        self.inner.base.parent_package.clone()
    }

    #[setter]
    fn set_parent_package(&mut self, value: Option<String>) {
        self.inner.base.parent_package = value;
    }

    #[getter]
    fn package_header(&self) -> Option<String> {
        self.inner.base.package_header.clone()
    }

    #[setter]
    fn set_package_header(&mut self, value: Option<String>) {
        self.inner.base.package_header = value;
    }

    #[getter]
    fn primary(&self) -> bool {
        self.inner.base.primary
    }

    #[setter]
    fn set_primary(&mut self, value: bool) {
        self.inner.base.primary = value;
    }

    fn is_self(&self) -> bool {
        self.inner.is_self()
    }

    fn get_name(&self) -> Option<String> {
        self.inner.get_name().map(|s| s.to_string())
    }

    fn get_group_path(&self) -> String {
        self.inner.get_group_path()
    }

    fn get_config_path(&self) -> String {
        self.inner.get_config_path()
    }

    fn get_default_package(&self) -> String {
        self.inner.get_default_package()
    }

    fn update_parent(&mut self, parent_base_dir: Option<String>, parent_package: Option<String>) {
        self.inner.base.update_parent(parent_base_dir, parent_package);
    }

    fn __repr__(&self) -> String {
        format!("ConfigDefault(path={:?}, package={:?}, optional={}, deleted={})",
            self.inner.path, self.inner.base.package, self.inner.optional, self.inner.deleted)
    }
}

/// A config group default
#[pyclass(name = "GroupDefault")]
#[derive(Clone)]
pub struct PyGroupDefault {
    inner: GroupDefault,
}

#[pymethods]
impl PyGroupDefault {
    #[new]
    #[pyo3(signature = (group, value=None, package=None, optional=false, deleted=false, is_override=false, external_append=false, config_name_overridden=false))]
    fn new(
        group: String,
        value: Option<Py<PyAny>>,
        package: Option<String>,
        optional: bool,
        deleted: bool,
        is_override: bool,
        external_append: bool,
        config_name_overridden: bool,
        py: Python,
    ) -> PyResult<Self> {
        let group_value = match value {
            Some(v) => {
                if let Ok(s) = v.extract::<String>(py) {
                    GroupValue::Single(s)
                } else if let Ok(list) = v.cast_bound::<PyList>(py) {
                    let values: Vec<String> = list.iter()
                        .filter_map(|item| item.extract::<String>().ok())
                        .collect();
                    GroupValue::Multiple(values)
                } else {
                    GroupValue::Single("???".to_string())
                }
            }
            None => GroupValue::Single("???".to_string()),
        };

        let mut gd = match group_value {
            GroupValue::Single(ref v) => GroupDefault::new(group.clone(), v.clone()),
            GroupValue::Multiple(ref v) => GroupDefault::new_multi(group.clone(), v.clone()),
        };
        gd.value = group_value;
        gd.base.package = package;
        gd.optional = optional;
        gd.deleted = deleted;
        gd.is_override = is_override;
        gd.external_append = external_append;
        gd.config_name_overridden = config_name_overridden;

        Ok(Self { inner: gd })
    }

    #[getter]
    fn group(&self) -> String {
        self.inner.group.clone()
    }

    #[setter]
    fn set_group(&mut self, value: String) {
        self.inner.group = value;
    }

    #[getter]
    fn value(&self, py: Python) -> Py<PyAny> {
        match &self.inner.value {
            GroupValue::Single(s) => s.clone().into_pyobject(py).unwrap().into_any().unbind(),
            GroupValue::Multiple(v) => v.clone().into_pyobject(py).unwrap().into_any().unbind(),
        }
    }

    fn set_value_py(&mut self, value: Py<PyAny>, py: Python) -> PyResult<()> {
        if let Ok(s) = value.extract::<String>(py) {
            self.inner.value = GroupValue::Single(s);
        } else if let Ok(list) = value.cast_bound::<PyList>(py) {
            let values: Vec<String> = list.iter()
                .filter_map(|item| item.extract::<String>().ok())
                .collect();
            self.inner.value = GroupValue::Multiple(values);
        }
        Ok(())
    }

    #[setter]
    fn set_value(&mut self, value: String) {
        self.inner.value = GroupValue::Single(value);
    }

    #[getter]
    fn package(&self) -> Option<String> {
        self.inner.base.package.clone()
    }

    #[setter]
    fn set_package(&mut self, value: Option<String>) {
        self.inner.base.package = value;
    }

    #[getter]
    fn optional(&self) -> bool {
        self.inner.optional
    }

    #[setter]
    fn set_optional(&mut self, value: bool) {
        self.inner.optional = value;
    }

    #[getter]
    fn deleted(&self) -> bool {
        self.inner.deleted
    }

    #[setter]
    fn set_deleted(&mut self, value: bool) {
        self.inner.deleted = value;
    }

    #[getter]
    fn is_override(&self) -> bool {
        self.inner.is_override
    }

    #[setter]
    fn set_is_override(&mut self, value: bool) {
        self.inner.is_override = value;
    }

    #[getter]
    fn external_append(&self) -> bool {
        self.inner.external_append
    }

    #[setter]
    fn set_external_append(&mut self, value: bool) {
        self.inner.external_append = value;
    }

    #[getter]
    fn config_name_overridden(&self) -> bool {
        self.inner.config_name_overridden
    }

    #[setter]
    fn set_config_name_overridden(&mut self, value: bool) {
        self.inner.config_name_overridden = value;
    }

    #[getter]
    fn parent_base_dir(&self) -> Option<String> {
        self.inner.base.parent_base_dir.clone()
    }

    #[setter]
    fn set_parent_base_dir(&mut self, value: Option<String>) {
        self.inner.base.parent_base_dir = value;
    }

    #[getter]
    fn parent_package(&self) -> Option<String> {
        self.inner.base.parent_package.clone()
    }

    #[setter]
    fn set_parent_package(&mut self, value: Option<String>) {
        self.inner.base.parent_package = value;
    }

    #[getter]
    fn package_header(&self) -> Option<String> {
        self.inner.base.package_header.clone()
    }

    #[setter]
    fn set_package_header(&mut self, value: Option<String>) {
        self.inner.base.package_header = value;
    }

    #[getter]
    fn primary(&self) -> bool {
        self.inner.base.primary
    }

    #[setter]
    fn set_primary(&mut self, value: bool) {
        self.inner.base.primary = value;
    }

    fn get_group_path(&self) -> String {
        self.inner.get_group_path()
    }

    #[pyo3(signature = (value=None))]
    fn get_config_path(&self, value: Option<String>) -> String {
        match value {
            Some(v) => self.inner.get_config_path(&v),
            None => {
                match &self.inner.value {
                    GroupValue::Single(v) => self.inner.get_config_path(v),
                    GroupValue::Multiple(vs) if !vs.is_empty() => self.inner.get_config_path(&vs[0]),
                    _ => self.inner.get_group_path(),
                }
            }
        }
    }

    fn get_default_package(&self) -> String {
        self.inner.get_default_package()
    }

    fn get_override_key(&self) -> String {
        self.inner.get_override_key()
    }

    #[pyo3(signature = (default_to_package_header=true))]
    fn get_final_package(&self, default_to_package_header: bool) -> String {
        self.inner.get_final_package(default_to_package_header)
    }

    fn is_missing(&self) -> bool {
        self.inner.is_missing()
    }

    fn update_parent(&mut self, parent_base_dir: Option<String>, parent_package: Option<String>) {
        self.inner.base.update_parent(parent_base_dir, parent_package);
    }

    fn __repr__(&self) -> String {
        let value_str = match &self.inner.value {
            GroupValue::Single(s) => format!("{:?}", s),
            GroupValue::Multiple(v) => format!("{:?}", v),
        };
        format!("GroupDefault(group={:?}, value={}, package={:?}, optional={}, deleted={})",
            self.inner.group, value_str, self.inner.base.package, self.inner.optional, self.inner.deleted)
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyResultDefault>()?;
    m.add_class::<PyConfigDefault>()?;
    m.add_class::<PyGroupDefault>()?;
    Ok(())
}
