use std::{
    convert::TryInto,
    path::{Path, PathBuf},
};

use gdal::{raster::GDALDataType, Dataset, DatasetOptions};
use geoengine_datatypes::{
    dataset::{DatasetId, InternalDatasetId},
    hashmap,
    primitives::{Measurement, TimeGranularity, TimeInstance, TimeStep},
    raster::RasterDataType,
    spatial_reference::SpatialReference,
    util::Identifier,
};
use snafu::ResultExt;

use crate::{
    engine::{MockExecutionContext, RasterResultDescriptor},
    error::{self, Error},
    source::{
        FileNotFoundHandling, GdalDatasetGeoTransform, GdalDatasetParameters, GdalMetaDataRegular,
        GdalSourceTimePlaceholder, TimeReference,
    },
    test_data,
    util::Result,
};

// TODO: move test helper somewhere else?
#[allow(clippy::missing_panics_doc)]
pub fn create_ndvi_meta_data() -> GdalMetaDataRegular {
    let no_data_value = Some(0.); // TODO: is it really 0?
    GdalMetaDataRegular {
        start: TimeInstance::from_millis(1_388_534_400_000).unwrap(),
        step: TimeStep {
            granularity: TimeGranularity::Months,
            step: 1,
        },
        time_placeholders: hashmap! {
            "%_START_TIME_%".to_string() => GdalSourceTimePlaceholder {
                format: "%Y-%m-%d".to_string(),
                reference: TimeReference::Start,
            },
        },
        params: GdalDatasetParameters {
            file_path: test_data!("raster/modis_ndvi/MOD13A2_M_NDVI_%_START_TIME_%.TIFF").into(),
            rasterband_channel: 1,
            geo_transform: GdalDatasetGeoTransform {
                origin_coordinate: (-180., 90.).into(),
                x_pixel_size: 0.1,
                y_pixel_size: -0.1,
            },
            width: 3600,
            height: 1800,
            file_not_found_handling: FileNotFoundHandling::NoData,
            no_data_value,
            properties_mapping: None,
            gdal_open_options: None,
            gdal_config_options: None,
        },
        result_descriptor: RasterResultDescriptor {
            data_type: RasterDataType::U8,
            spatial_reference: SpatialReference::epsg_4326().into(),
            measurement: Measurement::Unitless,
            no_data_value,
        },
    }
}

// TODO: move test helper somewhere else?
pub fn add_ndvi_dataset(ctx: &mut MockExecutionContext) -> DatasetId {
    let id: DatasetId = InternalDatasetId::new().into();
    ctx.add_meta_data(id.clone(), Box::new(create_ndvi_meta_data()));
    id
}

/// Opens a Gdal Dataset with the given `path`.
/// Other crates should use this method for Gdal Dataset access as a workaround to avoid strange errors.
pub fn gdal_open_dataset(path: &Path) -> Result<Dataset> {
    gdal_open_dataset_ex(path, DatasetOptions::default())
}

/// Opens a Gdal Dataset with the given `path` and `dataset_options`.
/// Other crates should use this method for Gdal Dataset access as a workaround to avoid strange errors.
pub fn gdal_open_dataset_ex(path: &Path, dataset_options: DatasetOptions) -> Result<Dataset> {
    #[cfg(debug_assertions)]
    let dataset_options = {
        let mut dataset_options = dataset_options;
        dataset_options.open_flags |= gdal::GdalOpenFlags::GDAL_OF_VERBOSE_ERROR;
        dataset_options
    };

    Dataset::open_ex(path, dataset_options).context(error::Gdal)
}

/// Create a `RasterResultDescriptor` for the given `band` and `dataset`. If the raster data type is
/// unknown, the default is F64 unless it is otherwise specified by `default_data_type`. If the data
/// type is a complex floating point type, an error is returned
pub fn raster_descriptor_from_dataset(
    dataset: &Dataset,
    band: isize,
    default_data_type: Option<RasterDataType>,
) -> Result<RasterResultDescriptor> {
    let rasterband = &dataset.rasterband(band)?;

    let spatial_ref: SpatialReference =
        dataset.spatial_ref()?.try_into().context(error::DataType)?;

    let data_type = match rasterband.band_type() {
        GDALDataType::GDT_Byte => RasterDataType::U8,
        GDALDataType::GDT_UInt16 => RasterDataType::U16,
        GDALDataType::GDT_Int16 => RasterDataType::I16,
        GDALDataType::GDT_UInt32 => RasterDataType::U32,
        GDALDataType::GDT_Int32 => RasterDataType::I32,
        GDALDataType::GDT_Float32 => RasterDataType::F32,
        GDALDataType::GDT_Float64 => RasterDataType::F64,
        GDALDataType::GDT_Unknown => default_data_type.unwrap_or(RasterDataType::F64),
        _ => return Err(Error::GdalRasterDataTypeNotSupported),
    };

    Ok(RasterResultDescriptor {
        data_type,
        spatial_reference: spatial_ref.into(),
        measurement: Measurement::Unitless,
        no_data_value: rasterband.no_data_value(),
    })
}

/// Create `GdalDatasetParameters` from the infos in the given `dataset` and its `band`.
/// `path` is the location of the actual data, `band_out` allows optionally specifying a different
/// band in the resulting parameters, otherwise `band` is used.
pub fn gdal_parameters_from_dataset(
    dataset: &Dataset,
    band: usize,
    path: &Path,
    band_out: Option<usize>,
    open_options: Option<Vec<String>>,
) -> Result<GdalDatasetParameters> {
    let rasterband = &dataset.rasterband(band as isize)?;

    Ok(GdalDatasetParameters {
        file_path: PathBuf::from(path),
        rasterband_channel: band_out.unwrap_or(band),
        geo_transform: dataset.geo_transform().context(error::Gdal)?.into(),
        file_not_found_handling: FileNotFoundHandling::Error,
        no_data_value: rasterband.no_data_value(),
        properties_mapping: None,
        width: rasterband.x_size(),
        height: rasterband.y_size(),
        gdal_open_options: open_options,
        gdal_config_options: None,
    })
}
