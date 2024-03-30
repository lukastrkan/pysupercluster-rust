use geojson::Feature;
use geojson::Geometry;
use geojson::JsonObject;
use geojson::Value::Point;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use supercluster::Options;
use supercluster::Supercluster;

#[pyclass]
struct PySupercluster {
    inner: Supercluster,
}

#[pymethods]
impl PySupercluster {
    #[new]
    #[pyo3(signature = (min_zoom=0, max_zoom=16, min_points=2, radius=40.0, extent=512.0, node_size=64))]
    fn new(
        min_zoom: u8,
        max_zoom: u8,
        min_points: u8,
        radius: f64,
        extent: f64,
        node_size: usize,
    ) -> Self {
        let options = Options {
            min_zoom,
            max_zoom,
            min_points,
            radius,
            extent,
            node_size,
        };
        PySupercluster {
            inner: Supercluster::new(options),
        }
    }

    #[pyo3(signature = (points))]
    fn load(&mut self, _py: Python, points: Vec<&PyDict>) -> PyResult<()> {
        let features: Vec<Feature> = points
            .into_iter()
            .map(|p| {
                let geometry = p
                    .get_item("geometry")
                    .expect("Geometry not found")
                    .unwrap()
                    .extract::<&PyDict>()?;
                let properties = p
                    .get_item("properties")
                    .expect("Properties not found")
                    .unwrap()
                    .extract::<&PyDict>()?;
                let coords = geometry
                    .get_item("coordinates")
                    .expect("Coordinates not found")
                    .unwrap()
                    .extract::<&PyList>()?;

                let latitude = coords.get_item(1).unwrap().extract::<f64>().unwrap();
                let longitude = coords.get_item(0).unwrap().extract::<f64>().unwrap();     

                //hacky way to convert PyDict to json string
                let json_properities = properties.to_string().replace("'", "\"");
                    
                Ok(Feature {
                    geometry: Some(Geometry::new(Point(vec![longitude, latitude]))),                                        
                    properties: Some(
                        serde_json::from_str(&json_properities)
                            .unwrap_or_else(|_| JsonObject::new()),
                    ),
                    ..Default::default()
                })
            })
            .collect::<PyResult<Vec<Feature>>>()?;

        self.inner.load(features);

        Ok(())
    }

    
    #[pyo3(signature = (bbox, zoom))]
    fn get_clusters(&self, py: Python, bbox: [f64;4], zoom: u8) -> PyResult<Vec<PyObject>> {
        let clusters = self.inner.get_clusters(bbox, zoom);
        let mut py_clusters = Vec::new();
        for cluster in clusters {
            let py_cluster = PyDict::new(py);
            if let Some(geometry) = &cluster.geometry {
                let geometry_dict = PyDict::new(py);
                geometry_dict.set_item("type", "Point")?;

                match &geometry.value {
                    geojson::Value::Point(coords) => {
                        geometry_dict.set_item("coordinates", coords)?;
                    },
                    _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Expected point geometry")),
                }
                
                py_cluster.set_item("geometry", geometry_dict)?;
            }

            if let Some(properties) = &cluster.properties {
                let properties_dict = PyDict::new(py);
                for (key, value) in properties {
                    let py_value = json_to_pyobject(py, value);
                    properties_dict.set_item(key, py_value)?;
                }
                py_cluster.set_item("properties", properties_dict)?;
            } else {
                py_cluster.set_item("properties", PyDict::new(py))?;
            }

            py_cluster.set_item("type", "Feature")?;
            py_clusters.push(py_cluster.to_object(py));
        }
        Ok(py_clusters)
    }

    fn get_cluster_expansion_zoom(&self, cluster_id: usize) -> PyResult<usize> {
        let expansion_zoom = self.inner.get_cluster_expansion_zoom(cluster_id);
        Ok(expansion_zoom)
    }
}

#[pymodule]
fn pysupercluster(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PySupercluster>()?;
    Ok(())
}

fn json_to_pyobject(py: Python, value: &serde_json::Value) -> PyObject {
    match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.into_py(py),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py(py)
            } else if let Some(f) = n.as_f64() {
                f.into_py(py)
            } else {
                py.None()
            }
        },
        serde_json::Value::String(s) => s.into_py(py),
        serde_json::Value::Array(arr) => {
            let py_list = PyList::empty(py);
            for item in arr {
                let py_item = json_to_pyobject(py, item);
                py_list.append(py_item).unwrap();
            }
            py_list.into()
        },
        serde_json::Value::Object(obj) => {
            let py_dict = PyDict::new(py);
            for (k, v) in obj {
                let py_val = json_to_pyobject(py, v);
                py_dict.set_item(k, py_val).unwrap();
            }
            py_dict.into()
        },
    }
}
