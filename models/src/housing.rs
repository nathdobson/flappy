#![deny(unused_must_use)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_imports)]

use anyhow::Context;
use housing::encode_sdf::encode_model;
use patina_bambu::BambuBuilder;
use patina_bambu::model::SdfModel;
use patina_geo::aabb::Aabb;
use patina_geo::geo2::polygon2::Polygon2;
use patina_geo::geo2::triangle2::Triangle2;
use patina_geo::geo3::aabb3::Aabb3;
use patina_geo::geo3::cylinder::Cylinder;
use patina_geo::geo3::plane::Plane;
use patina_geo::geo3::triangle3::Triangle3;
use patina_geo::sphere::Circle;
use patina_mesh::decimate::Decimate;
use patina_mesh::half_edge_mesh::HalfEdgeMesh;
use patina_mesh::mesh::Mesh;
use patina_mesh::ser::encode_file;
use patina_sdf::marching_mesh::MarchingMesh;
use patina_sdf::sdf::leaf::SdfLeafImpl;
use patina_sdf::sdf::truncated_cone::TruncatedCone;
use patina_sdf::sdf::{AsSdf, Sdf, Sdf3};
use patina_threads::{THREAD_M2, THREAD_M3, ThreadMetrics};
use patina_vec::vec2::Vec2;
use patina_vec::vec3::Vec3;
use std::f64;
use std::path::Path;
use std::time::Instant;

struct Tab {
    size: f64,
    thickness: f64,
    wall_size: f64,
    bottom_x: f64,
    top_x: f64,
    right_y: f64,
    tab_fitment: f64,
    housing_fitment: f64,
    through_hole_excess_radius: f64,
}

struct Catch {
    bottom_thickness: f64,
    indent: f64,
}

struct Mount {
    off_x: f64,
    off_y: f64,
    length: f64,
    motor_radius: f64,
    motor_fit: f64,
    rad1: f64,
    rad2: f64,
    extra_back: f64,
}

struct Brace {
    width: f64,
    extent: f64,
    indent: f64,
}

struct Port {
    start_x: f64,
    width: f64,
    length: f64,
}

struct Tube {
    width: f64,
    wall_bottom: f64,
    wall_top: f64,
    wire_inlet1: f64,
    wire_inlet2: f64,
    tab_width: f64,
}

struct HallMount {
    width: f64,
    thickness: f64,
    length: f64,

    hole1_x: f64,
    off_y: f64,
    rad1: f64,
    rad2: f64,
    tilt_deg: f64,
    extra_cone: f64,
    hole_bias: f64,
}

struct DrumGuide {
    length: f64,
    rad_inner: f64,
    rad_outer: f64,
    seam_cut_width: f64,
    seam_cut_depth: f64,
}

struct HallChannel {
    width: f64,
    length: f64,
}

struct BoardMounts {
    standoff: f64,
    thread: &'static ThreadMetrics,
    brace_width: f64,
    brace_inset: f64,
    board1_vertical: f64,
    board1_width: f64,
    board1_height1: f64,
    board1_height2: f64,

    board2_vertical: f64,
    board2_width: f64,
    board2_height: f64,
}
struct HousingBuilder {
    aabb: Aabb3,
    inf: f64,
    drum_bounding_radius: f64,
    back_thickness: f64,
    catch: Catch,
    mount: Mount,
    brace: Brace,
    port: Port,
    tab: Tab,
    tube: Tube,
    hall_mount: HallMount,
    drum_guide: DrumGuide,
    hall_channel: HallChannel,
    top_catch: TopCatch,
    board_mounts: BoardMounts,
}

