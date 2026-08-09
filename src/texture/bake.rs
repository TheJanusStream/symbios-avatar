//! Rasterising a body into texture space.
//!
//! Painting a body procedurally needs the inverse of rendering: for each texel
//! of the atlas, *where on the body is this?* Rasterising the unwrapped charts
//! answers that once, producing a geometry buffer of position, normal, and zone
//! per texel. Every painter after that is a pure function of one texel's sample,
//! which is what makes procedural skin tractable — freckles on a cheek become
//! plain arithmetic on a position rather than a search through geometry.
//!
//! Charts do not tile the atlas, so texels between them start empty. They are
//! **dilated** outward afterwards, because a bilinear tap or a mip level near a
//! chart edge otherwise pulls in background and draws a dark seam around every
//! body part.

use glam::{Vec2, Vec3};

use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::uv::UvUnwrap;

/// What the body looks like at one texel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Texel {
    /// Position on the body, in body space.
    pub position: Vec3,
    /// Smooth surface normal there.
    pub normal: Vec3,
    /// How deeply the surface folds back on itself here, `0` on open skin and
    /// rising to `1` in a cavity. Carried from the geometry rather than derived
    /// by the painter, because the mesh is the only thing that knows: see
    /// [`PolyMesh::crease`].
    pub crease: f32,
    /// Whether this texel is inside the mouth (#154): `0` on skin, near `0.5`
    /// on the teeth ridge, `1` in the cavity. Carried per vertex from the
    /// surgery's own classes, because nothing derivable from position can
    /// tell the inside of a closed fold from the lip a millimetre outside it.
    pub mouth: f32,
    /// Which part of the body it belongs to.
    pub zone: Zone,
}

/// The body, rasterised into texture space.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AtlasGeometry {
    /// Atlas width in texels.
    pub width: u32,
    /// Atlas height in texels.
    pub height: u32,
    /// One entry per texel, row-major from the bottom-left. `None` where no
    /// chart covers the texel even after dilation.
    pub texels: Vec<Option<Texel>>,
}

impl AtlasGeometry {
    /// The sample at `(x, y)`, if the body covers it.
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> Option<Texel> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.texels[(y * self.width + x) as usize]
    }

    /// How many texels the body covers.
    #[must_use]
    pub fn covered(&self) -> usize {
        self.texels.iter().filter(|texel| texel.is_some()).count()
    }

    /// Fraction of the atlas the body covers.
    #[must_use]
    pub fn coverage(&self) -> f32 {
        if self.texels.is_empty() {
            return 0.0;
        }
        self.covered() as f32 / self.texels.len() as f32
    }
}

/// How far, in texels, chart edges are grown outward.
///
/// Enough to survive a few mip levels; more than this and neighbouring charts
/// start growing into each other's gutters.
pub const DILATION: u32 = 8;

/// Rasterises `uv`'s charts into a geometry buffer of `size` texels square.
///
/// The body only. Use [`bake`] for an avatar that also carries attached parts,
/// which is every avatar with a face.
#[must_use]
pub fn bake_geometry(mesh: &PolyMesh, uv: &UvUnwrap, size: u32) -> AtlasGeometry {
    bake(mesh, uv, &[], &[], size)
}

