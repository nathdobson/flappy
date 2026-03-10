use itertools::Itertools;
use patina_3mf::project_settings::color::Color;
use patina_3mf::settings_id::filament_settings_id::{FilamentBrand, FilamentMaterial};
use patina_bambu::{BambuObject, BambuPart, BambuPartType, BambuPlate};
use patina_extrude::ExtrusionBuilder;
use patina_font::PolygonOutlineBuilder;
use patina_geo::geo2::aabb2::Aabb2;
use patina_mesh::bimesh2::Bimesh2;
use patina_mesh::edge_mesh2::EdgeMesh2;
use patina_mesh::ser::{Encode, encode_file, encode_test_file};
use patina_vec::mat4::Mat4;
use patina_vec::vec2::Vec2;
use patina_vec::vec3::Vec3;
use rusttype::{Font, Point, Rect, Scale};
use serde::Deserialize;
use std::collections::HashMap;
use std::f64;
use std::iter::repeat_n;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Deserialize, Clone)]
pub struct MaterialSelector {
    pub color: Color,
    pub brand: FilamentBrand,
    pub material: FilamentMaterial,
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub glyphs: Vec<String>,
    pub glyph_config: GlyphConfig,
    #[serde(default)]
    pub overrides: Vec<GlyphConfig>,
    pub foreground: MaterialSelector,
    pub background: MaterialSelector,
    pub support: MaterialSelector,
}

#[derive(Deserialize, Clone)]
pub struct GlyphConfig {
    pub font: PathBuf,
    #[serde(default)]
    pub reverse_winding: bool,
    #[serde(default)]
    pub glyphs: Vec<String>,
    #[serde(default)]
    pub position: Vec2,
}

pub struct StackBuilder {
    pub working_dir: PathBuf,

    pub width: f64,
    pub length: f64,
    pub incut: f64,
    pub extension: f64,
    pub axle_diameter: f64,
    pub drum_diameter: f64,

    pub thickness: f64,
    pub support_thickness: f64,
    pub letter_thickness: f64,

    pub flap_separation: f64,
    pub wall_separation: f64,
    pub letter_scale: f32,

    pub wedge_width: f64,
    pub wedge_height: f64,
    pub flap_grid_width: usize,
    pub flap_grid_height: usize,
    pub max_concurrent_flaps: usize,
    pub horizontal_gap: f64,
    pub replicas: usize,
    pub config: Config,
    pub fonts: HashMap<PathBuf, Font<'static>>,
}

pub struct FlapParts {
    body_part: BambuPart,
    letter_part: BambuPart,
    support_part: BambuPart,
}

