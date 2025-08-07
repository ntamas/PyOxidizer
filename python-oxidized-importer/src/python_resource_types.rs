// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/*! Defines Python type objects that represent Python resources. */

use {
    crate::conversion::pyobject_to_owned_bytes,
    pyo3::{
        exceptions::{PyTypeError, PyValueError},
        prelude::*,
        types::PyBytes,
    },
    python_packaging::resource::{
        BytecodeOptimizationLevel, PythonExtensionModule as RawPythonExtensionModule,
        PythonModuleBytecode as RawPythonModuleBytecode,
        PythonModuleSource as RawPythonModuleSource,
        PythonPackageDistributionResource as RawPythonPackageDistributionResource,
        PythonPackageResource as RawPythonPackageResource,
    },
    simple_file_manifest::FileData,
    std::sync::{RwLock, RwLockReadGuard},
};

#[pyclass(module = "oxidized_importer")]
pub(crate) struct PythonModuleSource {
    resource: RwLock<RawPythonModuleSource>,
}

impl PythonModuleSource {
    pub fn new(py: Python, resource: RawPythonModuleSource) -> PyResult<Bound<Self>> {
        Bound::new(
            py,
            PythonModuleSource {
                resource: RwLock::new(resource),
            },
        )
    }

    pub fn get_resource(&self) -> RwLockReadGuard<'_, RawPythonModuleSource> {
        self.resource.read().unwrap()
    }
}

#[pymethods]
impl PythonModuleSource {
    fn __repr__(&self) -> String {
        format!(
            "<PythonModuleSource module=\"{}\">",
            self.resource.read().unwrap().name
        )
    }

    #[getter]
    fn get_module(&self) -> PyResult<String> {
        Ok(self.resource.read().unwrap().name.to_string())
    }

    #[setter]
    fn set_module(&self, value: Option<&str>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().name = value.to_string();

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete module"))
        }
    }

    #[getter]
    fn get_source<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyBytes>> {
        let source = self
            .resource
            .read()
            .unwrap()
            .source
            .resolve_content()
            .map_err(|_| PyValueError::new_err("error resolving source code"))?;

        Ok(PyBytes::new(py, &source))
    }

    #[setter]
    fn set_source(&self, value: Option<&Bound<PyAny>>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().source = FileData::Memory(pyobject_to_owned_bytes(value)?);

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete source"))
        }
    }

    #[getter]
    fn get_is_package(&self) -> bool {
        self.resource.read().unwrap().is_package
    }

    #[setter]
    fn set_is_package(&self, value: Option<bool>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().is_package = value;

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete is_package"))
        }
    }
}

#[pyclass(module = "oxidized_importer")]
pub(crate) struct PythonModuleBytecode {
    resource: RwLock<RawPythonModuleBytecode>,
}

impl PythonModuleBytecode {
    pub fn new(py: Python, resource: RawPythonModuleBytecode) -> PyResult<Bound<Self>> {
        Bound::new(
            py,
            Self {
                resource: RwLock::new(resource),
            },
        )
    }

    pub fn get_resource(&self) -> RwLockReadGuard<'_, RawPythonModuleBytecode> {
        self.resource.read().unwrap()
    }
}

#[pymethods]
impl PythonModuleBytecode {
    fn __repr__(&self) -> String {
        format!(
            "<PythonModuleBytecode module=\"{}\">",
            self.resource.read().unwrap().name
        )
    }

    #[getter]
    fn get_module(&self) -> String {
        self.resource.read().unwrap().name.to_string()
    }

    #[setter]
    fn set_module(&self, value: Option<&str>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().name = value.to_string();

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete module"))
        }
    }

    #[getter]
    fn get_bytecode<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyBytes>> {
        let bytecode = self
            .resource
            .read().unwrap()
            .resolve_bytecode()
            .map_err(|_| PyValueError::new_err("error resolving bytecode"))?;

        Ok(PyBytes::new(py, &bytecode))
    }

    #[setter]
    fn set_bytecode(&self, value: Option<&Bound<PyAny>>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource
                .write().unwrap()
                .set_bytecode(&pyobject_to_owned_bytes(value)?);

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete bytecode"))
        }
    }

    #[getter]
    fn get_optimize_level(&self) -> i32 {
        self.resource.read().unwrap().optimize_level.into()
    }

    #[setter]
    fn set_optimize_level(&self, value: Option<i32>) -> PyResult<()> {
        if let Some(value) = value {
            let value = BytecodeOptimizationLevel::try_from(value)
                .map_err(|_| PyValueError::new_err("invalid bytecode optimization level"))?;

            self.resource.write().unwrap().optimize_level = value;

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete optimize_level"))
        }
    }

    #[getter]
    fn get_is_package(&self) -> bool {
        self.resource.read().unwrap().is_package
    }

    #[setter]
    fn set_is_package(&self, value: Option<bool>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().is_package = value;

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannote delete is_package"))
        }
    }
}

