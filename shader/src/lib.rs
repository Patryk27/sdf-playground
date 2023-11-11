#![cfg_attr(target_arch = "spirv", no_std)]

use sdf_playground_common::Params;
use spirv_std::glam::*;
#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::*;
use spirv_std::spirv;

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

/// Fragment shader, generates color for each pixel on the screen.
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

    // Where the sun is located
    let sun_pos = vec3(50.0, 100.0, 50.0);

    // Where the camera is located
    let ray_origin = vec3(8.0, 4.0, 8.0);

    // Where the camera is looking at
    let ray_direction =
        sdf_playground_common::direction(ray_origin, uv);

    // ---

    let hit_point = march(time, ray_origin, ray_direction);

    *out_color = if hit_point.is_finite() {
        // We hit something - let's perform shading
        let hit_normal = normal(time, hit_point);

        let sun_dir = (sun_pos - hit_point).normalize();

        let diffuse = hit_normal
            .dot(sun_pos.normalize())
            .clamp(0.0, 1.0);

        let diffuse = vec3(0.02, 0.19, 0.58) * diffuse;

        // TODO incorporate half-vector
        let specular = hit_normal
            .dot(sun_dir)
            .clamp(0.0, 1.0)
            .powf(50.0);

        let specular = vec3(1.0, 1.0, 1.0) * specular;

        (diffuse + specular).extend(1.0)
    } else {
        // We hit nothing - let's output the background color
        vec4(0.0, 0.0, 0.0, 1.0)
    };
}

fn scene(t: f32, p: Vec3) -> f32 {
    if p.length() <= 25.0 {
        let a = ocean(t, p);
        let b = sdf::sphere(p, 7.0);

        sdf::intersection(a, b)
    } else {
        f32::MAX
    }
}

/// SDF describing an ocean.
///
/// Thanks to https://www.shadertoy.com/view/MdXyzX.
fn ocean(t: f32, p: Vec3) -> f32 {
    let mut h_sum = 0.0;
    let mut h_weight = 0.0;

    let mut wave_pos = p.xz();
    let mut wave_freq = 1.0;
    let mut wave_weight = 1.0;

    let mut noise = 0.0f32;

    for _ in 0..20 {
        let wave_dir = vec2(noise.cos(), noise.sin());

        let wave = wave_dir.dot(wave_pos) * wave_freq + t;
        let wave_h = (wave.sin() - 1.0).exp();
        let wave_dh = -wave_h * wave.cos();

        h_sum += wave_h * wave_weight;
        h_weight += wave_weight;

        wave_pos += 0.28 * wave_dh * wave_dir * wave_weight;
        wave_freq *= 1.18;
        wave_weight *= 0.82;

        noise += 1234.4321;
    }

    p.y - (h_sum / h_weight)
}

fn march(time: f32, origin: Vec3, direction: Vec3) -> Vec3 {
    const STEPS: u32 = 128;

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

/// Returns normal of the surface at point `p`.
///
/// Intuitively, normal describes the orientation of surface at given point,
/// which allows us to make it brighter / dimmer, depending on whether that
/// surface looks _at_ the sun or _away_ from it.
fn normal(t: f32, p: Vec3) -> Vec3 {
    let d = 0.001;
    let dx = vec3(d, 0.0, 0.0);
    let dy = vec3(0.0, d, 0.0);
    let dz = vec3(0.0, 0.0, d);

    let gx = scene(t, p + dx) - scene(t, p - dx);
    let gy = scene(t, p + dy) - scene(t, p - dy);
    let gz = scene(t, p + dz) - scene(t, p - dz);

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
}