/// Rasterises a body **and its attached parts** into one geometry buffer.
///
/// Each part is a mesh already carrying body-space positions and atlas
/// coordinates — that is, one whose reserved region from
/// [`crate::uv::unwrap_with`] has already been applied. Baking them into the
/// same buffer is what lets one painter cover the whole avatar: a nose is
/// painted by the same complexion arithmetic as the cheek beside it, because by
/// the time the painter runs there is no difference between them.
///
/// Dilation happens once, at the end, over everything. Doing it per part would
/// let one part's grown edge overwrite a neighbour's genuine texels.
#[must_use]
pub fn bake(
    mesh: &PolyMesh,
    uv: &UvUnwrap,
    parts: &[(&PolyMesh, Zone)],
    mouth: &[f32],
    size: u32,
) -> AtlasGeometry {
    if size == 0 || (uv.faces.is_empty() && parts.is_empty()) {
        return AtlasGeometry::default();
    }

    let mut geometry = AtlasGeometry {
        width: size,
        height: size,
        texels: vec![None; (size * size) as usize],
    };

    if !uv.faces.is_empty() {
        let normals = uv.gather(&mesh.vertex_normals());
        let positions = uv.gather(&mesh.positions);
        // Measured on the body before it was split at its seams, so a crease
        // that runs across a chart boundary is the same depth on both sides.
        let creases = uv.gather(&mesh.crease());
        let mouths = if mouth.len() == mesh.vertex_count() {
            uv.gather(mouth)
        } else {
            vec![0.0; uv.uvs.len()]
        };
        for (index, face) in uv.faces.iter().enumerate() {
            let zone = uv.charts[uv.chart_of_face[index] as usize].zone;
            // Fan-triangulate: a quad from a subdivided body is planar enough
            // that which diagonal is chosen makes no visible difference.
            for corner in 1..face.len().saturating_sub(1) {
                let triangle = [face[0], face[corner], face[corner + 1]];
                rasterize(
                    &mut geometry,
                    zone,
                    triangle.map(|v| uv.uvs[v as usize]),
                    triangle.map(|v| positions[v as usize]),
                    triangle.map(|v| normals[v as usize]),
                    triangle.map(|v| creases[v as usize]),
                    triangle.map(|v| mouths[v as usize]),
                );
            }
        }
    }

    for &(part, zone) in parts {
        draw_part(&mut geometry, part, zone);
    }

    dilate(&mut geometry, DILATION);
    geometry
}

/// Rasterises one attached part, using the coordinates it carries.
fn draw_part(geometry: &mut AtlasGeometry, part: &PolyMesh, zone: Zone) {
    if part.uvs.len() != part.positions.len() {
        return;
    }
    let normals = part.shading_normals();
    let creases = part.crease();
    for face in &part.faces {
        for corner in 1..face.len().saturating_sub(1) {
            let triangle = [face[0], face[corner], face[corner + 1]];
            rasterize(
                geometry,
                zone,
                triangle.map(|v| part.uvs[v as usize]),
                triangle.map(|v| part.positions[v as usize]),
                triangle.map(|v| normals[v as usize]),
                triangle.map(|v| creases[v as usize]),
                [0.0; 3],
            );
        }
    }
}

/// Fills every texel a triangle covers.
fn rasterize(
    geometry: &mut AtlasGeometry,
    zone: Zone,
    uvs: [Vec2; 3],
    positions: [Vec3; 3],
    normals: [Vec3; 3],
    creases: [f32; 3],
    mouths: [f32; 3],
) {
    let scale = Vec2::new(geometry.width as f32, geometry.height as f32);
    let pixels = uvs.map(|uv| uv * scale);

    let area = edge(pixels[0], pixels[1], pixels[2]);
    if area.abs() < 1e-9 {
        return;
    }

    let lo = pixels[0].min(pixels[1]).min(pixels[2]);
    let hi = pixels[0].max(pixels[1]).max(pixels[2]);
    let x0 = lo.x.floor().max(0.0) as u32;
    let y0 = lo.y.floor().max(0.0) as u32;
    let x1 = (hi.x.ceil() as i64).clamp(0, i64::from(geometry.width)) as u32;
    let y1 = (hi.y.ceil() as i64).clamp(0, i64::from(geometry.height)) as u32;

    for y in y0..y1 {
        for x in x0..x1 {
            let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let mut weights = Vec3::new(
                edge(pixels[1], pixels[2], point),
                edge(pixels[2], pixels[0], point),
                edge(pixels[0], pixels[1], point),
            ) / area;

            // A small tolerance closes the hairline cracks that appear along
            // shared edges when a sample lands exactly on the boundary.
            if weights.min_element() < -1e-4 {
                continue;
            }
            weights = weights.max(Vec3::ZERO);
            let total = weights.element_sum();
            if total <= 0.0 {
                continue;
            }
            weights /= total;

            geometry.texels[(y * geometry.width + x) as usize] = Some(Texel {
                position: positions[0] * weights.x
                    + positions[1] * weights.y
                    + positions[2] * weights.z,
                normal: (normals[0] * weights.x + normals[1] * weights.y + normals[2] * weights.z)
                    .normalize_or(Vec3::Y),
                crease: creases[0] * weights.x + creases[1] * weights.y + creases[2] * weights.z,
                mouth: mouths[0] * weights.x + mouths[1] * weights.y + mouths[2] * weights.z,
                zone,
            });
        }
    }
}

