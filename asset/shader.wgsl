//!include "./noise.wgsl"

/* struct VertexInput {
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
}; */

// struct UpdatePush {
//     time: f32,
// };

/* var<push_constant> draw_push: DrawPush;

struct UpdatePush {
    time: f32,
};
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
} */

struct UpdatePush {
    cursor: vec2<f32>,
    time: f32,
    flags: u32,
};

var<push_constant> update_push: UpdatePush;

struct ShadowPush {
    time: f32,
    flags: u32,
};

var<push_constant> shadow_push: ShadowPush;

@group(0)
@binding(0)
var texture: texture_storage_2d<r32float, read_write>;

@group(0)
@binding(1)
var<storage, read_write> points: array<vec4<f32>>;

// @group(0)
// @binding(0)
// var s_texture: texture_storage_2d<r32float, read_write>;

@compute
@workgroup_size(16, 16, 1)
fn cs_main_shadow(@builtin(global_invocation_id) id: vec3<u32>) {
    var shadow_sub = 0.00015;
    var shadow_mul = 0.99;
    if (shadow_push.flags & 1u) == 1u {
        shadow_sub = 0.005;
        shadow_mul = 0.98;
    }

    let coords = id.xy;
    var pix = textureLoad(texture, coords);
    if (shadow_push.flags & 2u) == 2u {
        pix -= shadow_sub;
    } else {
        pix *= shadow_mul;
    }

    textureStore(texture, coords, pix);
}

@compute
@workgroup_size(512, 1, 1)
fn cs_main_update(@builtin(global_invocation_id) id: vec3<u32>) {
    // let i = id.x + id.y * 8u + id.z * 64u;
    let i = id.x;

    if i >= arrayLength(&points) {
        return;
    }

    let now = points[i];
    var pos = now.xy;
    let time = 0.005 * update_push.time;
    // let time = 10.0 * update_push.time;
    /* let dir = vec2<f32>(
        simplex_noise_3d(vec3<f32>(pos, time - 1000.0)),
        simplex_noise_3d(vec3<f32>(pos, time + 1000.0)),
    ); */

    let cursor_flipped = update_push.cursor / vec2<f32>(textureDimensions(texture)) * 2.0 - 1.0;
    let cursor = vec2<f32>(cursor_flipped.x, -cursor_flipped.y);
    let dir = cursor - pos;
    // var vel = now.zw * 0.9985 + dir * 0.00001;
    let vel = normalize(dir) / length(dir) * 0.001;
    // let vel = now.zw * 0.95 + dir * 0.01;
    pos += vel;

    // pos.x = ((fract(pos.x * 0.5 + 0.5)) * 2.0 - 1.0);
    // pos.y = ((fract(pos.x * 0.5 + 0.5)) * 2.0 - 1.0);
    pos = ((fract(pos * 0.5 + 0.5)) * 2.0 - 1.0);

    points[i] = vec4<f32>(pos, vel);

    let coords = vec2<u32>((pos + 1.0) * 0.5 * vec2<f32>(textureDimensions(texture)));

    var point = 0.008;
    if (update_push.flags & 4u) == 4u {
        point = 1.0;
    }

    // textureStore(texture, coords, vec4<f32>(1.0));// min(textureLoad(texture, coords) + 0.5, vec4<f32>(1.0)));
    // textureStore(texture, vec2<u32>(10u, 10u), vec4<f32>(1.0));

    textureStore(texture, coords, min(textureLoad(texture, coords) + point, vec4<f32>(1.0)));
}
