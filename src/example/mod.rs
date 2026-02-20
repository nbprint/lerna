use pyo3::prelude::*;

use lerna::Example as BaseExample;

#[pyclass]
pub struct Example {
    pub example: BaseExample,
}

#[pymethods]
impl Example {
    #[new]
    fn py_new(value: String) -> PyResult<Self> {
        Ok(Example {
            example: BaseExample {
                stuff: value.as_str().to_string(),
            },
        })
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{}", self.example.stuff))
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("Example<{}>", self.example.stuff))
    }
}
