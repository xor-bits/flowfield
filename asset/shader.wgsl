//!include "./noise.wgsl"

struct VertexInput {
    @builtin(vertex_index) i: u32,
    @location(0) pos: vec4<f32>,
};

struct FragmentInput {
    @builtin(position) _pos: vec4<f32>,
    @location(0) pos: vec2<f32>,
    @location(1) mid: vec2<f32>,
};

struct DrawPush {
    mvp: mat4x4<f32>,
};

struct UpdatePush {
    time: f32,
};

var<push_constant> draw_push: DrawPush;

var<push_constant> update_push: UpdatePush;

const pt_size = 0.005;

@vertex
fn vs_main(vin: VertexInput) -> FragmentInput {
    let l = -1.732 * pt_size;
    let t = -2.0 * pt_size;
    let b = 1.0 * pt_size;

    if (vin.i == 0u) {
        let pos = vec2<f32>(vin.pos.x, vin.pos.y + t);
        return FragmentInput(
            draw_push.mvp * vec4<f32>(pos.x, pos.y, 0.0, 1.0),
            pos,
            vin.pos.xy,
        );
    } else if (vin.i == 1u) {
        let pos = vec2<f32>(vin.pos.x + l, vin.pos.y + b);
        return FragmentInput(
            draw_push.mvp * vec4<f32>(pos.x, pos.y, 0.0, 1.0),
            pos,
            vin.pos.xy,
        );
    } else {
        let pos = vec2<f32>(vin.pos.x - l, vin.pos.y + b);
        return FragmentInput(
            draw_push.mvp * vec4<f32>(pos.x, pos.y, 0.0, 1.0),
            pos,
            vin.pos.xy,
        );
    }
}

@fragment
fn fs_main(fin: FragmentInput) -> @location(0) vec4<f32> {
    let d = fin.pos - fin.mid;
    let t = smoothstep(0.0, -pt_size * pt_size, d.x * d.x + d.y * d.y - pt_size * pt_size);
    return vec4<f32>(1.0, 1.0, 1.0, t * 0.001);
}

@group(0)
@binding(0)
var<storage, read_write> points: array<vec4<f32>>;

@compute
@workgroup_size(512, 1, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    // let i = id.x + id.y * 8u + id.z * 64u;
    let i = id.x;

    let now = points[i];
    var pos = now.xy;
    let time = 0.01 * update_push.time;
    // let time = 10.0 * update_push.time;
    let dir = vec2<f32>(
        simplex_noise_3d(vec3<f32>(pos, time - 1000.0)),
        simplex_noise_3d(vec3<f32>(pos, time + 1000.0)),
    );
    var vel = now.zw * 0.997 + dir * 0.00001;
    // let vel = now.zw * 0.95 + dir * 0.01;
    pos += vel;

    // pos.x = ((fract(pos.x * 0.5 + 0.5)) * 2.0 - 1.0);
    // pos.y = ((fract(pos.x * 0.5 + 0.5)) * 2.0 - 1.0);
    pos = ((fract(pos * 0.5 + 0.5)) * 2.0 - 1.0);

    points[i] = vec4<f32>(pos, vel);
}
