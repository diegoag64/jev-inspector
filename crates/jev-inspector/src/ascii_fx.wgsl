// A procedural ASCII field, drawn without a texture atlas or per-character nodes.
// Rune supplies the slot rectangle, clipping, layer opacity and display scale.
struct U {
    rect: vec4f, clip: vec4f, fade: vec4u, frame: vec4f,
    signal: vec4f, flags: vec4f,
}
@group(0) @binding(0) var<uniform> u: U;
struct V { @builtin(position) pos: vec4f }
@vertex fn vs(@builtin(vertex_index) vi: u32) -> V {
    let corner = vec2f(f32(vi & 1u), f32(vi >> 1u));
    let p = (u.rect.xy + corner * u.rect.zw - u.frame.xy) / u.frame.zw;
    return V(vec4f(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0));
}

// Five-by-seven glyphs: dot, colon, plus, slash, hash.
fn glyph(kind: u32, p: vec2u) -> f32 {
    if p.x >= 5u || p.y >= 7u { return 0.0; }
    var bits = 0u;
    switch kind {
        case 0u: { if p.y == 5u { bits = 4u; } }
        case 1u: { if p.y == 2u || p.y == 5u { bits = 4u; } }
        case 2u: { bits = select(select(0u, 4u, p.y >= 1u && p.y <= 5u), 31u, p.y == 3u); }
        case 3u: { bits = 1u << min(4u, (6u - p.y) * 5u / 7u); }
        default: { bits = select(10u, 31u, p.y == 2u || p.y == 4u); }
    }
    return f32((bits >> p.x) & 1u);
}

fn encode(x: f32) -> f32 {
    return select(1.055 * pow(x, 1.0 / 2.4) - 0.055, x * 12.92, x <= 0.0031308);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
    let pos = v.pos.xy + u.frame.xy;
    let local = (pos - u.rect.xy) / max(u.signal.y, 0.1);
    let size = u.rect.zw / max(u.signal.y, 0.1);
    let phase = u.signal.x;
    // Fade the perturbation to zero at both ends, so stopping has no jump.
    let energy = sin(phase * 3.14159265);
    var alpha = 0.0;
    if u.signal.z < 0.5 {
        // Pixel-built lowercase j, with a quiet signal passing over its cells.
        let q = floor(local / size * 11.0);
        let stem = q.x >= 6.0 && q.x <= 7.0 && q.y >= 4.0 && q.y <= 8.0;
        let dot = q.x >= 6.0 && q.x <= 7.0 && q.y >= 1.0 && q.y <= 2.0;
        let hook = (q.y == 9.0 && q.x >= 3.0 && q.x <= 6.0) || (q.x == 2.0 && q.y >= 7.0 && q.y <= 8.0);
        let pixel = fract(local / size * 11.0);
        let inset = step(0.13, pixel.x) * step(pixel.x, 0.87) * step(0.13, pixel.y) * step(pixel.y, 0.87);
        let scan = exp(-pow((q.y / 11.0 - phase) * 5.0, 2.0));
        alpha = select(0.0, (0.64 + energy * scan * 0.36) * inset, stem || dot || hook);
    } else {
        let cell = floor(local / vec2f(8.0, 11.0));
        let grid = max(size / vec2f(8.0, 11.0), vec2f(1.0));
        let p = (cell + 0.5) / grid;
        let wave = sin(p.x * 12.0 + p.y * 3.0 + energy * 2.2) * 0.15;
        let band = exp(-pow((p.y - 0.5 - wave) * 3.3, 2.0));
        let edge = sin(clamp(p.x, 0.0, 1.0) * 3.14159265);
        let density = clamp(band * edge, 0.0, 0.999);
        let kind = u32(density * 5.0);
        let ink = glyph(kind, vec2u(floor(local - cell * vec2f(8.0, 11.0))));
        alpha = ink * (0.08 + density * 0.45);
    }
    alpha *= clip_alpha(pos, u.clip, u.fade);
    // Linear monochrome values, encoded only for Rune's optional sRGB blend mode.
    var ink = select(0.025, 0.82, u.signal.w > 0.5);
    if u.flags.x > 0.5 { ink = encode(ink); }
    return vec4f(vec3f(ink * alpha), alpha);
}