impl StackBuilder {
    fn blank_profile(&self) -> Vec<Vec2> {
        vec![
            Vec2::new(self.width / 2.0 - self.incut, 0.0),
            Vec2::new(self.width / 2.0 - self.incut, self.extension),
            Vec2::new(self.width / 2.0, self.extension),
            Vec2::new(self.width / 2.0, self.extension + self.axle_diameter),
            Vec2::new(
                self.width / 2.0 - self.incut,
                self.extension + self.axle_diameter,
            ),
            Vec2::new(self.width / 2.0 - self.incut, self.drum_diameter),
            Vec2::new(self.width / 2.0, self.drum_diameter),
            Vec2::new(self.width / 2.0, self.length),
        ]
    }
    fn support_poly(&self) -> EdgeMesh2 {
        // let mut profile = self.blank_profile();
        let profile = vec![
            Vec2::new(self.wedge_width, self.length - self.wedge_height),
            Vec2::new(self.wedge_width, self.length),
        ];
        let mut poly = profile.clone();
        for v in profile.iter().rev() {
            poly.push(Vec2::new(-v.x(), v.y()));
        }
        let mut mesh = EdgeMesh2::new();
        mesh.add_polygon(poly.into_iter());
        mesh
    }
    fn blank_poly(&self) -> EdgeMesh2 {
        let profile = self.blank_profile();
        let mut poly = profile.clone();
        for v in profile.iter().rev() {
            poly.push(Vec2::new(-v.x(), v.y()));
        }
        let mut mesh = EdgeMesh2::new();
        mesh.add_polygon(poly.into_iter());
        mesh
    }
    fn letter_poly(&self, index: usize) -> Arc<EdgeMesh2> {
        let glyph = &self.config.glyphs[index];
        let config = self
            .config
            .overrides
            .iter()
            .find(|o| o.glyphs.contains(&glyph))
            .unwrap_or(&self.config.glyph_config);
        let font = self.fonts.get(&config.font).expect("missing font");
        let scale = Scale::uniform(self.letter_scale);
        let v_metrics = font.v_metrics(scale);
        let v_shift = (v_metrics.ascent / 2.0) as f64;
        let glyph = font.glyph(
            glyph
                .chars()
                .exactly_one()
                .expect("Multi-codepoint glyphs are not yet supported"),
        );
        println!("Glyph id = {}", glyph.id().0);
        let glyph = glyph.scaled(scale);
        let h_metrics = glyph.h_metrics();
        let h_shift = (-h_metrics.advance_width / 2.0) as f64;
        let shift = config.position + Vec2::new(h_shift, v_shift);
        let mut outline = PolygonOutlineBuilder::new(1.0);
        // let bb = glyph.exact_bounding_box().unwrap_or(Rect::default());
        // let minx = bb.min.x as f64;
        // let maxx = bb.max.x as f64;
        glyph.build_outline(&mut outline);
        let outline = outline.build();
        let mut outline_mesh = EdgeMesh2::new();
        for outline in outline {
            let mut polygon: Vec<_> = outline.points().iter().map(|p| *p + shift).collect();
            if config.reverse_winding {
                polygon.reverse();
            }
            outline_mesh.add_polygon(polygon.into_iter());
        }
        Arc::new(outline_mesh)
    }
    fn letter_split(&self, letter: Arc<EdgeMesh2>) -> [EdgeMesh2; 2] {
        [false, true].map(|side| {
            let mut sub = EdgeMesh2::new();
            let minx = -self.width / 2.0 + self.incut + self.wall_separation;
            let maxx = self.width / 2.0 - self.incut - self.wall_separation;
            let miny;
            let maxy;
            if side {
                miny = -self.length + self.wall_separation - self.flap_separation / 2.0;
                maxy = -self.wall_separation - self.flap_separation / 2.0;
            } else {
                miny = self.wall_separation + self.flap_separation / 2.0;
                maxy = self.length - self.wall_separation + self.flap_separation / 2.0;
            }
            sub.add_polygon(
                vec![
                    Vec2::new(minx, miny),
                    Vec2::new(maxx, miny),
                    Vec2::new(maxx, maxy),
                    Vec2::new(minx, maxy),
                ]
                .into_iter(),
            );
            let bimesh = Bimesh2::new(letter.clone(), Arc::new(sub));
            let result = bimesh.intersection();
            if side {
                result.map_vertices(|v| Vec2::new(-v.x(), -v.y() - self.flap_separation / 2.0))
            } else {
                result
                    .map_vertices(|v| Vec2::new(-v.x(), v.y() - self.flap_separation / 2.0))
                    .invert_edges()
            }
        })
    }
    async fn render_svg(&self, index: usize, blank: &EdgeMesh2, split: &[EdgeMesh2; 2]) {
        let mut mixed = EdgeMesh2::new();
        mixed.add_mesh(
            &blank.map_vertices(|v| Vec2::new(v.x(), v.y() + self.flap_separation / 2.0)),
            false,
        );
        mixed.add_mesh(
            &blank.map_vertices(|v| Vec2::new(v.x(), -v.y() - self.flap_separation / 2.0)),
            true,
        );
        let mut background = mixed.clone();
        mixed.add_mesh(
            &split[0].map_vertices(|v| Vec2::new(-v.x(), v.y() + self.flap_separation / 2.0)),
            false,
        );
        mixed.add_mesh(
            &split[1].map_vertices(|v| Vec2::new(-v.x(), -v.y() - self.flap_separation / 2.0)),
            true,
        );

        encode_file(
            &mixed,
            &self
                .working_dir
                .join(format!("flaps/letters/letter_{}.svg", index)),
        )
        .await
        .unwrap();
        encode_file(
            &Preview {
                foreground: mixed,
                background,
            },
            &self
                .working_dir
                .join(format!("flaps/previews/preview_{}.svg", index)),
        )
        .await
        .unwrap();
    }
    async fn support_part(&self, index: usize, support: &EdgeMesh2) -> BambuPart {
        let mut ext = ExtrusionBuilder::new();
        let p1 = ext.add_plane(0.0, true);
        let p2 = ext.add_plane(self.support_thickness, false);
        ext.add_prism(&support, (p1, false), (p2, false));
        let mesh = ext.build();
        if let Err(e) = mesh.check_manifold() {
            eprintln!("support_part {:?}", e);
        }
        encode_file(
            &mesh,
            &self
                .working_dir
                .join(format!("flaps/supports/support_{}.stl", index)),
        )
        .await
        .unwrap();
        let mut body = BambuPart::new(mesh);
        body.material(Some(3));
        body.name(Some(format!("part({})", index)));
        body.typ(BambuPartType::SupportBlocker);
        body
    }
    async fn body_part(
        &self,
        index: usize,
        blank: &EdgeMesh2,
        letter1: &EdgeMesh2,
        letter2: &EdgeMesh2,
    ) -> BambuPart {
        let start = Instant::now();
        let mut ext = ExtrusionBuilder::new();
        let p1 = ext.add_plane(0.0, true);
        let p2 = ext.add_plane(self.letter_thickness, true);
        let p3 = ext.add_plane(self.thickness - self.letter_thickness, false);
        let p4 = ext.add_plane(self.thickness, false);
        ext.add_prism(&blank, (p1, false), (p4, false));
        ext.add_prism(&letter1, (p2, false), (p1, true));
        ext.add_prism(&letter2, (p4, true), (p3, false));
        let mesh = ext.build();
        if let Err(e) = mesh.check_manifold() {
            eprintln!("body_part {:?}", e);
        }
        println!("Built mesh in {}", start.elapsed().as_secs_f64());
        encode_file(
            &mesh,
            &self
                .working_dir
                .join(format!("flaps/bodies/body_{}.stl", index)),
        )
        .await
        .unwrap();
        let mut body = BambuPart::new(mesh);
        body.material(Some(2));
        body.name(Some(format!("part({})", index)));
        body
    }
    async fn letter_part(
        &self,
        index: usize,
        blank: &EdgeMesh2,
        letter1: &EdgeMesh2,
        letter2: &EdgeMesh2,
    ) -> BambuPart {
        let start = Instant::now();
        let mut ext = ExtrusionBuilder::new();
        let p1 = ext.add_plane(0.0, true);
        let p2 = ext.add_plane(self.letter_thickness, false);
        let p3 = ext.add_plane(self.thickness - self.letter_thickness, true);
        let p4 = ext.add_plane(self.thickness, false);
        ext.add_prism(&letter1, (p1, false), (p2, false));
        ext.add_prism(&letter2, (p3, false), (p4, false));
        let mesh = ext.build();
        if let Err(e) = mesh.check_manifold() {
            eprintln!("letter_part {:?}", e);
        }
        println!("Built mesh in {}", start.elapsed().as_secs_f64());
        encode_file(
            &mesh,
            &self
                .working_dir
                .join(format!("flaps/inserts/insert_{}.stl", index)),
        )
        .await
        .unwrap();
        let mut body = BambuPart::new(mesh);
        body.material(Some(1));
        body.name(Some(format!("part({})", index)));
        body
    }
    pub async fn flap_parts(
        &self,
        index: usize,
        blank: &EdgeMesh2,
        support: &EdgeMesh2,
        letter1: &EdgeMesh2,
        letter2: &EdgeMesh2,
    ) -> FlapParts {
        FlapParts {
            body_part: self.body_part(index, &blank, &letter1, &letter2).await,
            letter_part: self.letter_part(index, &blank, &letter1, &letter2).await,
            support_part: self.support_part(index, &support).await,
        }
    }
    fn flap_parts_at_position(
        &self,
        flap: &FlapParts,
        x_index: usize,
        y_index: usize,
        z_index: usize,
        w: usize,
        h: usize,
    ) -> Vec<BambuPart> {
        let mut result = vec![];
        let x =
            90.0 + (x_index as f64 + 0.5 - (w as f64) / 2.0) * (self.width + self.horizontal_gap);
        let y = 90.0
            + 21.0
            + (y_index as f64 + 0.5 - (h as f64) / 2.0) * (self.length + self.horizontal_gap);
        let transform_flap = (Mat4::translate(Vec3::new(
            x,
            y,
            (z_index as f64) * (self.thickness + self.support_thickness),
        )) * Mat4::translate(Vec3::new(0.0, -self.length / 2.0, 0.0)))
        .as_affine()
        .unwrap();
        let transform_support = (Mat4::translate(Vec3::new(
            x,
            y,
            (z_index as f64) * (self.thickness + self.support_thickness)
                + self.thickness
                + self.support_thickness / 2.0,
        )) * Mat4::translate(Vec3::new(0.0, -self.length / 2.0, 0.0)))
        .as_affine()
        .unwrap();
        let mut body_part = flap.body_part.clone();
        body_part.transform(Some(transform_flap));
        result.push(body_part);
        let mut letter_part = flap.letter_part.clone();
        letter_part.transform(Some(transform_flap));
        result.push(letter_part);
        let mut support_part = flap.support_part.clone();
        support_part.transform(Some(transform_support));
        result.push(support_part);
        result
    }
    pub async fn build(&self) -> HashMap<PathBuf, BambuPlate> {
        let blank = self.blank_poly();
        let support = self.support_poly();
        let mut letters = vec![];
        for index in 0..self.config.glyphs.len() {
            println!("Building letter {}", index);
            let split = self.letter_split(self.letter_poly(index));
            self.render_svg(index, &blank, &split).await;
            letters.push(split);
        }
        let mut parts = vec![];
        for index in 0..self.config.glyphs.len() {
            println!("Building part {}", index);
            parts.push(
                self.flap_parts(
                    index,
                    &blank,
                    &support,
                    &letters[index][1],
                    &letters[(index + 1) % letters.len()][0],
                )
                .await,
            );
        }
        let stacks: Vec<Vec<usize>> = (0..self.config.glyphs.len())
            .chunks(self.config.glyphs.len() / self.max_concurrent_flaps)
            .into_iter()
            .map(|x| {
                repeat_n(x.collect::<Vec<_>>().into_iter(), self.replicas)
                    .flatten()
                    .collect()
            })
            .collect();
        let mut plates = HashMap::new();
        for (index, flap) in parts.iter().enumerate() {
            let mut  plate = BambuPlate::new();
            let mut object = BambuObject::new();
            for part in self.flap_parts_at_position(flap, 0, 0, 0, 1, 1) {
                object.add_part(part);
            }
            plate.add_object(object);
            plates.insert(
                self.working_dir
                    .join(format!("flaps/singles/flap_{}.3mf", index)),
                plate,
            );
        }
        for begin in 0..stacks[0].len() {
            for end in begin + 1..stacks[0].len() + 1 {
                let mut plate = BambuPlate::new();
                for (x_index, row) in stacks
                    .iter()
                    .chunks(self.flap_grid_width)
                    .into_iter()
                    .enumerate()
                {
                    for (y_index, stack) in row.enumerate() {
                        let mut object = BambuObject::new();
                        for (z_index, &index) in stack[begin..end].iter().enumerate() {
                            for part in self.flap_parts_at_position(
                                &parts[index],
                                x_index,
                                y_index,
                                z_index,
                                self.flap_grid_width,
                                self.flap_grid_height,
                            ) {
                                object.add_part(part);
                            }
                        }
                        plate.add_object(object);
                    }
                }
                let output_path;
                if begin == 0 && end == stacks[0].len() {
                    output_path = self.working_dir.join("flaps.3mf".to_string());
                } else {
                    output_path = self.working_dir.join(format!(
                        "flaps/layers/flaps-layer{}-through-layer{}.3mf",
                        begin, end
                    ));
                }
                plates.insert(output_path, plate);
            }
        }
        plates
    }
}

