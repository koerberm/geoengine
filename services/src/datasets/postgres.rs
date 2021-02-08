use crate::datasets::listing::{DataSetListOptions, DataSetListing, DataSetProvider};
use crate::datasets::storage::{
    AddDataSetProvider, DataSetDB, DataSetProviderDB, DataSetProviderListOptions,
    DataSetProviderListing,
};
use crate::error::Result;
use crate::users::user::UserId;
use crate::util::user_input::Validated;
use async_trait::async_trait;
use geoengine_datatypes::dataset::{DataSetId, DataSetProviderId};
use geoengine_operators::engine::{MetaData, MetaDataProvider, ResultDescriptor};

// TODO: implement in separate PR, need placeholder here to satisfy bounds of `Context`
pub struct PostgresDataSetDB {}

impl DataSetDB for PostgresDataSetDB {}

#[async_trait]
impl DataSetProviderDB for PostgresDataSetDB {
    async fn add_data_set_provider(
        &mut self,
        _user: UserId,
        _provider: Validated<AddDataSetProvider>,
    ) -> Result<DataSetProviderId> {
        todo!()
    }

    async fn list_data_set_providers(
        &self,
        _user: UserId,
        _options: Validated<DataSetProviderListOptions>,
    ) -> Result<Vec<DataSetProviderListing>> {
        todo!()
    }

    async fn data_set_provider(
        &self,
        _user: UserId,
        _provider: DataSetProviderId,
    ) -> Result<&dyn DataSetProvider> {
        todo!()
    }
}

#[async_trait]
impl DataSetProvider for PostgresDataSetDB {
    async fn list(
        &self,
        _user: UserId,
        _options: Validated<DataSetListOptions>,
    ) -> Result<Vec<DataSetListing>> {
        todo!()
    }
}

impl<L, R> MetaDataProvider<L, R> for PostgresDataSetDB
where
    R: ResultDescriptor,
{
    fn meta_data(
        &self,
        _data_set: &DataSetId,
    ) -> std::result::Result<Box<dyn MetaData<L, R>>, geoengine_operators::error::Error> {
        todo!()
    }
}