#[pyclass(module = "oxidized_importer")]
pub(crate) struct PythonPackageResource {
    resource: RwLock<RawPythonPackageResource>,
}

impl PythonPackageResource {
    pub fn new(py: Python, resource: RawPythonPackageResource) -> PyResult<Bound<Self>> {
        Bound::new(
            py,
            Self {
                resource: RwLock::new(resource),
            },
        )
    }

    pub fn get_resource(&self) -> RwLockReadGuard<'_, RawPythonPackageResource> {
        self.resource.read().unwrap()
    }
}

#[pymethods]
impl PythonPackageResource {
    fn __repr__(&self) -> String {
        let resource = self.resource.read().unwrap();
        format!(
            "<PythonPackageResource package=\"{}\", path=\"{}\">",
            resource.leaf_package, resource.relative_name
        )
    }

    #[getter]
    fn get_package(&self) -> String {
        self.resource.read().unwrap().leaf_package.clone()
    }

    #[setter]
    fn set_package(&self, value: Option<&str>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().leaf_package = value.to_string();

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete package"))
        }
    }

    #[getter]
    fn get_name(&self) -> String {
        self.resource.read().unwrap().relative_name.clone()
    }

    #[setter]
    fn set_name(&self, value: Option<&str>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().relative_name = value.to_string();

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete name"))
        }
    }

    #[getter]
    fn get_data<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyBytes>> {
        let data = self
            .resource
            .read().unwrap()
            .data
            .resolve_content()
            .map_err(|_| PyValueError::new_err("error resolving data"))?;

        Ok(PyBytes::new(py, &data))
    }

    #[setter]
    fn set_data(&self, value: Option<&Bound<PyAny>>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().data = FileData::Memory(pyobject_to_owned_bytes(value)?);

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete data"))
        }
    }
}

#[pyclass(module = "oxidized_importer")]
pub(crate) struct PythonPackageDistributionResource {
    resource: RwLock<RawPythonPackageDistributionResource>,
}

impl PythonPackageDistributionResource {
    pub fn new(
        py: Python,
        resource: RawPythonPackageDistributionResource,
    ) -> PyResult<Bound<Self>> {
        Bound::new(
            py,
            Self {
                resource: RwLock::new(resource),
            },
        )
    }

    pub fn get_resource(&self) -> RwLockReadGuard<'_, RawPythonPackageDistributionResource> {
        self.resource.read().unwrap()
    }
}

#[pymethods]
impl PythonPackageDistributionResource {
    fn __repr__(&self) -> String {
        let resource = self.resource.read().unwrap();
        format!(
            "<PythonPackageDistributionResource package=\"{}\", path=\"{}\">",
            resource.package, resource.name
        )
    }

    #[getter]
    fn get_package(&self) -> String {
        self.resource.read().unwrap().package.clone()
    }

    #[setter]
    fn set_package(&self, value: Option<&str>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().package = value.to_string();

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete package"))
        }
    }

    #[getter]
    fn get_version(&self) -> String {
        self.resource.read().unwrap().version.clone()
    }

    #[setter]
    fn set_version(&self, value: Option<&str>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().version = value.to_string();

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannote delete version"))
        }
    }

    #[getter]
    fn get_name(&self) -> String {
        self.resource.read().unwrap().name.clone()
    }

    #[setter]
    fn set_name(&self, value: Option<&str>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().name = value.to_string();

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete name"))
        }
    }

    #[getter]
    fn get_data<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyBytes>> {
        let data = self
            .resource
            .read().unwrap()
            .data
            .resolve_content()
            .map_err(|_| PyValueError::new_err("error resolving data"))?;

        Ok(PyBytes::new(py, &data))
    }

    #[setter]
    fn set_data(&self, value: Option<&Bound<PyAny>>) -> PyResult<()> {
        if let Some(value) = value {
            self.resource.write().unwrap().data = FileData::Memory(pyobject_to_owned_bytes(value)?);

            Ok(())
        } else {
            Err(PyTypeError::new_err("cannot delete data"))
        }
    }
}

#[pyclass(module = "oxidized_importer")]
pub(crate) struct PythonExtensionModule {
    resource: RwLock<RawPythonExtensionModule>,
}

impl PythonExtensionModule {
    pub fn new(py: Python, resource: RawPythonExtensionModule) -> PyResult<Bound<Self>> {
        Bound::new(
            py,
            Self {
                resource: RwLock::new(resource),
            },
        )
    }

    pub fn get_resource(&self) -> RwLockReadGuard<'_, RawPythonExtensionModule> {
        self.resource.read().unwrap()
    }
}

#[pymethods]
impl PythonExtensionModule {
    fn __repr__(&self) -> String {
        format!(
            "<PythonExtensionModule module=\"{}\">",
            self.resource.read().unwrap().name
        )
    }

    #[getter]
    fn name(&self) -> String {
        self.resource.read().unwrap().name.clone()
    }
}
