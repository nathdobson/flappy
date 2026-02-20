use crate::tabs::Tabs;
use core::f64;
use models::encode_sdf::encode_model;
use patina_bambu::BambuBuilder;
use patina_bambu::model::SdfModel;
use patina_geo::aabb::Aabb;
use patina_geo::geo2::polygon2::Polygon2;
use patina_geo::geo3::aabb3::Aabb3;
use patina_sdf::sdf::AsSdf;
use patina_vec::mat4::Mat4;
use patina_vec::vec2::Vec2;
use patina_vec::vec3::Vec3;
mod tabs;

struct LeftCap {
    tabs: Tabs,
    aabb: Aabb3,
    top: f64,
    front_back_thickness: f64,
    top_bottom_thickness: f64,
    notch: f64,
    notch_size: f64,
}

impl LeftCap {
    fn build_sdf(&self) -> SdfModel {
        let mut sdf = SdfModel::new();
        sdf.add_sdf(&self.aabb.as_sdf());
        sdf.subtract_sdf(
            &Aabb::new(
                self.aabb.min()
                    + Vec3::new(self.front_back_thickness, self.top_bottom_thickness, 0.0),
                self.aabb.max()
                    - Vec3::new(
                        self.front_back_thickness,
                        self.top_bottom_thickness,
                        self.top,
                    ),
            )
            .as_sdf(),
        );
        self.tabs.tabs(&mut sdf, true, None);
        sdf.subtract_sdf(
            &Polygon2::new(vec![
                Vec2::new(self.aabb.min().x(), self.notch - self.notch_size),
                Vec2::new(self.aabb.min().x() + self.notch_size, self.notch),
                Vec2::new(self.aabb.min().x(), self.notch + self.notch_size),
            ])
            .as_sdf()
            .extrude_y(self.aabb.min().y()..self.aabb.max().y()),
        );
        sdf
    }
    pub async fn build(&self) -> anyhow::Result<()> {
        encode_model(
            "right-cap",
            self.build_sdf(),
            BambuBuilder::new(),
            &[],
            Mat4::translate(Vec3::axis_z() * self.aabb.max().z())
                * Mat4::rotate(Vec3::axis_x(), f64::consts::PI),
            &Aabb::new(
                self.aabb.min() - Vec3::splat(0.1),
                self.aabb.max() + Vec3::splat(0.1) + Vec3::new(0.0, 0.0, self.tabs.size),
            ),
        )
        .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tabs = Tabs::new();
    LeftCap {
        tabs: Tabs::new(),
        aabb: Aabb3::new(
            Vec3::new(tabs.aabb.min().x(), tabs.aabb.min().y(), 0.0),
            Vec3::new(tabs.aabb.max().x(), tabs.aabb.max().y(), 39.0),
        ),
        top: 2.0,
        front_back_thickness: 2.0,
        top_bottom_thickness: 15.0,
        notch: 4.0,
        notch_size: 0.5,
    }
    .build()
    .await?;
    Ok(())
}
