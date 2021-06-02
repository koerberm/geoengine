use crate::engine::{
    ExecutionContext, InitializedOperator, InitializedVectorOperator, Operator, QueryContext,
    QueryProcessor, QueryRectangle, TypedVectorQueryProcessor, VectorOperator,
    VectorQueryProcessor, VectorResultDescriptor,
};
use crate::error;
use crate::util::input::StringOrNumberRange;
use crate::util::Result;
use crate::{adapters::FeatureCollectionChunkMerger, engine::SingleVectorSource};
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use geoengine_datatypes::collections::{
    FeatureCollection, FeatureCollectionInfos, FeatureCollectionModifications,
};
use geoengine_datatypes::primitives::{FeatureDataType, FeatureDataValue, Geometry};
use geoengine_datatypes::util::arrow::ArrowTyped;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::ops::RangeInclusive;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ColumnRangeFilterParams {
    pub column: String,
    pub ranges: Vec<StringOrNumberRange>,
    pub keep_nulls: bool,
}

pub type ColumnRangeFilter = Operator<ColumnRangeFilterParams, SingleVectorSource>;

#[typetag::serde]
impl VectorOperator for ColumnRangeFilter {
    fn initialize(
        self: Box<Self>,
        context: &dyn ExecutionContext,
    ) -> Result<Box<InitializedVectorOperator>> {
        let vector_source = self.sources.vector.initialize(context)?;

        let initialized_operator = InitializedColumnRangeFilter {
            result_descriptor: vector_source.result_descriptor().clone(),
            vector_source,
            state: self.params,
        };

        Ok(initialized_operator.boxed())
    }
}

pub struct InitializedColumnRangeFilter {
    result_descriptor: VectorResultDescriptor,
    vector_source: Box<InitializedVectorOperator>,
    state: ColumnRangeFilterParams,
}

impl InitializedOperator<VectorResultDescriptor, TypedVectorQueryProcessor>
    for InitializedColumnRangeFilter
{
    fn query_processor(&self) -> Result<TypedVectorQueryProcessor> {
        Ok(map_typed_vector_query_processor!(
            self.vector_source.query_processor()?,
            source => ColumnRangeFilterProcessor::new(source, self.state.clone()).boxed()
        ))
    }

    fn result_descriptor(&self) -> &VectorResultDescriptor {
        &self.result_descriptor
    }
}

pub struct ColumnRangeFilterProcessor<G> {
    vector_type: PhantomData<FeatureCollection<G>>,
    source: Box<dyn VectorQueryProcessor<VectorType = FeatureCollection<G>>>,
    column: String,
    keep_nulls: bool,
    ranges: Vec<StringOrNumberRange>,
}

impl<G> ColumnRangeFilterProcessor<G>
where
    G: Geometry + ArrowTyped + Sync + Send,
{
    pub fn new(
        source: Box<dyn VectorQueryProcessor<VectorType = FeatureCollection<G>>>,
        params: ColumnRangeFilterParams,
    ) -> Self {
        Self {
            vector_type: Default::default(),
            source,
            column: params.column,
            keep_nulls: params.keep_nulls,
            ranges: params.ranges,
        }
    }
}

