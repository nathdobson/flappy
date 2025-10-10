#![deny(unused_must_use)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_imports)]

use housing::encode_sdf::encode_model;
use patina_bambu::BambuBuilder;
use patina_bambu::model::SdfModel;
use patina_geo::aabb::Aabb;
use patina_geo::geo2::polygon2::Polygon2;
use patina_geo::geo3::cylinder::Cylinder;
use patina_mesh::mesh::Mesh;
use patina_mesh::ser::encode_file;
use patina_sdf::marching_mesh::MarchingMesh;
use patina_sdf::sdf::truncated_cone::TruncatedCone;
use patina_sdf::sdf::{AsSdf, Sdf3};
use patina_threads::{THREAD_M2, ThreadMetrics};
use patina_vec::vec2::Vec2;
use patina_vec::vec3::Vec3;
use std::f64;
use std::path::Path;
use std::time::Instant;

pub struct DrumBuilder {
    eps: f64,
    flange_radius: f64,
    flange_height: f64,
    height: f64,
    outer_radius: f64,
    inner_radius: f64,
    guide_inner: f64,
    guide_outer: f64,
    guide_z_min: f64,
    guide_z_max: f64,
    guide_off_max: f64,
    guide_off_min: f64,
    letter_count: usize,
    flap_pos_radius: f64,
    flap_hole_radius: f64,
    mount_inner: f64,
    mount_outer: f64,
    magnet_ring_radius: f64,
    screw_y: f64,
    screw_x: f64,
    mount_width: f64,
    mount_rise: f64,
    seam_cut_depth: f64,
    seam_cut_width: f64,
}

impl DrumBuilder {
    pub fn build_sdf(&self) -> SdfModel {
        let mut model = SdfModel::new();
        model.add_sdf(
            &Cylinder::new(
                Vec3::zero(),
                Vec3::axis_z() * self.flange_height,
                self.flange_radius,
            )
            .as_sdf(),
        );
        model.add_sdf(
            &Cylinder::new(
                Vec3::zero(),
                Vec3::axis_z() * self.height,
                self.outer_radius,
            )
            .as_sdf(),
        );
        model.subtract_sdf(
            &Cylinder::new(
                Vec3::zero(),
                Vec3::axis_z() * self.height,
                self.inner_radius,
            )
            .as_sdf(),
        );

        model.add_sdf(&self.guide(Vec3::axis_x(), Vec3::axis_y()));
        model.add_sdf(&self.guide(-Vec3::axis_x(), -Vec3::axis_y()));
        model.add_sdf(&self.guide(Vec3::axis_y(), -Vec3::axis_x()));
        model.add_sdf(&self.guide(-Vec3::axis_y(), Vec3::axis_x()));
        for side in [-1.0, 1.0] {
            model.add_sdf(
                &Polygon2::new(vec![
                    Vec2::new(side * -self.mount_outer, self.guide_z_min + self.mount_rise),
                    Vec2::new(side * -self.mount_outer, self.guide_z_max),
                    Vec2::new(side * -self.mount_inner, self.guide_z_max),
                    Vec2::new(
                        side * -self.mount_inner,
                        self.guide_z_min + self.mount_inner - self.mount_outer + self.mount_rise,
                    ),
                ])
                .as_sdf()
                .extrude(
                    Vec3::new(
                        side * (self.inner_radius + 0.01),
                        self.mount_width + self.guide_off_min,
                        0.0,
                    ),
                    Vec3::axis_x(),
                    Vec3::axis_z(),
                    self.mount_width,
                ),
            );
            model.drill_ruthex(
                Vec3::new(side * self.screw_x, self.screw_y, self.guide_z_max),
                -Vec3::axis_z(),
                &THREAD_M2,
            );
        }
        model.subtract_sdf(
            &Cylinder::new(
                Vec3::zero(),
                Vec3::axis_z() * self.height,
                self.magnet_ring_radius,
            )
            .as_sdf(),
        );

        for i in 0..self.letter_count {
            let pos =
                Vec2::from_rad(2.0 * f64::consts::PI * (i as f64) / (self.letter_count as f64))
                    * self.flap_pos_radius;
            model.subtract_sdf(
                &Cylinder::new(
                    Vec3::new(pos.x(), pos.y(), 0.0),
                    Vec3::axis_z() * self.flange_height,
                    self.flap_hole_radius,
                )
                .as_sdf(),
            )
        }
        model.subtract_sdf(
            &Polygon2::new(vec![
                Vec2::from_rad(f64::consts::PI / 4.0) * (self.inner_radius + self.seam_cut_depth),
                Vec2::from_rad(
                    f64::consts::PI / 4.0 - self.seam_cut_width / (2.0 * self.inner_radius),
                ) * self.inner_radius,
                Vec2::from_rad(
                    f64::consts::PI / 4.0 + self.seam_cut_width / (2.0 * self.inner_radius),
                ) * self.inner_radius,
            ])
            .as_sdf()
            .extrude_z(0.0..self.height),
        );
        model
    }
    fn guide(&self, axis1: Vec3, axis2: Vec3) -> Sdf3 {
        let guide_poly = Polygon2::new(vec![
            Vec2::new(-self.guide_outer, self.guide_z_min),
            Vec2::new(-self.guide_outer, self.guide_z_max),
            Vec2::new(-self.guide_inner, self.guide_z_max),
            Vec2::new(
                -self.guide_inner,
                self.guide_z_min + self.guide_inner - self.guide_outer,
            ),
        ])
        .as_sdf();
        guide_poly
            .extrude(
                axis1 * self.outer_radius - axis2 * self.guide_off_min,
                axis1,
                Vec3::axis_z(),
                self.guide_off_max - self.guide_off_min,
            )
            .union(&guide_poly.extrude(
                axis1 * self.outer_radius + axis2 * self.guide_off_max,
                axis1,
                Vec3::axis_z(),
                self.guide_off_max - self.guide_off_min,
            ))
    }
    pub async fn build(&self) -> anyhow::Result<()> {
        let mut bambu = BambuBuilder::new();
        bambu.elefant_foot_compensation(0.1);
        encode_model(
            "outer",
            self.build_sdf(),
            bambu,
            &Aabb::new(
                Vec3::new(
                    -self.flange_radius - self.eps,
                    -self.flange_radius - self.eps,
                    -self.eps,
                ),
                Vec3::new(
                    self.flange_radius + self.eps,
                    self.flange_radius + self.eps,
                    self.height + self.eps,
                ),
            ),
        )
        .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    DrumBuilder {
        eps: 0.5,
        flange_radius: 69.0 / 2.0,
        flange_height: 1.6,
        height: 41.6,
        outer_radius: 58.5 / 2.0,
        inner_radius: 55.4 / 2.0,
        guide_inner: 3.5,
        guide_outer: 0.5,
        guide_z_min: 21.0,
        guide_z_max: 35.0,
        guide_off_max: 3.5,
        guide_off_min: 0.6,
        letter_count: 45,
        flap_hole_radius: 2.0 / 2.0,
        flap_pos_radius: 64.0 / 2.0,
        mount_inner: 10.0,
        mount_outer: 0.5,
        magnet_ring_radius: 22.2,
        screw_x: 25.0,
        screw_y: 4.0,
        mount_width: 7.0,
        mount_rise: 5.0,
        seam_cut_depth: 0.3,
        seam_cut_width: 2.0,
    }
    .build()
    .await?;
    Ok(())
}
