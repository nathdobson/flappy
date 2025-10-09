#![feature(exit_status_error)]
#![deny(unused_must_use)]
#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(unused_variables)]

mod flap_model;

use crate::flap_model::StackBuilder;
use anyhow::anyhow;
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
use patina_3mf::model_settings::{
    Assemble, AssembleItem,  ModelSettings,

};
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
use std::collections::HashSet;
use std::env;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;
use zip::write::{FileOptions, SimpleFileOptions};
use zip::{ZipArchive, ZipWriter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    build_output().await?;
    Ok(())
}

async fn build_output() -> anyhow::Result<()> {
    let font = Font::try_from_vec(fs::read("fonts/ComicMono.ttf").await?)
        .ok_or_else(|| anyhow!("bad font"))?;

    tokio::fs::create_dir_all("flaps/bodies").await?;
    tokio::fs::create_dir_all("flaps/inserts").await?;
    tokio::fs::create_dir_all("flaps/letters").await?;
    tokio::fs::create_dir_all("flaps/supports").await?;

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
            width: 43.0,
            length: 35.0,
            thickness: 1.0,
            support_thickness: 0.4,
            incut: 2.0,
            extension: 1.2,
            axle_diameter: 1.2,
            drum_diameter: 18.0,
            letter_thickness: 0.4,
            letters: " ABCDEFGHIJKLMNOPQRSTUVWXYZ$&#0123456789:.-?!"
                .chars()
                .collect(),
            font,
            flap_separation: 3.0,
            wall_separation: 0.01,
            letter_scale: 78.0,
            shift_letter: Vec2::new(0.0, -6.3),
            wedge_width: 5.0,
            wedge_height: 0.5,
            flap_grid_width: 3,
            flap_grid_height: 3,
            max_concurrent_flaps: 9,
            horizontal_gap: 2.0,
            replicas: 1,
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
        filament.filament_flow_ratio(Some(1.15));
        filament
    });
    tokio::fs::write("flaps.3mf", bambu.build()?).await?;
    // let mut command = BambuStudioCommand::new();
    // command.debug(DebugLevel::Warning);
    // command.machine(machine.clone());
    // command.process(process.clone());
    // command.add_filament(pla_basic.clone());
    // command.add_filament(pla_matte.clone());
    // command.slice(Slice::AllPlates);
    // command.export_3mf(std::path::absolute(Path::new(
    //     "examples/flap/output/sliced.3mf",
    // ))?);
    // command.input("examples/flap/output/original.3mf".into());
    // command.enable_timelapse();
    // command.timelapse_type(1);
    // command.run().await?;

    // let sliced_out = tokio::fs::read("examples/flap/output/sliced.3mf").await?;
    // let mut zip = ZipArchive::new(Cursor::new(sliced_out.as_slice()))?;
    // for i in 0..zip.len() {
    //     let mut file = zip.by_index(i)?;
    //     if file.name().ends_with("gcode") {
    //         let mut contents = vec![];
    //         file.read_to_end(&mut contents)?;
    //         let path = file.enclosed_name().ok_or_else(|| anyhow!("bad path"))?;
    //         let path = path.file_name().ok_or_else(|| anyhow!("bad filename"))?;
    //         tokio::fs::write(Path::new("examples/flap/output/").join(path), contents).await?;
    //     }
    // }
    Ok(())
}
