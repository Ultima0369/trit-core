//! Public data sources — climate, ecology, science, geopolitics.
//!
//! Each source implements the [`DataSource`](crate::source::DataSource) trait
//! and is registered in the [`SourceRegistry`](crate::registry::SourceRegistry).

pub mod arxiv;
pub mod gbif;
pub mod gibs;
pub mod nasa_power;
pub mod noaa_co2;
pub mod noaa_tides;
pub mod nsidc;
pub mod open_meteo;
pub mod ucdp;
pub mod usgs;
