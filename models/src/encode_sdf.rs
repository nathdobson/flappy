use patina_geo::aabb::Aabb;
use patina_geo::geo3::aabb3::Aabb3;
use patina_mesh::decimate::Decimate;
use patina_mesh::half_edge_mesh::HalfEdgeMesh;
use patina_mesh::ser::encode_file;
use patina_sdf::marching_mesh::MarchingMesh;
use patina_sdf::sdf::Sdf3;
use patina_vec::vec3::Vec3;
use std::path::Path;
use std::time::Instant;
use tokio::task::spawn_blocking;

pub async fn encode_sdf(name: &str, sdf: Sdf3, aabb: Aabb3) -> anyhow::Result<()> {
    let start = Instant::now();
    let draft = spawn_blocking({
        let sdf = sdf.clone();
        move || {
            let mut marching = MarchingMesh::new(aabb);
            marching
                .min_render_depth(6)
                .max_render_depth(7)
                .subdiv_max_dot(0.9);
            marching.build(&sdf)
        }
    })
    .await?;
    println!("manifold = {:?}", draft.check_manifold());
    let path = format!("draft/{}.stl", name);
    encode_file(&draft, Path::new(&path)).await?;
    println!("Built {} in {:?}", path, start.elapsed());
    let start = Instant::now();
    let full = spawn_blocking({
        let sdf = sdf.clone();
        move || {
            let mut marching = MarchingMesh::new(aabb);
            marching
                .min_render_depth(7)
                .max_render_depth(10)
                .subdiv_max_dot(0.999);
            marching.build(&sdf)
        }
    })
    .await?;
    println!("manifold = {:?}", full.check_manifold());
    let path = format!("full/{}.stl", name);
    encode_file(&full, Path::new(&path)).await?;
    println!("Built {} in {:?}", path, start.elapsed());
    let start = Instant::now();
    let simplified = spawn_blocking(move || {
        let mut hem = HalfEdgeMesh::new(&full);
        let mut decimate = Decimate::new(&mut hem);
        decimate.max_degree(13);
        decimate.min_score(0.9999);
        decimate.run_arbitrary();
        hem.as_mesh()
    })
    .await?;
    let path = format!("simplified/{}.stl", name);
    encode_file(&simplified, Path::new(&path)).await?;
    println!("Built {} in {:?}", path, start.elapsed());
    Ok(())
}