/// Twice the signed area of the triangle `a, b, c`.
fn edge(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Grows filled texels outward into the gutters.
fn dilate(geometry: &mut AtlasGeometry, passes: u32) {
    let (width, height) = (geometry.width as i64, geometry.height as i64);

    for _ in 0..passes {
        let previous = geometry.texels.clone();
        let mut grew = false;

        for y in 0..height {
            for x in 0..width {
                let here = (y * width + x) as usize;
                if previous[here].is_some() {
                    continue;
                }
                // Take the first filled neighbour rather than blending: this is
                // padding for filtering, and an averaged value between two body
                // parts belongs to neither.
                let found = [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)]
                    .into_iter()
                    .find_map(|(dx, dy)| {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= width || ny >= height {
                            return None;
                        }
                        previous[(ny * width + nx) as usize]
                    });
                if let Some(texel) = found {
                    geometry.texels[here] = Some(texel);
                    grew = true;
                }
            }
        }

        if !grew {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cage::{CageConfig, build_cage};
    use crate::plan::{BodyPlan, HumanoidParams};
    use crate::rig::{Rig, SkinConfig, skin};
    use crate::subdiv::catmull_clark;
    use crate::uv::{UvConfig, unwrap};

    fn baked(size: u32) -> (PolyMesh, UvUnwrap, AtlasGeometry) {
        let skeleton = HumanoidParams::default().skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);
        let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());
        let geometry = bake_geometry(&mesh, &uv, size);
        (mesh, uv, geometry)
    }

    #[test]
    fn the_body_covers_most_of_the_atlas() {
        let (_, _, geometry) = baked(256);
        assert_eq!(geometry.width, 256);
        // Coverage tracks how tightly the charts packed — around 60-80% — since
        // the body fills its charts and the charts fill the atlas.
        assert!(
            geometry.coverage() > 0.5,
            "only {:.0}% of the atlas has a body under it",
            geometry.coverage() * 100.0
        );
    }

    #[test]
    fn every_sample_lands_on_the_body() {
        let (mesh, _, geometry) = baked(128);
        let (lo, hi) = mesh.bounds();
        let slack = (hi - lo) * 0.05;

        for texel in geometry.texels.iter().flatten() {
            assert!(
                texel.position.cmpge(lo - slack).all() && texel.position.cmple(hi + slack).all(),
                "sample at {:?} is off the body",
                texel.position
            );
            assert!(
                (texel.normal.length() - 1.0).abs() < 1e-3,
                "normal is not unit length"
            );
        }
    }

    #[test]
    fn zones_survive_into_texture_space() {
        let (_, uv, geometry) = baked(256);
        let charted: Vec<Zone> = uv.charts.iter().map(|chart| chart.zone).collect();

        for zone in &charted {
            assert!(
                geometry
                    .texels
                    .iter()
                    .flatten()
                    .any(|texel| texel.zone == *zone),
                "{zone:?} has a chart but no texels"
            );
        }
    }

    #[test]
    fn dilation_fills_the_gutters() {
        let (mesh, uv, _) = baked(128);
        let mut bare = AtlasGeometry {
            width: 128,
            height: 128,
            texels: vec![None; 128 * 128],
        };
        let normals = uv.gather(&mesh.vertex_normals());
        let positions = uv.gather(&mesh.positions);
        for (index, face) in uv.faces.iter().enumerate() {
            let zone = uv.charts[uv.chart_of_face[index] as usize].zone;
            for corner in 1..face.len().saturating_sub(1) {
                let triangle = [face[0], face[corner], face[corner + 1]];
                rasterize(
                    &mut bare,
                    zone,
                    triangle.map(|v| uv.uvs[v as usize]),
                    triangle.map(|v| positions[v as usize]),
                    triangle.map(|v| normals[v as usize]),
                    [0.0; 3],
                    [0.0; 3],
                );
            }
        }
        let before = bare.coverage();
        dilate(&mut bare, DILATION);
        assert!(
            bare.coverage() > before,
            "dilation must grow coverage past {before:.3}"
        );
    }

    #[test]
    fn baking_is_deterministic() {
        let (_, _, first) = baked(64);
        let (_, _, second) = baked(64);
        assert_eq!(first, second);
    }

    #[test]
    fn a_zero_sized_atlas_bakes_to_nothing() {
        let (mesh, uv, _) = baked(32);
        assert_eq!(bake_geometry(&mesh, &uv, 0), AtlasGeometry::default());
    }
}
