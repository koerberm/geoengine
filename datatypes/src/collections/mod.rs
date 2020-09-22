pub(self) mod error;
mod feature_collection;
#[macro_use]
mod geo_feature_collection;
#[macro_use]
mod data_types;
mod batch_builder;
mod feature_collection_builder;

mod data_collection;
mod multi_line_string_collection;
mod multi_point_collection;
mod multi_polygon_collection;

pub(crate) use error::FeatureCollectionError;
pub use feature_collection::FeatureCollection;
pub use feature_collection_builder::{
    BuilderProvider, FeatureCollectionBuilder, FeatureCollectionRowBuilder,
    GeoFeatureCollectionRowBuilder,
};
pub use geo_feature_collection::{IntoGeometryIterator, IntoGeometryOptionsIterator};

pub use data_collection::DataCollection;
pub use data_types::VectorDataType;
pub use multi_line_string_collection::MultiLineStringCollection;
pub use multi_point_collection::MultiPointCollection;
pub use multi_polygon_collection::MultiPolygonCollection;

pub use batch_builder::{FeatureCollectionBatchBuilder, GeoFromBuffers, MultiPointBuffers};
pub use data_types::TypedFeatureCollection;

/// Calls a function on a `TypedFeatureCollection` by calling it on its variant.
/// Call via `call_generic_features!(input, features => function)`.
#[macro_export]
macro_rules! call_generic_features {
    ($input_features:expr, $features:ident => $function_call:expr) => {
        call_generic_features!(
            @variants $input_features, $features => $function_call,
            Data, MultiPoint, MultiLineString, MultiPolygon
        )
    };

    (@variants $input_features:expr, $features:ident => $function_call:expr, $($variant:tt),+) => {
        match $input_features {
            $(
                $crate::collections::TypedFeatureCollection::$variant($features) => $function_call,
            )+
        }
    };
}