struct Preview {
    foreground: EdgeMesh2,
    background: EdgeMesh2,
}

impl Encode for Preview {
    fn extension() -> &'static str {
        "svg"
    }

    fn encode<W: Unpin + Send + AsyncWrite>(
        &self,
        w: &mut W,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move {
            let polys = self.foreground.as_polygons();
            let aabb = self
                .foreground
                .vertices()
                .iter()
                .cloned()
                .collect::<Aabb2>();
            w.write_all(
                format!(
                    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\">\n",
                    aabb.min().x(),
                    aabb.min().y(),
                    aabb.dimensions().x(),
                    aabb.dimensions().y()
                )
                .as_bytes(),
            )
            .await?;
            w.write_all(
                format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#666666\"/>",
                    aabb.min().x(),
                    aabb.min().y(),
                    aabb.dimensions().x(),
                    aabb.dimensions().y()
                )
                .as_bytes(),
            )
            .await?;
            w.write_all("<path d=\"".as_bytes()).await?;
            for poly in self.background.as_polygons() {
                w.write_all("M ".as_bytes()).await?;
                for point in poly.points() {
                    w.write_all(format!("{},{} ", point.x(), point.y()).as_bytes())
                        .await?;
                }
                w.write_all("z ".as_bytes()).await?;
            }
            w.write_all("\" fill=\"#FFFFFF\" fill-rule=\"evenodd\" />\n".as_bytes())
                .await?;
            w.write_all("<path d=\"".as_bytes()).await?;
            for poly in polys {
                w.write_all("M ".as_bytes()).await?;
                for point in poly.points() {
                    w.write_all(format!("{},{} ", point.x(), point.y()).as_bytes())
                        .await?;
                }
                w.write_all("z ".as_bytes()).await?;
            }
            w.write_all("\" fill=\"#000000\" fill-rule=\"evenodd\" />\n".as_bytes())
                .await?;
            w.write_all("</svg>\n".as_bytes()).await?;

            Ok(())
        }
    }
}