#[async_trait]
impl<G> VectorQueryProcessor for ColumnRangeFilterProcessor<G>
where
    G: Geometry + ArrowTyped + Sync + Send + 'static,
{
    type VectorType = FeatureCollection<G>;

    async fn vector_query<'a>(
        &'a self,
        query: QueryRectangle,
        ctx: &'a dyn QueryContext,
    ) -> Result<BoxStream<'a, Result<Self::VectorType>>> {
        let column_name = self.column.clone();
        let ranges = self.ranges.clone();
        let keep_nulls = self.keep_nulls;

        let filter_stream = self.source.query(query, ctx).await?.map(move |collection| {
            let collection = collection?;

            // TODO: do transformation work only once
            let ranges: Result<Vec<RangeInclusive<FeatureDataValue>>> =
                match collection.column_type(&column_name)? {
                    FeatureDataType::Text => ranges
                        .iter()
                        .cloned()
                        .map(|range| range.into_string_range().map(Into::into))
                        .collect(),
                    FeatureDataType::Float => ranges
                        .iter()
                        .cloned()
                        .map(|range| range.into_float_range().map(Into::into))
                        .collect(),
                    FeatureDataType::Int => ranges
                        .iter()
                        .cloned()
                        .map(|range| range.into_int_range().map(Into::into))
                        .collect(),
                    FeatureDataType::Category => Err(error::Error::InvalidType {
                        expected: "text, float, or int".to_string(),
                        found: "category".to_string(),
                    }),
                };

            collection
                .column_range_filter(&column_name, &ranges?, keep_nulls)
                .map_err(Into::into)
        });

        let merged_chunks_stream =
            FeatureCollectionChunkMerger::new(filter_stream.fuse(), ctx.chunk_byte_size());

        Ok(merged_chunks_stream.boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{MockExecutionContext, MockQueryContext};
    use crate::mock::MockFeatureCollectionSource;
    use geoengine_datatypes::collections::{FeatureCollectionModifications, MultiPointCollection};
    use geoengine_datatypes::primitives::{
        BoundingBox2D, Coordinate2D, FeatureData, MultiPoint, SpatialResolution, TimeInterval,
    };

    #[test]
    fn serde() {
        let filter = ColumnRangeFilter {
            params: ColumnRangeFilterParams {
                column: "foobar".to_string(),
                ranges: vec![(1..=2).into()],
                keep_nulls: false,
            },
            sources: MockFeatureCollectionSource::<MultiPoint>::multiple(vec![])
                .boxed()
                .into(),
        }
        .boxed();

        let serialized = serde_json::to_string(&filter).unwrap();

        assert_eq!(
            serialized,
            serde_json::json!({
                "type": "ColumnRangeFilter",
                "params": {
                    "column": "foobar",
                    "ranges": [
                        [1, 2]
                    ],
                    "keepNulls": false
                },
                "sources": {
                    "vector": {
                        "type": "MockFeatureCollectionSourceMultiPoint",
                        "params": {
                            "collections": []
                        }
                    }
                },
            })
            .to_string()
        );

        let _operator: Box<dyn VectorOperator> = serde_json::from_str(&serialized).unwrap();
    }

    #[tokio::test]
    async fn execute() {
        let column_name = "foo";

        let collection = MultiPointCollection::from_data(
            MultiPoint::many(vec![(0.0, 0.1), (1.0, 1.1), (2.0, 2.1), (3.0, 3.1)]).unwrap(),
            vec![TimeInterval::new(0, 1).unwrap(); 4],
            [(
                column_name.to_string(),
                FeatureData::Float(vec![0., 1., 2., 3.]),
            )]
            .iter()
            .cloned()
            .collect(),
        )
        .unwrap();

        let source = MockFeatureCollectionSource::single(collection.clone()).boxed();

        let filter = ColumnRangeFilter {
            params: ColumnRangeFilterParams {
                column: column_name.to_string(),
                ranges: vec![(1..=2).into()],
                keep_nulls: false,
            },
            sources: source.into(),
        }
        .boxed();

        let initialized = filter.initialize(&MockExecutionContext::default()).unwrap();

        let point_processor = match initialized.query_processor() {
            Ok(TypedVectorQueryProcessor::MultiPoint(processor)) => processor,
            _ => panic!(),
        };

        let query_rectangle = QueryRectangle {
            bbox: BoundingBox2D::new((0., 0.).into(), (4., 4.).into()).unwrap(),
            time_interval: TimeInterval::default(),
            spatial_resolution: SpatialResolution::zero_point_one(),
        };

        let ctx = MockQueryContext::new(2 * std::mem::size_of::<Coordinate2D>());

        let stream = point_processor.vector_query(query_rectangle, &ctx).unwrap();

        let collections: Vec<MultiPointCollection> = stream.map(Result::unwrap).collect().await;

        assert_eq!(collections.len(), 1);

        assert_eq!(
            collections[0],
            collection.filter(vec![false, true, true, false]).unwrap()
        );
    }
}
