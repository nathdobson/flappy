use patina_bambu::model::SdfModel;
use patina_geo::aabb::Aabb;
use patina_geo::geo2::polygon2::Polygon2;
use patina_geo::geo3::cylinder::Cylinder;
use patina_sdf::sdf::AsSdf;
use patina_threads::THREAD_M3;
use patina_vec::vec2::Vec2;
use patina_vec::vec3::Vec3;

pub struct Tabs {
    pub size: f64,
    pub thickness: f64,
    pub wall_size: f64,
    pub tab_fitment: f64,
    pub housing_fitment: f64,
    pub through_hole_excess_radius: f64,
    pub toe_width: f64,
    pub aabb: Aabb<2>,
    pub notch: f64,
    pub notch_size: f64,
}

impl Tabs {
    pub fn new() -> Self {
        Tabs {
            size: 14.0,
            thickness: 5.0,
            wall_size: 6.0,
            tab_fitment: 0.2,
            housing_fitment: 0.25,
            through_hole_excess_radius: 0.25,
            toe_width: 2.001,
            aabb: Aabb::new(Vec2::new(-35.0, -66.0), Vec2::new(54.0, 62.0)),
            notch: 4.0,
            notch_size: 0.5,
        }
    }
    pub fn tabs(&self, sdf: &mut SdfModel, slot: bool, tab_z: Option<f64>) {
        let offset = self.size + self.tab_fitment + self.toe_width;
        self.tab(
            sdf,
            Vec2::new(self.aabb.min().x() + offset + 10.0, self.aabb.min().y()),
            Vec3::axis_y(),
            slot,
            tab_z,
        );
        self.tab(
            sdf,
            Vec2::new(self.aabb.min().x() + offset, self.aabb.max().y()),
            -Vec3::axis_y(),
            slot,
            tab_z,
        );
        self.tab(
            sdf,
            Vec2::new(self.aabb.max().x() - offset, self.aabb.min().y()),
            Vec3::axis_y(),
            slot,
            tab_z,
        );
        self.tab(
            sdf,
            Vec2::new(self.aabb.max().x() - offset, self.aabb.max().y()),
            -Vec3::axis_y(),
            slot,
            tab_z,
        );
    }
    pub fn tab(
        &self,
        sdf: &mut SdfModel,
        origin: Vec2,
        axis: Vec3,
        slot: bool,
        tab_z: Option<f64>,
    ) {
        let axis2 = Vec3::axis_z();
        let axis1 = -axis.cross(axis2);
        if let Some(tab_z) = tab_z {
            sdf.add_sdf(
                &Polygon2::new(vec![
                    Vec2::new(-self.size, 0.0),
                    Vec2::new(self.size, 0.0),
                    Vec2::new(0.0, self.size),
                ])
                .as_sdf()
                .extrude(
                    Vec3::new(origin.x(), origin.y(), tab_z),
                    axis1,
                    axis2,
                    self.thickness,
                ),
            );
            sdf.subtract_sdf(
                &Cylinder::new(
                    Vec3::new(origin.x(), origin.y(), tab_z + self.wall_size),
                    axis * self.thickness * 2.0,
                    THREAD_M3.through_radius + self.through_hole_excess_radius,
                )
                .as_sdf(),
            );
            sdf.subtract_sdf(
                &Cylinder::new(
                    Vec3::new(origin.x(), origin.y(), tab_z + self.wall_size),
                    axis * THREAD_M3.countersink_depth,
                    THREAD_M3.countersink_radius,
                )
                .as_sdf(),
            );
        }
        if slot {
            sdf.subtract_sdf(
                &Polygon2::new(vec![
                    Vec2::new(-self.size - self.tab_fitment, 0.0),
                    Vec2::new(self.size + self.tab_fitment, 0.0),
                    Vec2::new(0.0, self.size + self.tab_fitment),
                ])
                .as_sdf()
                .extrude(
                    Vec3::new(origin.x(), origin.y(), 0.0),
                    axis1,
                    axis2,
                    self.thickness,
                ),
            );
            sdf.drill_ruthex(
                Vec3::new(
                    origin.x(),
                    origin.y(),
                    self.wall_size - self.housing_fitment,
                ) + axis * self.thickness,
                axis,
                &THREAD_M3,
            );
            sdf.subtract_sdf(
                &Polygon2::new(vec![
                    Vec2::new(self.aabb.min().x(), self.notch - self.notch_size),
                    Vec2::new(self.aabb.min().x() + self.notch_size, self.notch),
                    Vec2::new(self.aabb.min().x(), self.notch + self.notch_size),
                ])
                .as_sdf()
                .extrude_y(self.aabb.min().y()..self.aabb.max().y()),
            );
        }
    }
}
