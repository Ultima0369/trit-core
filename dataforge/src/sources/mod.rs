//! Public data sources — climate, ecology, science, geopolitics.
//!
//! Each source implements the [`DataSource`](crate::source::DataSource) trait
//! and is registered in the [`SourceRegistry`](crate::registry::SourceRegistry).

pub mod arxiv;
pub mod gbif;
pub mod noaa_co2;
pub mod open_meteo;
pub mod ucdp;
