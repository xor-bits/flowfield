struct VertexInput {
    @builtin(vertex_index) vi: u32,
};

struct FragmentInput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct Push {
    flags: u32,
};

@group(0)
@binding(0)
var texture_t: texture_2d<f32>;

@group(0)
@binding(1)
var texture_s: sampler;

var<push_constant> push: Push;

@vertex
fn vs_main(vin: VertexInput) -> FragmentInput {
    let uv = vec2<f32>(f32(vin.vi % 2u), f32(vin.vi / 2u));

    var fin: FragmentInput;
    fin.uv = vec2<f32>(uv.x, uv.y);
    fin.pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    return fin;
}

@fragment
fn fs_main(fin: FragmentInput) -> @location(0) vec4<f32> {
    if push.flags == 0u {
        /* let o = 1.0 / vec2<f32>(textureDimensions(texture_t));
        let c = textureSample(texture_t, texture_s, fin.uv + vec2f(0.0, 0.0)).r * 0.11111111
            + textureSample(texture_t, texture_s, fin.uv + vec2f(o.x, o.y)).r * 0.11111111
            + textureSample(texture_t, texture_s, fin.uv + vec2f(0.0, o.y)).r * 0.11111111
            + textureSample(texture_t, texture_s, fin.uv + vec2f(-o.x, o.y)).r * 0.11111111
            + textureSample(texture_t, texture_s, fin.uv + vec2f(o.x,  0.0)).r * 0.11111111
            + textureSample(texture_t, texture_s, fin.uv + vec2f(0.0,  0.0)).r * 0.11111111
            + textureSample(texture_t, texture_s, fin.uv + vec2f(-o.x, 0.0)).r * 0.11111111
            + textureSample(texture_t, texture_s, fin.uv + vec2f(o.x, -o.y)).r * 0.11111111
            + textureSample(texture_t, texture_s, fin.uv + vec2f(0.0, -o.y)).r * 0.11111111
            + textureSample(texture_t, texture_s, fin.uv + vec2f(-o.x,-o.y)).r * 0.11111111; */

        let c = textureSample(texture_t, texture_s, fin.uv).r;

        return vec4<f32>(c, c, c, 1.0);
    } else {
        return vec4<f32>(fin.uv, 0.0, 1.0);
    }
}
