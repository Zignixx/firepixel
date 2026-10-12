// Vertex shader bindings

struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>, // gamma 0-1
    @builtin(position) position: vec4<f32>,
};

struct Locals {
    screen_size: vec2<f32>,

    /// 1 if dithering is enabled, 0 otherwise
    dithering: u32,

    /// 1 to do manual filtering for more predictable kittest snapshot images.
    /// See also https://github.com/emilk/egui/issues/5295
    predictable_texture_filtering: u32,
};
@group(0) @binding(0) var<uniform> r_locals: Locals;


// -----------------------------------------------
// Adapted from
// https://www.shadertoy.com/view/llVGzG
// Originally presented in:
// Jimenez 2014, "Next Generation Post-Processing in Call of Duty"
//
// A good overview can be found in
// https://blog.demofox.org/2022/01/01/interleaved-gradient-noise-a-different-kind-of-low-discrepancy-sequence/
// via https://github.com/rerun-io/rerun/
fn interleaved_gradient_noise(n: vec2<f32>) -> f32 {
    let f = 0.06711056 * n.x + 0.00583715 * n.y;
    return fract(52.9829189 * fract(f));
}

fn dither_interleaved(rgb: vec3<f32>, levels: f32, frag_coord: vec4<f32>) -> vec3<f32> {
    var noise = interleaved_gradient_noise(frag_coord.xy);
    // scale down the noise slightly to ensure flat colors aren't getting dithered
    noise = (noise - 0.5) * 0.95;
    return rgb + noise / (levels - 1.0);
}