impl HousingBuilder {
    fn main_body(&self) -> SdfModel {
        let mut sdf = SdfModel::new();
        sdf.add_sdf(
            &self.aabb.as_sdf().difference(
                &Cylinder::new(
                    Vec3::new(0.0, 0.0, self.back_thickness),
                    Vec3::axis_z() * self.inf,
                    self.drum_bounding_radius,
                )
                .as_sdf(),
            ),
        );
        sdf.subtract_sdf(
            &Aabb::new(
                Vec3::new(
                    -self.inf,
                    self.aabb.min().y() + self.catch.bottom_thickness,
                    self.back_thickness,
                ),
                Vec3::new(self.aabb.min().x() + self.catch.indent, 0.0, self.inf),
            )
            .as_sdf(),
        );
        sdf
    }
    fn mount(&self, sdf: &mut SdfModel, y: f64) {
        sdf.add_sdf(
            &TruncatedCone::new(
                Vec3::new(self.mount.off_x, y, self.back_thickness),
                Vec3::new(0.0, 0.0, self.mount.length),
                self.mount.rad1,
                self.mount.rad2,
            )
            .as_sdf(),
        );
        sdf.drill_ruthex(
            Vec3::new(self.mount.off_x, y, self.back_thickness + self.mount.length),
            -Vec3::axis_z(),
            &THREAD_M3,
        );
        sdf.add_sdf(
            &Cylinder::new(
                Vec3::new(self.mount.off_x, 0.0, self.back_thickness),
                Vec3::new(0.0, 0.0, self.mount.extra_back),
                self.mount.motor_radius + self.mount.motor_fit,
            )
            .as_sdf(),
        );
    }
    fn brace_x(&self, y: f64, dy: f64) -> Sdf3 {
        let mut sdf = Sdf::empty();
        sdf = sdf.union(
            &Polygon2::new(vec![
                Vec2::new(
                    y + dy * (self.mount.rad2 - self.brace.indent),
                    self.back_thickness,
                ),
                Vec2::new(
                    y + dy * (self.mount.rad2 - self.brace.indent + self.brace.extent),
                    self.back_thickness,
                ),
                Vec2::new(
                    y + dy * (self.mount.rad2 - self.brace.indent),
                    self.back_thickness + self.mount.length,
                ),
            ])
            .as_sdf()
            .extrude_x(
                self.mount.off_x - self.brace.width / 2.0
                    ..self.mount.off_x + self.brace.width / 2.0,
            ),
        );
        sdf
    }
    fn brace_y(&self, y: f64, dx: f64) -> Sdf3 {
        let mut sdf = Sdf::empty();
        sdf = sdf.union(
            &Polygon2::new(vec![
                Vec2::new(
                    self.mount.off_x + dx * (self.mount.rad2 - self.brace.indent),
                    self.back_thickness,
                ),
                Vec2::new(
                    self.mount.off_x
                        + dx * (self.mount.rad2 - self.brace.indent + self.brace.extent),
                    self.back_thickness,
                ),
                Vec2::new(
                    self.mount.off_x + dx * (self.mount.rad2 - self.brace.indent),
                    self.back_thickness + self.mount.length,
                ),
            ])
            .as_sdf()
            .extrude_y(-y - self.brace.width / 2.0..-y + self.brace.width / 2.0),
        );
        sdf
    }
    fn mounts(&self, sdf: &mut SdfModel) {
        self.mount(sdf, self.mount.off_y);
        self.mount(sdf, -self.mount.off_y);
    }
    fn wiring_pos(&self) -> Sdf3 {
        let mut sdf = Sdf::empty();
        sdf = sdf.union(
            &Polygon2::new(vec![
                Vec2::new(
                    self.tube.width / 2.0 - self.tube.wire_inlet1,
                    self.tube.wall_bottom,
                ),
                Vec2::new(
                    self.tube.width / 2.0 - self.tube.wire_inlet1 - self.tube.tab_width,
                    self.tube.wall_bottom,
                ),
                Vec2::new(
                    self.tube.width / 2.0 - self.tube.wire_inlet1 - self.tube.tab_width,
                    self.back_thickness - self.tube.wall_top - self.tube.wire_inlet2,
                ),
            ])
            .as_sdf()
            .extrude_x(self.port.start_x + self.port.width..self.aabb.max().x()),
        );
        sdf
    }
    fn wiring_neg(&self) -> Sdf3 {
        let mut sdf = Sdf::empty();
        sdf = sdf.union(
            &Aabb::new(
                Vec3::new(self.port.start_x, -self.port.length / 2.0, -self.inf),
                Vec3::new(
                    self.port.start_x + self.port.width,
                    self.port.length / 2.0,
                    self.inf,
                ),
            )
            .as_sdf(),
        );
        sdf = sdf.union(
            &Aabb::new(
                Vec3::new(
                    self.port.start_x + self.port.width,
                    -self.tube.width / 2.0,
                    self.tube.wall_bottom,
                ),
                Vec3::new(
                    self.inf,
                    self.tube.width / 2.0,
                    self.back_thickness - self.tube.wall_top,
                ),
            )
            .as_sdf(),
        );
        sdf = sdf.union(
            &Aabb::new(
                Vec3::new(
                    self.port.start_x + self.port.width,
                    self.tube.width / 2.0 - self.tube.wire_inlet1,
                    -self.inf,
                ),
                Vec3::new(
                    self.inf,
                    self.tube.width / 2.0,
                    (self.tube.wall_bottom + self.back_thickness - self.tube.wall_top) / 2.0,
                ),
            )
            .as_sdf(),
        );
        sdf = sdf.union(
            &Aabb::new(
                Vec3::new(
                    self.port.start_x - self.hall_channel.length,
                    -self.hall_channel.width / 2.0,
                    self.back_thickness,
                ),
                Vec3::new(
                    self.port.start_x,
                    self.hall_channel.width / 2.0,
                    self.back_thickness + self.mount.extra_back,
                ),
            )
            .as_sdf(),
        );
        sdf
    }
    fn tab(&self, sdf: &mut SdfModel, origin: Vec2, axis: Vec3) {
        let axis2 = Vec3::axis_z();
        let axis1 = -axis.cross(axis2);
        sdf.add_sdf(
            &Polygon2::new(vec![
                Vec2::new(-self.tab.size, 0.0),
                Vec2::new(self.tab.size, 0.0),
                Vec2::new(0.0, self.tab.size),
            ])
            .as_sdf()
            .extrude(
                Vec3::new(origin.x(), origin.y(), self.aabb.max().z()),
                axis1,
                axis2,
                self.tab.thickness,
            ),
        );
        sdf.subtract_sdf(
            &Polygon2::new(vec![
                Vec2::new(-self.tab.size - self.tab.tab_fitment, 0.0),
                Vec2::new(self.tab.size + self.tab.tab_fitment, 0.0),
                Vec2::new(0.0, self.tab.size + self.tab.tab_fitment),
            ])
            .as_sdf()
            .extrude(
                Vec3::new(origin.x(), origin.y(), 0.0),
                axis1,
                axis2,
                self.tab.thickness,
            ),
        );

        sdf.subtract_sdf(
            &Cylinder::new(
                Vec3::new(
                    origin.x(),
                    origin.y(),
                    self.aabb.max().z() + self.tab.wall_size,
                ),
                axis * self.tab.thickness * 2.0,
                THREAD_M3.through_radius + self.tab.through_hole_excess_radius,
            )
            .as_sdf(),
        );
        sdf.subtract_sdf(
            &Cylinder::new(
                Vec3::new(
                    origin.x(),
                    origin.y(),
                    self.aabb.max().z() + self.tab.wall_size,
                ),
                axis * THREAD_M3.countersink_depth,
                THREAD_M3.countersink_radius,
            )
            .as_sdf(),
        );
        sdf.drill_ruthex(
            Vec3::new(
                origin.x(),
                origin.y(),
                self.tab.wall_size - self.tab.housing_fitment,
            ) + axis * self.tab.thickness,
            axis,
            &THREAD_M3,
        );
    }
    fn hall_mount(&self, sdf: &mut SdfModel) {
        let norm = Vec2::from_deg(self.hall_mount.tilt_deg);
        let norm = Vec3::new(0.0, norm.x(), norm.y());
        sdf.add_sdf(
            &Aabb::new(
                Vec3::new(
                    self.hall_mount.hole1_x
                        - self.hall_mount.width
                        - self.hall_mount.thickness / 2.0,
                    self.hall_mount.off_y - self.hall_mount.thickness / 2.0,
                    self.back_thickness,
                ),
                Vec3::new(
                    self.hall_mount.hole1_x + self.hall_mount.thickness / 2.0,
                    self.hall_mount.off_y + self.hall_mount.thickness / 2.0,
                    self.back_thickness + self.hall_mount.length,
                ),
            )
            .as_sdf()
            .union(
                &TruncatedCone::new(
                    Vec3::new(
                        self.hall_mount.hole1_x,
                        self.hall_mount.off_y,
                        self.back_thickness,
                    ),
                    Vec3::axis_z() * (self.hall_mount.length + self.hall_mount.extra_cone),
                    self.hall_mount.rad1,
                    self.hall_mount.rad2,
                )
                .as_sdf(),
            )
            .union(
                &TruncatedCone::new(
                    Vec3::new(
                        self.hall_mount.hole1_x - self.hall_mount.width,
                        self.hall_mount.off_y,
                        self.back_thickness,
                    ),
                    Vec3::axis_z() * (self.hall_mount.length + self.hall_mount.extra_cone),
                    self.hall_mount.rad1,
                    self.hall_mount.rad2,
                )
                .as_sdf(),
            )
            .difference(
                &Plane::new(
                    Vec3::new(
                        0.0,
                        self.hall_mount.off_y + self.hall_mount.hole_bias,
                        self.back_thickness + self.hall_mount.length,
                    ),
                    -norm,
                )
                .as_sdf(),
            ),
        );

        sdf.drill_ruthex(
            Vec3::new(
                self.hall_mount.hole1_x - self.hall_mount.width,
                self.hall_mount.off_y + self.hall_mount.hole_bias,
                self.back_thickness + self.hall_mount.length,
            ),
            -norm,
            &THREAD_M2,
        );
        sdf.drill_ruthex(
            Vec3::new(
                self.hall_mount.hole1_x,
                self.hall_mount.off_y + self.hall_mount.hole_bias,
                self.back_thickness + self.hall_mount.length,
            ),
            -norm,
            &THREAD_M2,
        );
    }
    fn motor_clearance(&self) -> Sdf3 {
        Cylinder::new(
            Vec3::new(
                self.mount.off_x,
                0.0,
                self.back_thickness + self.mount.extra_back,
            ),
            Vec3::new(0.0, 0.0, self.inf),
            self.mount.motor_radius + self.mount.motor_fit,
        )
        .as_sdf()
    }
    fn drum_guide(&self) -> Sdf3 {
        let mut sdf = Sdf::empty();
        sdf = sdf.union(
            &Cylinder::new(
                Vec3::new(0.0, 0.0, self.back_thickness),
                Vec3::new(0.0, 0.0, self.drum_guide.length),
                self.drum_guide.rad_outer,
            )
            .as_sdf(),
        );
        sdf = sdf.difference(
            &Cylinder::new(
                Vec3::new(0.0, 0.0, -self.inf),
                Vec3::new(0.0, 0.0, 2.0 * self.inf),
                self.drum_guide.rad_inner,
            )
            .as_sdf(),
        );
        sdf = sdf.difference(
            &Polygon2::new(vec![
                Vec2::from_rad(f64::consts::PI / 4.0)
                    * (self.drum_guide.rad_outer - self.drum_guide.seam_cut_depth),
                Vec2::from_rad(
                    f64::consts::PI / 4.0
                        - self.drum_guide.seam_cut_width / (self.drum_guide.rad_outer),
                ) * (self.drum_guide.rad_outer + self.drum_guide.seam_cut_depth),
                Vec2::from_rad(
                    f64::consts::PI / 4.0
                        + self.drum_guide.seam_cut_width / (self.drum_guide.rad_outer),
                ) * (self.drum_guide.rad_outer + self.drum_guide.seam_cut_depth),
            ])
            .as_sdf()
            .extrude_z(self.back_thickness..self.back_thickness + self.drum_guide.length),
        );
        sdf
    }
    fn drillium(&self) -> Sdf3 {
        let mut sdf = Sdf::empty();
        sdf = sdf.union(
            &Cylinder::new(
                Vec3::new(42.0, 57.0, -self.inf),
                Vec3::axis_z() * self.inf * 2.0,
                10.0,
            )
            .as_sdf(),
        );
        sdf = sdf.union(
            &Cylinder::new(
                Vec3::new(44.0, -57.0, -self.inf),
                Vec3::axis_z() * self.inf * 2.0,
                12.0,
            )
            .as_sdf(),
        );
        sdf
    }
    fn top_catch(&self) -> Sdf3 {
        Aabb::new(
            Vec3::new(
                self.aabb.min().x(),
                self.top_catch.min_y,
                self.back_thickness,
            ),
            Vec3::new(
                self.aabb.min().x() + self.top_catch.thickness,
                self.top_catch.max_y,
                self.aabb.max().z(),
            ),
        )
        .as_sdf()
    }
    fn board_mount(&self, pos: Vec2, sdf: &mut SdfModel) {
        let origin = Vec3::new(self.aabb.max().x(), pos.y(), pos.x());
        sdf.add_sdf(
            &Cylinder::new(
                origin,
                Vec3::axis_x() * self.board_mounts.standoff,
                self.board_mounts.thread.ruthex_outer_radius(),
            )
            .as_sdf(),
        );
        sdf.drill_ruthex(
            origin + Vec3::axis_x() * self.board_mounts.standoff,
            -Vec3::axis_x(),
            self.board_mounts.thread,
        );
        sdf.add_sdf(
            &Polygon2::new(vec![
                Vec2::new(0.00001, 0.00001),
                Vec2::new(0.0000001, -self.board_mounts.standoff),
                Vec2::new(self.board_mounts.standoff, 0.000001),
            ])
            .as_sdf()
            .extrude(
                origin
                    + Vec3::new(
                        0.0,
                        self.board_mounts.brace_width / 2.0,
                        -self.board_mounts.thread.ruthex_outer_radius()
                            + self.board_mounts.brace_inset,
                    ),
                Vec3::axis_x(),
                Vec3::axis_z(),
                self.board_mounts.brace_width,
            ),
        );
    }
    fn board_mounts(&self, mut sdf: &mut SdfModel) {
        let center_x = self.aabb.center().z();
        self.board_mount(
            Vec2::new(
                center_x - self.board_mounts.board1_width / 2.0,
                self.aabb.max().y() - self.board_mounts.board1_vertical,
            ),
            sdf,
        );
        self.board_mount(
            Vec2::new(
                center_x + self.board_mounts.board1_width / 2.0,
                self.aabb.max().y() - self.board_mounts.board1_vertical,
            ),
            sdf,
        );
        self.board_mount(
            Vec2::new(
                center_x - self.board_mounts.board1_width / 2.0,
                self.aabb.max().y()
                    - self.board_mounts.board1_vertical
                    - self.board_mounts.board1_height2,
            ),
            sdf,
        );
        self.board_mount(
            Vec2::new(
                center_x + self.board_mounts.board1_width / 2.0,
                self.aabb.max().y()
                    - self.board_mounts.board1_vertical
                    - self.board_mounts.board1_height1,
            ),
            sdf,
        );
        self.board_mount(
            Vec2::new(
                center_x + self.board_mounts.board2_width / 2.0,
                self.aabb.min().y() + self.board_mounts.board2_vertical,
            ),
            sdf,
        );
        self.board_mount(
            Vec2::new(
                center_x - self.board_mounts.board2_width / 2.0,
                self.aabb.min().y() + self.board_mounts.board2_vertical,
            ),
            sdf,
        );
        self.board_mount(
            Vec2::new(
                center_x + self.board_mounts.board2_width / 2.0,
                self.aabb.min().y()
                    + self.board_mounts.board2_vertical
                    + self.board_mounts.board2_height,
            ),
            sdf,
        );
        self.board_mount(
            Vec2::new(
                center_x - self.board_mounts.board2_width / 2.0,
                self.aabb.min().y()
                    + self.board_mounts.board2_vertical
                    + self.board_mounts.board2_height,
            ),
            sdf,
        );
    }
    fn build_sdf(&self) -> SdfModel {
        let mut sdf = self.main_body();
        sdf.add_sdf(&self.top_catch());
        self.mounts(&mut sdf);
        sdf.subtract_sdf(&self.wiring_neg());
        sdf.add_sdf(&self.wiring_pos());
        self.tab(
            &mut sdf,
            Vec2::new(self.tab.bottom_x, self.aabb.min().y()),
            Vec3::axis_y(),
        );
        self.tab(
            &mut sdf,
            Vec2::new(self.tab.top_x, self.aabb.max().y()),
            -Vec3::axis_y(),
        );
        self.tab(
            &mut sdf,
            Vec2::new(self.aabb.max().x(), self.tab.right_y),
            -Vec3::axis_x(),
        );
        self.hall_mount(&mut sdf);
        sdf.subtract_sdf(&self.motor_clearance());
        sdf.add_sdf(&self.drum_guide());
        sdf.subtract_sdf(&self.drillium());
        self.board_mounts(&mut sdf);
        sdf
    }
    pub async fn build(&self) -> anyhow::Result<()> {
        let sdf = self.build_sdf();
        let aabb = Aabb::new(
            self.aabb.min() + Vec3::splat(-0.1),
            self.aabb.max() + Vec3::new(0.1 + self.board_mounts.standoff, 0.1, self.tab.size + 0.1),
        );
        encode_model("housing", sdf, BambuBuilder::new(), &aabb).await?;
        Ok(())
    }
}

