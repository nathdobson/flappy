use crate::settings::{SETTINGS_NOZZLE, SETTINGS_PRINTER, settings_machine, settings_primary_filament, settings_process, SETTINGS_PLATE_WIDTH, SETTINGS_PLATE_HEIGHT};
use patina_3mf::project_settings::color::Color;
use patina_3mf::settings_id::filament_settings_id::{
    FilamentBrand, FilamentMaterial, FilamentSettingsId,
};
use patina_3mf::settings_id::nozzle::Nozzle;
use patina_3mf::settings_id::print_settings_id::{PrintQuality, PrintSettingsId};
use patina_3mf::settings_id::printer::Printer;
use patina_3mf::settings_id::printer_settings_id::PrinterSettingsId;
use patina_bambu::model::{MeshModel, SdfModel};
use patina_bambu::{BambuBuilder, BambuFilament, BambuObject, BambuPlate};
use patina_geo::aabb::Aabb;
use patina_geo::geo3::aabb3::Aabb3;
use patina_mesh::decimate::Decimate;
use patina_mesh::half_edge_mesh::HalfEdgeMesh;
use patina_mesh::mesh::Mesh;
use patina_mesh::ser::encode_file;
use patina_sdf::marching_mesh::MarchingMesh;
use patina_sdf::sdf::Sdf3;
use patina_vec::mat4::Mat4;
use patina_vec::vec2::Vec2;
use patina_vec::vec3::Vec3;
use std::path::Path;
use std::time::Instant;
use tokio::fs;
use tokio::task::spawn_blocking;

pub async fn encode_files(
    start: Instant,
    prefix: &str,
    name: &str,
    model: &MeshModel,
    aabb: &Aabb3,
) -> anyhow::Result<()> {
    encode_file(&model.mesh, Path::new(&format!("{}/{}.stl", prefix, name))).await?;
    let mut bambu = BambuBuilder::new();
    bambu.add_filament(settings_primary_filament());
    bambu.printer_settings_id(Some(settings_machine()));
    bambu.print_settings_id(Some(settings_process()));
    bambu.add_plate({
        let mut plate = BambuPlate::new();
        let mut object = BambuObject::from_model(model.clone());
        object.transform(Some(
            Mat4::translate(Vec3::new(
                SETTINGS_PLATE_WIDTH / 2.0 - aabb.center().x(),
                SETTINGS_PLATE_HEIGHT / 2.0 - aabb.center().y(),
                0.0,
            ))
            .as_affine()
            .unwrap(),
        ));
        plate.add_object(object);
        plate
    });
    fs::write(&format!("{}/{}.3mf", prefix, name), bambu.build()?).await?;

    println!(
        "Built {} in {:?}",
        format!("{}/{}.*", prefix, name),
        start.elapsed()
    );
    Ok(())
}

pub async fn build_model(model: &SdfModel, marching: MarchingMesh) -> anyhow::Result<MeshModel> {
    let model = spawn_blocking({
        let model = model.clone();
        move || model.build(marching)
    })
    .await?;
    Ok(model)
}

pub async fn encode_model(name: &str, model: SdfModel, aabb: &Aabb3) -> anyhow::Result<()> {
    let draft;
    let full;
    {
        let start = Instant::now();
        draft = build_model(&model, {
            let mut marching = MarchingMesh::new(aabb);
            marching
                .min_render_depth(6)
                .max_render_depth(7)
                .subdiv_max_dot(0.9);
            marching
        })
        .await?;
        encode_files(start, "draft", name, &draft, aabb).await?;
    }
    {
        let start = Instant::now();
        full = build_model(&model, {
            let mut marching = MarchingMesh::new(aabb);
            marching
                .min_render_depth(7)
                .max_render_depth(10)
                .subdiv_max_dot(0.999);
            marching
        })
        .await?;
        encode_files(start, "full", name, &full, aabb).await?;
    }
    let start = Instant::now();
    let simplified = spawn_blocking(move || {
        let mut hem = HalfEdgeMesh::new(&full.mesh);
        let mut decimate = Decimate::new(&mut hem);
        decimate.max_degree(13);
        decimate.min_score(0.9999);
        decimate.run_arbitrary();
        hem.as_mesh()
    })
    .await?;
    encode_files(
        start,
        "simplified",
        name,
        &MeshModel {
            mesh: simplified,
            metadata: model.metadata.clone(),
        },
        aabb,
    )
    .await?;
    Ok(())
}
