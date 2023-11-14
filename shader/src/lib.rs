#![cfg_attr(target_arch = "spirv", no_std)]

use sdf_playground_common::Params;
use spirv_std::glam::*;
#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::*;
use spirv_std::spirv;

/// Signed distance function composing the entire scene.
///
/// As all SDFs do, it returns the closest distance to any object at given
/// coordinates.
fn scene(time: f32, point: Vec3) -> f32 {
    /// Choose which scene to show:
    const SCENE: u8 = 4;

    match SCENE {
        1 => {
            // Scene 1: Just a sphere
            sdf::sphere(point, 5.0)
        }

        2 => {
            // Scene 2: Just a rectangle
            sdf::rect(point, vec3(3.0, 3.0, 3.0))
        }

        3 => {
            // Scene 3: Intersection of sphere & rectangle
            let a = sdf::sphere(
                point,
                4.0 + (time * 3.0).sin(),
            );

            let b = sdf::rect(point, vec3(3.0, 3.0, 3.0));

            sdf::intersection(a, b)
        }

        4 => {
            // Scene 4: Ocean in a sphere
            if point.length() <= 15.0 {
                let a = sdf::ocean(time, point);
                let b = sdf::sphere(point, 7.0);

                sdf::intersection(a, b)
            } else {
                // (optimization - if we're looking too far away, don't compute
                //  ocean)
                f32::MAX
            }
        }

        _ => f32::MAX,
    }
}

// -----------------------------------------------------------------------------

/// Vertex shader, generates a so-called _full-screen triangle_.
#[spirv(vertex)]
pub fn main_vs(
    #[spirv(vertex_index)] id: i32,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    // Note that there exists a much better way to do this¹, but for
    // demonstration purposes it'll do.
    //
    // ¹ https://wallisc.github.io/rendering/2021/04/18/Fullscreen-Pass.html
    *out_pos = match id {
        0 => vec4(-1.0, -1.0, 0.0, 1.0),
        1 => vec4(3.0, -1.0, 0.0, 1.0),
        _ => vec4(-1.0, 3.0, 0.0, 1.0),
    };
}

/// Fragment shader, generates color for each pixel on the screen¹.
///
/// ¹ technically for each pixel on the triangle, but since our triangle takes
///   the entire screen...
#[spirv(fragment)]
pub fn main_fs(
    #[spirv(frag_coord)] pos: Vec4,
    #[spirv(descriptor_set = 0, binding = 0, uniform)]
    params: &Params,
    out_color: &mut Vec4,
) {
    // Time elapsed since the application started, in seconds
    let time = params.time;

    // Screen position, remapped to 0..1
    let uv = pos.xy()
        / vec2(params.width as f32, params.height as f32);

    // Where the sun is located (arbitrary, can be modified)
    let sun_pos = vec3(50.0, 100.0, 50.0);

    // Where the camera is located (arbitrary, can be modified)
    let ray_origin = vec3(7.0, 4.0, 7.0);

    // Where the camera is looking towards; it varies for each pixel, simulating
    // a perspective projection
    let ray_direction =
        sdf_playground_common::direction(ray_origin, uv);

    // -----
    //
    // Having everything ready, let's perform the ray-marching!
    //
    // This function returns a 3D point of the place in the world "we see" from
    // our pixel.
    //
    // If we see nothing, `march()` will return a point that's infinitely far
    // away (which we detect below).
    //
    let hit_point = march(time, ray_origin, ray_direction);

    *out_color = if hit_point.is_finite() {
        // We hit something - let's compute normal and perform shading!
        let hit_normal = normal(time, hit_point);

        // Direction from the hit-point to our sun
        let sun_dir = (sun_pos - hit_point).normalize();

        // Cosine of the angle between the hit-point and sun - intuitively:
        //
        // - when the angle is 1.0, the surface is pointing straight at the sun:
        //
        //     sun
        //      |
        //      |
        //     hit
        //
        // - when the angle is between 0.0 and 1.0, the surface is pointing
        //   *roughly* in the direction of the sun:
        //
        //      sun
        //      /
        //     /
        //   hit
        //
        // - otherwise, the surface doesn't receive any lightning from the sun:
        //
        //   hit -- sun
        //
        // tl;dr dot product of two normal vectors is like a similarity metric
        //       of them - when it's > 0.0, the normals are pointing in a
        //       similar direction
        let sun_cosine =
            hit_normal.dot(sun_dir).clamp(0.0, 1.0);

        // Diffuse lightning - it determines the "base" color of our object
        let diffuse = vec3(0.02, 0.19, 0.58) * sun_cosine;

        // Specular lightning - it shows a nice specular highlight on the place
        // where the sun shines the most.
        //
        // Note that this is a very rough approximation - in principle, we
        // should, at the very least, compute something called a *half-vector*,
        // but ain't nobody got time for that.
        let specular =
            vec3(1.0, 1.0, 1.0) * sun_cosine.powf(50.0);

        // Now, let's simply blend both colors together.
        //
        // As before, this is kind of an approximation - in principle, we should
        // use a *tone-mapping operator* here, so that very bright colors (with
        // R,G,B above > 1.0) can be properly displayed on typical displays.
        //
        // Our current approach (of not using any tone-mapping whatsoever) is
        // alright~ish, it's just that the colors will look a bit washed out.
        (diffuse + specular).extend(1.0)
    } else {
        // We hit nothing - let's output the background color
        vec4(0.0, 0.0, 0.0, 1.0)
    };
}

