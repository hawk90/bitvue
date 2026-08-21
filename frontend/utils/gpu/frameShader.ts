/**
 * WGSL fragment shader for YUV->RGB conversion.
 *
 * Ports the exact math from ../yuv/conversion.ts (BT601/BT709/BT2020 matrices)
 * so the color-space logic isn't reinvented, just relocated to the GPU:
 *   u0 = u - 128; v0 = v - 128
 *   r = y + rv*v0; g = y + gu*u0 + gv*v0; b = y + bu*u0
 * clamped to [0, 255].
 *
 * Chroma upsampling (4:2:0 / 4:2:2 / 4:4:4) needs no per-subsampling branch here --
 * the U/V planes are uploaded at their own native (already-subsampled) texture
 * dimensions, and sampling both textures with the same normalized [0,1] UV lets
 * the GPU's texture sampler do the upsampling. A `nearest` sampler (configured in
 * frameRenderer.ts) is required, not `linear`, to match ../yuv/chroma.ts's
 * nearest-neighbor `x>>1`/`y>>1` index strategy exactly -- linear filtering would
 * look smoother but would no longer be pixel-identical to the CPU path.
 */
export const FRAME_SHADER_WGSL = /* wgsl */ `
struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> VertexOutput {
  // Fullscreen triangle -- oversized so the visible NDC square [-1,1]x[-1,1] is
  // entirely covered by one triangle's interior/edges, cheaper than two triangles.
  var pos = array<vec2f, 3>(
    vec2f(-1.0, -1.0),
    vec2f(3.0, -1.0),
    vec2f(-1.0, 3.0),
  );
  var uv = array<vec2f, 3>(
    vec2f(0.0, 1.0),
    vec2f(2.0, 1.0),
    vec2f(0.0, -1.0),
  );
  var out: VertexOutput;
  out.position = vec4f(pos[vertexIndex], 0.0, 1.0);
  out.uv = uv[vertexIndex];
  return out;
}

struct ColorMatrix {
  rv: f32,
  gu: f32,
  gv: f32,
  bu: f32,
};

@group(0) @binding(0) var planeSampler: sampler;
@group(0) @binding(1) var yTexture: texture_2d<f32>;
@group(0) @binding(2) var uTexture: texture_2d<f32>;
@group(0) @binding(3) var vTexture: texture_2d<f32>;
@group(0) @binding(4) var<uniform> colorMatrix: ColorMatrix;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
  // r8unorm textures sample as [0,1] -- scale back to the [0,255] domain the
  // conversion math (ported directly from conversion.ts) is written in.
  let yVal = textureSample(yTexture, planeSampler, in.uv).r * 255.0;
  let u0 = textureSample(uTexture, planeSampler, in.uv).r * 255.0 - 128.0;
  let v0 = textureSample(vTexture, planeSampler, in.uv).r * 255.0 - 128.0;

  let r = yVal + colorMatrix.rv * v0;
  let g = yVal + colorMatrix.gu * u0 + colorMatrix.gv * v0;
  let b = yVal + colorMatrix.bu * u0;

  let rgb = clamp(vec3f(r, g, b), vec3f(0.0), vec3f(255.0)) / 255.0;
  return vec4f(rgb, 1.0);
}
`;
