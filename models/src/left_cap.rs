use crate::tabs::Tabs;
use models::encode_sdf::EncodeBuilder;
use patina_bambu::model::SdfModel;
use patina_geo::aabb::Aabb;
use patina_geo::geo3::aabb3::Aabb3;
use patina_geo::geo3::cylinder::Cylinder;
use patina_sdf::sdf::AsSdf;
use patina_threads::{THREAD_M2, ThreadMetrics};
use patina_vec::vec2::Vec2;
use patina_vec::vec3::Vec3;

mod tabs;

struct LeftCap {
    bottom: f64,
    standoff: f64,
    tabs: Tabs,
    aabb: Aabb3,
    mount_dimensions: Vec2,
    mount_threads: &'static ThreadMetrics,
}

impl LeftCap {
    fn build_sdf(&self) -> SdfModel {
        let mut sdf = SdfModel::new();
        sdf.add_sdf(&self.aabb.as_sdf());
        sdf.subtract_sdf(
            &Aabb::new(
                self.aabb.min() + Vec3::new(self.tabs.thickness, self.tabs.thickness, self.bottom),
                self.aabb.max() - Vec3::new(-self.tabs.thickness, self.tabs.thickness, -1.0),
            )
            .as_sdf(),
        );
        let center = self.tabs.aabb.center();
        for x in [-0.5, 0.5] {
            for y in [-0.5, 0.5] {
                let pos = center
                    + Vec2::new(x * self.mount_dimensions.x(), y * self.mount_dimensions.y());
                sdf.add_sdf(
                    &Cylinder::new(
                        Vec3::new(pos.x(), pos.y(), 0.0),
                        Vec3::axis_z() * (self.bottom + self.standoff),
                        self.mount_threads.ruthex_width + self.mount_threads.ruthex_radius,
                    )
                    .as_sdf(),
                );
                sdf.drill_ruthex(
                    Vec3::new(pos.x(), pos.y(), self.bottom + self.standoff),
                    -Vec3::axis_z(),
                    self.mount_threads,
                );
            }
        }
        self.tabs.tabs(&mut sdf, false, Some(self.aabb.max().z()));
        sdf
    }
    pub async fn build(&self) -> anyhow::Result<()> {
        let builder = EncodeBuilder::new(
            "left-cap",
            self.build_sdf(),
            &Aabb::new(
                self.aabb.min() - Vec3::splat(0.1),
                self.aabb.max() + Vec3::splat(0.1) + Vec3::new(0.0, 0.0, self.tabs.size),
            ),
        );
        builder.build().await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tabs = Tabs::new();
    LeftCap {
        bottom: 2.0,
        standoff: 4.0,
        tabs: Tabs::new(),
        aabb: Aabb3::new(
            Vec3::new(tabs.aabb.min().x(), tabs.aabb.min().y(), 0.0),
            Vec3::new(tabs.aabb.max().x(), tabs.aabb.max().y(), 35.0),
        ),
        mount_dimensions: Vec2::new(44.525, 82.690),
        mount_threads: &THREAD_M2,
    }
    .build()
    .await?;
    Ok(())
}