/// Follows a ray from origin through direction and returns the closest surface
/// hit by that ray.
///
/// Intuitively, in two dimensions, if `*` marked the origin and `->` marked the
/// direction, given a scene such as:
///
/// ```text
/// A          B
///
/// F    * ->  C  G
///
/// E          D
/// ```
///
/// ... `march()` would return the position of `C`.
fn march(time: f32, origin: Vec3, direction: Vec3) -> Vec3 {
    const STEPS: u32 = 64;

    let mut distance = 0.0;

    for _ in 0..STEPS {
        let point = origin + direction * distance;
        let step = scene(time, point);

        if step < 0.01 {
            return point;
        }

        distance += step;

        if distance > 100.0 {
            break;
        }
    }

    Vec3::INFINITY
}

/// Returns the normal of surface at given point.
///
/// Intuitively, normal describes the orientation ("rotation") of surface at
/// given point:
///
/// ```text
///     /-----------/
///    /    ^      /
///   /     |     /   (so this boi is "looking up")
///  /           /
/// /-----------/
/// ```
///
/// Later we calculate the angle between the surface's normal and the sun, which
/// allows us to compute how bright that surface should be:
///
/// ```text
///         *  (the sun)
///
///
///     /-----------/
///    /           /|
///   /     A     / |
///  /           /  |
/// /-----------/ C |
/// |           |   /
/// |     B     |  /
/// |           | /
/// |           |/
/// \-----------/
///
/// (in this case we'd imagine that `A` is bright, while `B` and `C` are black,
///  since their normals point totally outside "of" the sun)
/// ```
fn normal(time: f32, point: Vec3) -> Vec3 {
    let d = 0.001;
    let dx = vec3(d, 0.0, 0.0);
    let dy = vec3(0.0, d, 0.0);
    let dz = vec3(0.0, 0.0, d);

    // Calculating normal is as simple taking the derivative of `scene`, but
    // since (for our purposes here) that function is closed-form, we do the
    // next best thing:
    //
    // Calculate the gradient and use it to estimate the derivative.

    let gx =
        scene(time, point + dx) - scene(time, point - dx);

    let gy =
        scene(time, point + dy) - scene(time, point - dy);

    let gz =
        scene(time, point + dz) - scene(time, point - dz);

    vec3(gx, gy, gz).normalize()
}

mod sdf {
    #![allow(unused)]

    use super::*;

    pub fn union(f1: f32, f2: f32) -> f32 {
        f1.min(f2)
    }

    pub fn subtraction(f1: f32, f2: f32) -> f32 {
        f1.max(-f2)
    }

    pub fn intersection(f1: f32, f2: f32) -> f32 {
        f1.max(f2)
    }

    pub fn repeat(p: Vec3, s: Vec3) -> Vec3 {
        p - s * (p / s).round()
    }

    pub fn sphere(p: Vec3, r: f32) -> f32 {
        p.length() - r
    }

    pub fn rect(p: Vec3, b: Vec3) -> f32 {
        let q = p.abs() - b;

        q.max(Vec3::ZERO).length()
            + q.max_element().min(0.0)
    }

    /// Thanks to: https://www.shadertoy.com/view/MdXyzX.
    pub fn ocean(time: f32, point: Vec3) -> f32 {
        // Origin (the point at (0,0)) contains a ripple-artifact that doesn't
        // look great - to avoid it, let's offset the ocean
        let point = point + vec3(128.0, 0.0, 128.0);

        // Also, the default animation speed is kinda slow, so let's speed it up
        let time = 2.0 * time;

        // ---

        let mut h_sum = 0.0;
        let mut h_weight = 0.0;

        let mut wave_pos = point.xz();
        let mut wave_freq = 1.0;
        let mut wave_weight = 1.0;

        let mut noise = 0.0f32;

        for _ in 0..15 {
            let wave_dir = vec2(noise.cos(), noise.sin());

            let wave =
                wave_dir.dot(wave_pos) * wave_freq + time;

            let wave_h = (wave.sin() - 1.0).exp();
            let wave_dh = -wave_h * wave.cos();

            h_sum += wave_h * wave_weight;
            h_weight += wave_weight;

            wave_pos +=
                0.25 * wave_dh * wave_dir * wave_weight;

            wave_freq *= 1.18;
            wave_weight *= 0.82;

            noise += 1234.4321;
        }

        point.y - (h_sum / h_weight)
    }
}