// 0-1 linear  from  0-1 sRGB gamma
fn linear_from_gamma_rgb(srgb: vec3<f32>) -> vec3<f32> {
    let cutoff = srgb < vec3<f32>(0.04045);
    let lower = srgb / vec3<f32>(12.92);
    let higher = pow((srgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    return select(higher, lower, cutoff);
}

// 0-1 sRGB gamma  from  0-1 linear
fn gamma_from_linear_rgb(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb < vec3<f32>(0.0031308);
    let lower = rgb * vec3<f32>(12.92);
    let higher = vec3<f32>(1.055) * pow(rgb, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(higher, lower, cutoff);
}

// 0-1 sRGBA gamma  from  0-1 linear
fn gamma_from_linear_rgba(linear_rgba: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(gamma_from_linear_rgb(linear_rgba.rgb), linear_rgba.a);
}

// [u8; 4] SRGB as u32 -> [r, g, b, a] in 0.-1
fn unpack_color(color: u32) -> vec4<f32> {
    return vec4<f32>(
        f32(color & 255u),
        f32((color >> 8u) & 255u),
        f32((color >> 16u) & 255u),
        f32((color >> 24u) & 255u),
    ) / 255.0;
}

fn position_from_screen(screen_pos: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(
        2.0 * screen_pos.x / r_locals.screen_size.x - 1.0,
        1.0 - 2.0 * screen_pos.y / r_locals.screen_size.y,
        0.0,
        1.0,
    );
}

@vertex
fn vs_main(
    @location(0) a_pos: vec2<f32>,
    @location(1) a_tex_coord: vec2<f32>,
    @location(2) a_color: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = a_tex_coord;
    out.color = unpack_color(a_color);
    out.position = position_from_screen(a_pos);
    return out;
}

// Fragment shader bindings

@group(1) @binding(0) var r_tex_color: texture_2d<f32>;
@group(1) @binding(1) var r_tex_sampler: sampler;

// Firepixel-Anpassung (FV-8): Software-Rasterer (z. B. "Microsoft Basic Render Driver" ohne
// Grafikkarte, Fernsitzung) sind mit `textureSample` etwa 20-mal langsamer als mit `textureLoad`
// (gemessen 47 gegen 2,3 ms je Vollbild 1800x1000). Mit MANUAL_SAMPLING = 1 wird die Filterung
// des Samplers (Filter je Vergroesserung/Verkleinerung, Randmodus) hier mit `textureLoad`
// nachgebildet; die Werte setzt der Renderer je Pipeline passend zu den Texturoptionen.
override MANUAL_SAMPLING: u32 = 0u;
// 0 = naechster Nachbar, 1 = linear
override MAG_LINEAR: u32 = 1u;
override MIN_LINEAR: u32 = 1u;
// 0 = am Rand festhalten, 1 = wiederholen, 2 = gespiegelt wiederholen
override WRAP_MODE: u32 = 0u;

fn wrap_texel(p: vec2<i32>, size: vec2<i32>) -> vec2<i32> {
    if WRAP_MODE == 1u {
        return ((p % size) + size) % size;
    }
    if WRAP_MODE == 2u {
        let period = size * 2;
        let m = ((p % period) + period) % period;
        return select(m, period - vec2<i32>(1, 1) - m, m >= size);
    }
    return clamp(p, vec2<i32>(0, 0), size - vec2<i32>(1, 1));
}

fn sample_manual(uv: vec2<f32>, linear: bool) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(r_tex_color, 0));
    let pos = uv * vec2<f32>(size);
    if !linear {
        return textureLoad(r_tex_color, wrap_texel(vec2<i32>(floor(pos)), size), 0);
    }
    let q = pos - 0.5;
    let f = fract(q);
    let i = vec2<i32>(floor(q));
    let tl = textureLoad(r_tex_color, wrap_texel(i, size), 0);
    if f.x == 0.0 && f.y == 0.0 {
        return tl;
    }
    let tr = textureLoad(r_tex_color, wrap_texel(i + vec2<i32>(1, 0), size), 0);
    let bl = textureLoad(r_tex_color, wrap_texel(i + vec2<i32>(0, 1), size), 0);
    let br = textureLoad(r_tex_color, wrap_texel(i + vec2<i32>(1, 1), size), 0);
    return mix(mix(tl, tr, f.x), mix(bl, br, f.x), f.y);
}

fn sample_texture(in: VertexOutput) -> vec4<f32> {
    if MANUAL_SAMPLING == 1u {
        var linear = MAG_LINEAR == 1u;
        if MAG_LINEAR != MIN_LINEAR {
            // wie die Hardware: Verkleinerung, wenn ein Bildpunkt mehr als einen Texel ueberdeckt
            let texel = in.tex_coord * vec2<f32>(textureDimensions(r_tex_color, 0));
            let footprint = max(length(dpdx(texel)), length(dpdy(texel)));
            linear = select(MAG_LINEAR == 1u, MIN_LINEAR == 1u, footprint > 1.0);
        }
        return sample_manual(in.tex_coord, linear);
    }
    if r_locals.predictable_texture_filtering == 0 {
        // Hardware filtering: fast, but varies across GPUs and drivers.
        return textureSample(r_tex_color, r_tex_sampler, in.tex_coord);
    } else {
        // Manual bilinear filtering with four taps at pixel centers using textureLoad
        let texture_size = vec2<i32>(textureDimensions(r_tex_color, 0));
        let texture_size_f = vec2<f32>(texture_size);
        let pixel_coord = in.tex_coord * texture_size_f - 0.5;
        let pixel_fract = fract(pixel_coord);
        let pixel_floor = vec2<i32>(floor(pixel_coord));

        // Manual texture clamping
        let max_coord = texture_size - vec2<i32>(1, 1);
        let p00 = clamp(pixel_floor + vec2<i32>(0, 0), vec2<i32>(0, 0), max_coord);
        let p10 = clamp(pixel_floor + vec2<i32>(1, 0), vec2<i32>(0, 0), max_coord);
        let p01 = clamp(pixel_floor + vec2<i32>(0, 1), vec2<i32>(0, 0), max_coord);
        let p11 = clamp(pixel_floor + vec2<i32>(1, 1), vec2<i32>(0, 0), max_coord);

        // Load at pixel centers
        let tl = textureLoad(r_tex_color, p00, 0);
        let tr = textureLoad(r_tex_color, p10, 0);
        let bl = textureLoad(r_tex_color, p01, 0);
        let br = textureLoad(r_tex_color, p11, 0);

        // Manual bilinear interpolation
        let top = mix(tl, tr, pixel_fract.x);
        let bottom = mix(bl, br, pixel_fract.x);
        return mix(top, bottom, pixel_fract.y);
    }
}

@fragment
fn fs_main_linear_framebuffer(in: VertexOutput) -> @location(0) vec4<f32> {
    // We expect "normal" textures that are NOT sRGB-aware.
    let tex_gamma = sample_texture(in);
    var out_color_gamma = in.color * tex_gamma;
    // Dither the float color down to eight bits to reduce banding.
    // This step is optional for egui backends.
    // Note that dithering is performed on the gamma encoded values,
    // because this function is used together with a srgb converting target.
    if r_locals.dithering == 1 {
        let out_color_gamma_rgb = dither_interleaved(out_color_gamma.rgb, 256.0, in.position);
        out_color_gamma = vec4<f32>(out_color_gamma_rgb, out_color_gamma.a);
    }
    let out_color_linear = linear_from_gamma_rgb(out_color_gamma.rgb);
    return vec4<f32>(out_color_linear, out_color_gamma.a);
}

@fragment
fn fs_main_gamma_framebuffer(in: VertexOutput) -> @location(0) vec4<f32> {
    // We expect "normal" textures that are NOT sRGB-aware.
    let tex_gamma = sample_texture(in);
    var out_color_gamma = in.color * tex_gamma;
    // Dither the float color down to eight bits to reduce banding.
    // This step is optional for egui backends.
    if r_locals.dithering == 1 {
        let out_color_gamma_rgb = dither_interleaved(out_color_gamma.rgb, 256.0, in.position);
        out_color_gamma = vec4<f32>(out_color_gamma_rgb, out_color_gamma.a);
    }
    return out_color_gamma;
}
