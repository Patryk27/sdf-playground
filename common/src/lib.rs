#![cfg_attr(target_arch = "spirv", no_std)]

use bytemuck::*;
use glam::*;

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct Params {
    pub width: u32,
    pub height: u32,
    pub time: f32,
}

pub fn direction(origin: Vec3, uv: Vec2) -> Vec3 {
    let camera = {
        let up = vec3(0.0, 1.0, 0.0);
        let f = -origin.normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);

        Mat3 {
            x_axis: s,
            y_axis: u,
            z_axis: f,
        }
    };

    let uv = uv.xy() * 2.0 - 1.0;
    let uv = vec2(uv.x, -uv.y);

    (camera * uv.extend(1.0)).normalize()
}
