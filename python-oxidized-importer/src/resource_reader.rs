// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use {
    crate::importer::ImporterState,
    pyo3::{
        exceptions::{
            PyFileNotFoundError, PyIsADirectoryError, PyNotADirectoryError, PyValueError,
        },
        prelude::*,
        types::{PyDict, PyIterator, PyList, PyTuple},
    },
    std::{ffi::OsString, path::PathBuf, sync::Arc},
};

/// Implements in-memory reading of resource data.
///
/// Implements importlib.abc.ResourceReader.
#[pyclass(module = "oxidized_importer")]
pub(crate) struct OxidizedResourceReader {
    state: Arc<ImporterState>,
    package: String,
}

impl OxidizedResourceReader {
    pub(crate) fn new(state: Arc<ImporterState>, package: String) -> Self {
        Self { state, package }
    }
}

#[pymethods]
impl OxidizedResourceReader {
    /// Returns an opened, file-like object for binary reading of the resource.
    ///
    /// If the resource cannot be found, FileNotFoundError is raised.
    fn open_resource<'p>(&self, py: Python<'p>, resource: &str) -> PyResult<Bound<'p, PyAny>> {
        if let Some(file) = self.state.get_resources_state().get_package_resource_file(
            py,
            &self.package,
            resource,
        )? {
            Ok(file)
        } else {
            Err(PyFileNotFoundError::new_err("resource not found"))
        }
    }

    /// Returns the file system path to the resource.
    ///
    /// If the resource does not concretely exist on the file system, raise
    /// FileNotFoundError.
    fn resource_path(&self, resource: &str) -> PyResult<OsString> {
        self.state
            .get_resources_state()
            .package_resource_path(&self.package, resource)
            .map(PathBuf::into_os_string)
            .ok_or_else(|| {
                PyFileNotFoundError::new_err("resource does not exist on the filesystem")
            })
    }

    /// Returns True if the named name is considered a resource. FileNotFoundError
    /// is raised if name does not exist.
    fn is_resource(&self, name: &str) -> PyResult<bool> {
        if self
            .state
            .get_resources_state()
            .is_package_resource(&self.package, name)
        {
            Ok(true)
        } else {
            Err(PyFileNotFoundError::new_err("resource not found"))
        }
    }

    /// Returns an iterable of strings over the contents of the package.
    ///
    /// Do note that it is not required that all names returned by the iterator be actual resources,
    /// e.g. it is acceptable to return names for which is_resource() would be false.
    ///
    /// Allowing non-resource names to be returned is to allow for situations where how a package
    /// and its resources are stored are known a priori and the non-resource names would be useful.
    /// For instance, returning subdirectory names is allowed so that when it is known that the
    /// package and resources are stored on the file system then those subdirectory names can be
    /// used directly.
    fn contents<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyList>> {
        self.state
            .get_resources_state()
            .package_resource_names(py, &self.package)
    }

    /// Returns a Traversable object for the package.
    ///
    /// Implements importlib.resources.abc.TraversableResources.files().
    fn files(&self) -> OxidizedResourceTraversable {
        OxidizedResourceTraversable {
            state: self.state.clone(),
            package: self.package.clone(),
            path: "".to_string(),
        }
    }
}

/// Path-like object facilitating Python resource access.
///
/// Implements importlib.resources.abc.Traversable.
///
/// Instances are bound to a Python package and a resource path relative to
/// that package. The empty path denotes the package itself. Child packages
/// are treated as directories of their parent package.
///
/// Resources stored on the filesystem are returned as `pathlib.Path`
/// instances instead. `importlib.resources.as_file()` uses those directly
/// instead of copying the resource to a temporary file.
#[pyclass(module = "oxidized_importer")]
pub(crate) struct OxidizedResourceTraversable {
    state: Arc<ImporterState>,
    package: String,
    path: String,
}

impl OxidizedResourceTraversable {
    /// Obtain the Traversable for a named entry in this directory.
    fn child(&self, name: &str) -> Self {
        if self.path.is_empty() {
            let package = format!("{}.{}", self.package, name);

            if let Some(entry) = self
                .state
                .get_resources_state()
                .resolve_importable_module(&package, self.state.optimize_level)
            {
                if entry.is_package {
                    return Self {
                        state: self.state.clone(),
                        package,
                        path: "".to_string(),
                    };
                }
            }
        }

        let path = if self.path.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", self.path, name)
        };

