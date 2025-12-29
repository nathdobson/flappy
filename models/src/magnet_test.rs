use models::encode_sdf::encode_model;
use patina_bambu::BambuBuilder;
use patina_bambu::model::SdfModel;
use patina_geo::aabb::Aabb;
use patina_geo::geo3::cylinder::Cylinder;
use patina_sdf::sdf::{AsSdf, Sdf3};
use patina_threads::THREAD_M2;
use patina_vec::vec3::Vec3;

struct MagnetTestBuilder {
    outer_radius: f64,
    height: f64,
    magnet_radius: f64,
    magnet_height: f64,
    magnet_fit_radius: f64,
    magnet_fit_height: f64,
}

impl MagnetTestBuilder {
    fn build_cylinder(&self) -> Sdf3 {
        Cylinder::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::axis_z() * self.height,
            self.outer_radius,
        )
        .as_sdf()
        .difference(
            &Cylinder::new(
                Vec3::new(0.0, 0.0, self.height),
                -Vec3::axis_z() * self.magnet_height,
                self.magnet_radius - self.magnet_fit_radius,
            )
            .as_sdf(),
        )
        .difference(
            &Cylinder::new(
                Vec3::new(0.0, 0.0, self.height - self.magnet_fit_height),
                -Vec3::axis_z() * (self.magnet_height - self.magnet_fit_height),
                self.magnet_radius,
            )
            .as_sdf(),
        )
    }
    fn build_sdf(&self) -> SdfModel {
        let mut result = SdfModel::new();
        result.add_sdf(&self.build_cylinder());
        result
    }
    pub async fn build(&self) -> anyhow::Result<()> {
        encode_model(
            "magnet_test",
            self.build_sdf(),
            BambuBuilder::new(),
            &Aabb::new(
                Vec3::new(-self.outer_radius, -self.outer_radius, 0.0),
                Vec3::new(self.outer_radius, self.outer_radius, self.height),
            ),
        )
        .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    MagnetTestBuilder {
        outer_radius: 4.0,
        height: 3.0,
        magnet_radius: 3.02,
        magnet_height: 2.2,
        magnet_fit_radius: 0.04,
        magnet_fit_height: 0.2,
    }
    .build()
    .await?;
    Ok(())
}
