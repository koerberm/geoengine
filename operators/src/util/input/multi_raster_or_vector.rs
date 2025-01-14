use crate::engine::{OperatorDatasets, RasterOperator, VectorOperator};
use geoengine_datatypes::dataset::DatasetId;
use serde::{Deserialize, Serialize};

/// It is either a set of `RasterOperator` or a single `VectorOperator`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MultiRasterOrVectorOperator {
    Raster(Vec<Box<dyn RasterOperator>>),
    Vector(Box<dyn VectorOperator>),
}

impl MultiRasterOrVectorOperator {
    pub fn is_raster(&self) -> bool {
        match self {
            Self::Raster(_) => true,
            Self::Vector(_) => false,
        }
    }

    pub fn is_vector(&self) -> bool {
        match self {
            Self::Raster(_) => false,
            Self::Vector(_) => true,
        }
    }
}

impl From<Box<dyn RasterOperator>> for MultiRasterOrVectorOperator {
    fn from(operator: Box<dyn RasterOperator>) -> Self {
        Self::Raster(vec![operator])
    }
}

impl From<Vec<Box<dyn RasterOperator>>> for MultiRasterOrVectorOperator {
    fn from(operators: Vec<Box<dyn RasterOperator>>) -> Self {
        Self::Raster(operators)
    }
}

impl From<Box<dyn VectorOperator>> for MultiRasterOrVectorOperator {
    fn from(operator: Box<dyn VectorOperator>) -> Self {
        Self::Vector(operator)
    }
}

impl OperatorDatasets for MultiRasterOrVectorOperator {
    fn datasets_collect(&self, datasets: &mut Vec<DatasetId>) {
        match self {
            Self::Raster(rs) => {
                for r in rs {
                    r.datasets_collect(datasets);
                }
            }
            Self::Vector(v) => v.datasets_collect(datasets),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::source::{GdalSource, GdalSourceParameters};
    use geoengine_datatypes::dataset::InternalDatasetId;
    use std::str::FromStr;

    use super::*;

    #[test]
    fn it_serializes() {
        let operator = MultiRasterOrVectorOperator::Raster(vec![GdalSource {
            params: GdalSourceParameters {
                dataset: InternalDatasetId::from_str("fc734022-61e0-49da-b327-257ba9d602a7")
                    .unwrap()
                    .into(),
            },
        }
        .boxed()]);

        assert_eq!(
            serde_json::to_value(&operator).unwrap(),
            serde_json::json!([{
                "type": "GdalSource",
                "params": {
                    "dataset": {
                        "type": "internal",
                        "datasetId": "fc734022-61e0-49da-b327-257ba9d602a7"
                    }
                }
            }])
        );
    }

    #[test]
    fn it_deserializes_raster_ops() {
        let workflow = serde_json::json!([{
            "type": "GdalSource",
            "params": {
                "dataset": {
                    "type": "internal",
                    "datasetId":  "fc734022-61e0-49da-b327-257ba9d602a7"
                }
            }
        }])
        .to_string();

        let raster_or_vector_operator: MultiRasterOrVectorOperator =
            serde_json::from_str(&workflow).unwrap();

        assert!(raster_or_vector_operator.is_raster());
        assert!(!raster_or_vector_operator.is_vector());
    }

    #[test]
    fn it_deserializes_vector_ops() {
        let workflow = serde_json::json!({
            "type": "OgrSource",
            "params": {
                "dataset": {
                    "type": "internal",
                    "datasetId":  "fc734022-61e0-49da-b327-257ba9d602a7"
                },
                "attribute_projection": null,
            }
        })
        .to_string();

        let raster_or_vector_operator: MultiRasterOrVectorOperator =
            serde_json::from_str(&workflow).unwrap();

        assert!(raster_or_vector_operator.is_vector());
        assert!(!raster_or_vector_operator.is_raster());
    }
}
