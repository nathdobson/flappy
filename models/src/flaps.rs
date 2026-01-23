#![feature(exit_status_error)]
#![deny(unused_must_use)]
#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(unused_variables)]

mod flap_model;

use crate::flap_model::{Config, StackBuilder};
use anyhow::{Context, anyhow};
use clap::Parser;
use itertools::Itertools;
use patina_3mf::ModelContainer;
use patina_3mf::content_types::{ContentTypeDefault, ContentTypes};
use patina_3mf::model::build::{ModelBuild, ModelItem};
use patina_3mf::model::mesh::{
    ModelMesh, ModelTriangle, ModelTriangles, ModelVertex, ModelVertices,
};
use patina_3mf::model::resources::{
    ModelComponent, ModelComponents, ModelObject, ModelObjectType, ModelResources,
};
use patina_3mf::model::{Model, ModelMetadata, ModelUnit};
use patina_3mf::model_settings::{Assemble, AssembleItem, ModelSettings};
use patina_3mf::project_settings::color::Color;
use patina_3mf::project_settings::support_interface_pattern::SupportInterfacePattern;
use patina_3mf::project_settings::support_style::SupportStyle;
use patina_3mf::project_settings::support_type::SupportType;
use patina_3mf::relationships::{Relationship, Relationships};
use patina_3mf::settings_id::filament_settings_id::{
    FilamentBrand, FilamentMaterial, FilamentSettingsId,
};
use patina_3mf::settings_id::nozzle::Nozzle;
use patina_3mf::settings_id::print_settings_id::{PrintQuality, PrintSettingsId};
use patina_3mf::settings_id::printer::Printer;
use patina_3mf::settings_id::printer_settings_id::PrinterSettingsId;
use patina_bambu::cli::{BambuStudioCommand, DebugLevel, Slice};
use patina_bambu::{BambuBuilder, BambuFilament, BambuObject, BambuPart, BambuPlate, BambuSupport};
use patina_extrude::ExtrusionBuilder;
use patina_font::PolygonOutlineBuilder;
use patina_geo::aabb::Aabb;
use patina_geo::geo2::polygon2::Polygon2;
use patina_mesh::bimesh2::Bimesh2;
use patina_mesh::edge_mesh2::EdgeMesh2;
use patina_mesh::ser::encode_file;
use patina_sdf::marching_mesh::MarchingMesh;
use patina_sdf::sdf::Sdf;
use patina_sdf::sdf::leaf::SdfLeafImpl;
use patina_vec::mat4::Mat4;
use patina_vec::vec2::Vec2;
use patina_vec::vec3::Vec3;
use rand::rng;
use rusttype::{Font, OutlineBuilder, Point, Rect, Scale};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use std::{env, iter};
use tokio::fs;
use zip::write::{FileOptions, SimpleFileOptions};
use zip::{ZipArchive, ZipWriter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    build_output().await?;
    Ok(())
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    config: PathBuf,
}