        Self {
            state: self.state.clone(),
            package: self.package.clone(),
            path,
        }
    }

    /// Obtain the Traversable for a relative path below this one.
    fn descendant(&self, path: &str) -> Self {
        let mut result = Self {
            state: self.state.clone(),
            package: self.package.clone(),
            path: self.path.clone(),
        };

        for name in path
            .split('/')
            .filter(|name| !name.is_empty() && *name != ".")
        {
            result = result.child(name);
        }

        result
    }

    /// Convert into the Python object to hand out for this resource.
    ///
    /// This is a `pathlib.Path` for a resource stored on the filesystem and
    /// the instance itself otherwise.
    fn into_python<'p>(self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        if !self.path.is_empty() {
            if let Some(path) = self
                .state
                .get_resources_state()
                .package_resource_path(&self.package, &self.path)
            {
                return py.import("pathlib")?.getattr("Path")?.call1((path,));
            }
        }

        Ok(Bound::new(py, self)?.into_any())
    }

    /// Open the resource as a binary file-like object.
    fn open_binary<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        if self.is_dir() {
            return Err(PyIsADirectoryError::new_err(format!(
                "resource is a directory: {}",
                self.__repr__()
            )));
        }

        self.state
            .get_resources_state()
            .get_package_resource_file(py, &self.package, &self.path)?
            .ok_or_else(|| {
                PyFileNotFoundError::new_err(format!("resource not found: {}", self.__repr__()))
            })
    }
}

#[pymethods]
impl OxidizedResourceTraversable {
    fn __repr__(&self) -> String {
        format!(
            "OxidizedResourceTraversable(package={:?}, path={:?})",
            self.package, self.path
        )
    }

    /// The base name of this object without any parent references.
    #[getter]
    fn name(&self) -> String {
        if self.path.is_empty() {
            self.package.rsplit('.').next().unwrap_or_default()
        } else {
            self.path.rsplit('/').next().unwrap_or_default()
        }
        .to_string()
    }

    /// Yield Traversable objects in self.
    fn iterdir<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyIterator>> {
        if self.is_file() {
            return Err(PyNotADirectoryError::new_err(format!(
                "resource is not a directory: {}",
                self.__repr__()
            )));
        } else if !self.is_dir() {
            return Err(PyFileNotFoundError::new_err(format!(
                "resource not found: {}",
                self.__repr__()
            )));
        }

        let resources_state = self.state.get_resources_state();

        let mut names =
            resources_state.package_resources_directory_entries(&self.package, &self.path);

        if self.path.is_empty() {
            names.extend(resources_state.child_package_names(&self.package));
            names.sort();
            names.dedup();
        }

        let children = names
            .iter()
            .map(|name| self.child(name).into_python(py))
            .collect::<PyResult<Vec<_>>>()?;

        PyList::new(py, children)?.try_iter()
    }

    /// Read contents of self as bytes.
    fn read_bytes<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let fh = self.open_binary(py)?;
        let data = fh.call_method0("read")?;
        fh.call_method0("close")?;

        Ok(data)
    }

    /// Read contents of self as text.
    #[pyo3(signature = (encoding=None, errors=None))]
    fn read_text<'p>(
        &self,
        py: Python<'p>,
        encoding: Option<&str>,
        errors: Option<&str>,
    ) -> PyResult<Bound<'p, PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("encoding", encoding)?;
        kwargs.set_item("errors", errors)?;

        let fh = py
            .import("io")?
            .getattr("TextIOWrapper")?
            .call((self.open_binary(py)?,), Some(&kwargs))?;
        let data = fh.call_method0("read");
        fh.call_method0("close")?;

        data
    }

    /// Return True if self is a dir.
    fn is_dir(&self) -> bool {
        self.path.is_empty()
            || self
                .state
                .get_resources_state()
                .is_package_resource_directory(&self.package, &self.path)
    }

    /// Return True if self is a file.
    fn is_file(&self) -> bool {
        !self.path.is_empty()
            && self
                .state
                .get_resources_state()
                .is_package_resource(&self.package, &self.path)
    }

    /// Return Traversable resolved with any descendants applied.
    #[pyo3(signature = (*descendants))]
    fn joinpath<'p>(
        &self,
        py: Python<'p>,
        descendants: &Bound<'p, PyTuple>,
    ) -> PyResult<Bound<'p, PyAny>> {
        let mut result = self.descendant("");

        for descendant in descendants.iter() {
            result = result.descendant(&descendant.extract::<String>()?);
        }

        result.into_python(py)
    }

    /// Return Traversable child in self.
    fn __truediv__<'p>(&self, py: Python<'p>, child: &str) -> PyResult<Bound<'p, PyAny>> {
        self.descendant(child).into_python(py)
    }

    /// mode may be 'r' or 'rb' to open as text or binary. Return a handle
    /// suitable for reading (same as pathlib.Path.open).
    ///
    /// When opening as text, accepts encoding parameters such as those
    /// accepted by io.TextIOWrapper.
    #[pyo3(signature = (mode="r", *args, **kwargs))]
    fn open<'p>(
        &self,
        py: Python<'p>,
        mode: &str,
        args: &Bound<'p, PyTuple>,
        kwargs: Option<&Bound<'p, PyDict>>,
    ) -> PyResult<Bound<'p, PyAny>> {
        match mode {
            "rb" => self.open_binary(py),
            "r" => {
                let args = PyTuple::new(
                    py,
                    std::iter::once(self.open_binary(py)?)
                        .chain(args.iter())
                        .collect::<Vec<_>>(),
                )?;

                py.import("io")?
                    .getattr("TextIOWrapper")?
                    .call(args, kwargs)
            }
            _ => Err(PyValueError::new_err(format!(
                "invalid mode: {:?}; only 'r' and 'rb' are supported",
                mode
            ))),
        }
    }
}