struct TopCatch {
    min_y: f64,
    max_y: f64,
    thickness: f64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    HousingBuilder {
        inf: 1000.0,
        aabb: Aabb::new(Vec3::new(-35.0, -71.0, 0.0), Vec3::new(59.0, 70.0, 50.0)),

        drum_bounding_radius: 56.0,
        back_thickness: 4.0,
        tab: Tab {
            size: 14.0,
            thickness: 5.0,
            wall_size: 6.0,
            bottom_x: 20.0,
            top_x: -20.0,
            right_y: 45.0,
            tab_fitment: 0.2,
            housing_fitment: 0.35,
            through_hole_excess_radius: 0.25,
        },
        catch: Catch {
            bottom_thickness: 15.0,
            indent: 10.0,
        },
        mount: Mount {
            off_x: 8.0,
            off_y: 17.5,
            length: 22.2,
            motor_radius: 14.0,
            motor_fit: 0.05,
            rad1: 8.0,
            rad2: 5.0,
            extra_back: 4.0,
        },
        brace: Brace {
            width: 2.0,
            extent: 6.0,
            indent: 0.2,
        },
        port: Port {
            start_x: 16.0,
            width: 7.0,
            length: 16.0,
        },
        tube: Tube {
            width: 14.0,
            wall_bottom: 1.0,
            wall_top: 1.0,
            wire_inlet1: 1.2,
            wire_inlet2: 0.8,
            tab_width: 2.0,
        },
        hall_mount: HallMount {
            width: 10.0,
            thickness: 6.0,
            length: 20.0,
            hole1_x: -9.0,
            off_y: -13.0,
            rad1: 4.0,
            rad2: 4.0,
            tilt_deg: 60.0,
            extra_cone: 2.0,
            hole_bias: 2.0,
        },
        drum_guide: DrumGuide {
            length: 20.0,
            rad_inner: 53.0 / 2.0,
            rad_outer: 55.0 / 2.0,
            seam_cut_width: 2.0,
            seam_cut_depth: 0.3,
        },
        hall_channel: HallChannel {
            width: 6.0,
            length: 30.0,
        },
        top_catch: TopCatch {
            min_y: 35.0,
            max_y: 50.0,
            thickness: 2.0,
        },
        board_mounts: BoardMounts {
            standoff: 5.0,
            thread: &THREAD_M2,
            brace_width: 3.0,
            brace_inset: 0.5,

            board1_width: 29.37,
            board1_vertical: 11.5,
            board1_height1: 26.39,
            board1_height2: 27.39,

            board2_width: 31.75,
            board2_height: 44.45,
            board2_vertical: 10.0,
        },
    }
    .build()
    .await?;
    Ok(())
}