async fn build_output() -> anyhow::Result<()> {
    let args = Args::parse();
    let config_path = tokio::fs::canonicalize(&args.config)
        .await
        .with_context(|| format!("could not resolve {}", args.config.display()))?;
    let working_dir: PathBuf = args
        .config
        .parent()
        .ok_or_else(|| anyhow!("could not resolve directory containing config"))?
        .to_path_buf();
    let config = tokio::fs::read(&config_path)
        .await
        .with_context(|| format!("could not read file {}", config_path.display()))?;
    let config: Config = serde_json::from_slice(&config)
        .with_context(|| format!("could not parse file {}", config_path.display()))?;

    let mut fonts = HashMap::new();
    for config in iter::once(&config.glyph_config).chain(&config.overrides) {
        if !fonts.contains_key(&config.font) {
            let font = working_dir.join(&config.font);
            let font = fs::read(&font)
                .await
                .with_context(|| format!("could not read font {}", font.display()))?;
            let font = Font::try_from_vec(font).ok_or_else(|| anyhow!("bad font"))?;
            fonts.insert(config.font.clone(), font);
        }
    }

    tokio::fs::create_dir_all(working_dir.join("flaps/bodies")).await?;
    tokio::fs::create_dir_all(working_dir.join("flaps/inserts")).await?;
    tokio::fs::create_dir_all(working_dir.join("flaps/letters")).await?;
    tokio::fs::create_dir_all(working_dir.join("flaps/previews")).await?;
    tokio::fs::create_dir_all(working_dir.join("flaps/supports")).await?;

    let mut bambu = BambuBuilder::new();
    let printer = Printer::A1Mini;
    let nozzle = Nozzle::Nozzle0_4;
    let mut machine = PrinterSettingsId::new(printer.clone());
    machine.nozzle = Some(nozzle.clone());
    let process = PrintSettingsId::new(0.2, PrintQuality::Standard, printer.clone(), nozzle);
    let pla_basic = FilamentSettingsId::new(
        FilamentBrand::Bambu,
        FilamentMaterial::PlaBasic,
        printer.clone(),
    );
    let pla_matte = FilamentSettingsId::new(
        FilamentBrand::Bambu,
        FilamentMaterial::PlaMatte,
        printer.clone(),
    );
    let pla_support = FilamentSettingsId::new(
        FilamentBrand::Bambu,
        FilamentMaterial::SupportForPla,
        printer.clone(),
    );

    bambu.printer_settings_id(Some(machine.clone()));
    bambu.print_settings_id(Some(process.clone()));
    bambu.prime_tower_positions(Some(vec![Vec2::new(15.0, 15.0)]));
    bambu.support({
        let mut support = BambuSupport::new();
        support.independent_support_layer_height(0);
        support.support_bottom_z_distance(0);
        support.support_filament(3);
        support.support_interface_filament(3);
        support.support_interface_pattern(SupportInterfacePattern::Concentric);
        support.support_interface_spacing(0);
        support.support_style(SupportStyle::Snug);
        support.support_top_z_distance(0);
        support.support_type(SupportType::NormalAuto);
        support.support_expansion(-0.25);
        support
    });
    bambu.add_plate({
        let mut plate = BambuPlate::new();
        let mut object = BambuObject::new();
        object.name(Some("stack".to_string()));
        StackBuilder {
            working_dir: working_dir.clone(),
            width: 43.0,
            length: 35.0,
            thickness: 1.0,
            support_thickness: 0.4,
            incut: 2.0,
            extension: 1.2,
            axle_diameter: 1.2,
            drum_diameter: 18.0,
            letter_thickness: 0.4,
            flap_separation: 3.01,
            wall_separation: 0.01,
            letter_scale: 78.0,
            wedge_width: 5.0,
            wedge_height: 0.5,
            flap_grid_width: 3,
            flap_grid_height: 3,
            max_concurrent_flaps: 9,
            horizontal_gap: 2.0,
            replicas: 1,
            config,
            fonts,
        }
        .build()
        .await
    });
    bambu.add_filament({
        let mut filament = BambuFilament::new();
        filament.color(Some(Color::new(255, 255, 255)));
        filament.support(Some(false));
        filament.settings_id(Some(pla_matte.clone()));
        filament.diameter(Some(1.75));
        filament.shrink(Some("100%".to_string()));
        filament
    });
    bambu.add_filament({
        let mut filament = BambuFilament::new();
        filament.color(Some(Color::new(90, 68, 177)));
        filament.support(Some(false));
        filament.settings_id(Some(pla_basic.clone()));
        filament.diameter(Some(1.75));
        filament.shrink(Some("100%".to_string()));
        filament
    });
    bambu.add_filament({
        let mut filament = BambuFilament::new();
        filament.color(Some(Color::new(255, 255, 255)));
        filament.support(Some(true));
        filament.settings_id(Some(pla_support.clone()));
        filament.diameter(Some(1.75));
        filament.shrink(Some("100%".to_string()));
        filament.filament_flow_ratio(Some(1.00));
        filament
    });
    tokio::fs::write(&working_dir.join("flaps.3mf"), bambu.build()?).await?;

    Ok(())
}
