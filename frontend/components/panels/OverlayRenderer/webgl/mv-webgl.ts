/**
 * WebGL2 MV (Motion Vector) Line Renderer
 *
 * Uses GL_LINES for maximum throughput when MV grid density exceeds
 * WEBGL_MV_THRESHOLD blocks. Falls back gracefully when WebGL2 is unavailable.
 *
 * Vertex format: vec2 position (2 × float32 = 8 bytes per vertex)
 * Two draw calls: L0 (cyan) and L1 (orange)
 */

import type { FrameInfo } from "../../../../types/video";

// ─── Public threshold ────────────────────────────────────────────────────────

/** Minimum block count that switches rendering to the WebGL path. */
export const WEBGL_MV_THRESHOLD = 300;

// ─── Shaders ─────────────────────────────────────────────────────────────────

const VERT_SRC = /* glsl */ `#version 300 es
in vec2 a_pos;
uniform vec2 u_resolution;
void main() {
  vec2 clip = (a_pos / u_resolution) * 2.0 - 1.0;
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
}
`;

const FRAG_SRC = /* glsl */ `#version 300 es
precision mediump float;
uniform vec4 u_color;
out vec4 fragColor;
void main() { fragColor = u_color; }
`;

// ─── WebGL state cache ────────────────────────────────────────────────────────

interface WebGLState {
  gl: WebGL2RenderingContext;
  program: WebGLProgram;
  buffer: WebGLBuffer;
  aPos: number;
  uResolution: WebGLUniformLocation;
  uColor: WebGLUniformLocation;
}

const stateCache = new WeakMap<HTMLCanvasElement, WebGLState>();

// ─── Internal helpers ─────────────────────────────────────────────────────────

function compileShader(
  gl: WebGL2RenderingContext,
  type: number,
  src: string,
): WebGLShader {
  const shader = gl.createShader(type);
  if (!shader) throw new Error("Failed to create shader");
  gl.shaderSource(shader, src);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader) ?? "unknown error";
    gl.deleteShader(shader);
    throw new Error(`Shader compile error: ${info}`);
  }
  return shader;
}

function createProgram(gl: WebGL2RenderingContext): WebGLProgram {
  const vert = compileShader(gl, gl.VERTEX_SHADER, VERT_SRC);
  const frag = compileShader(gl, gl.FRAGMENT_SHADER, FRAG_SRC);
  const prog = gl.createProgram();
  if (!prog) throw new Error("Failed to create program");
  gl.attachShader(prog, vert);
  gl.attachShader(prog, frag);
  gl.linkProgram(prog);
  gl.deleteShader(vert);
  gl.deleteShader(frag);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(prog) ?? "unknown error";
    gl.deleteProgram(prog);
    throw new Error(`Program link error: ${info}`);
  }
  return prog;
}

function getOrCreateState(canvas: HTMLCanvasElement): WebGLState | null {
  const cached = stateCache.get(canvas);
  if (cached) return cached;

  const gl = canvas.getContext("webgl2", {
    alpha: true,
    premultipliedAlpha: false,
    antialias: false,
    depth: false,
    stencil: false,
  }) as WebGL2RenderingContext | null;

  if (!gl) return null;

  try {
    const program = createProgram(gl);
    const buffer = gl.createBuffer();
    if (!buffer) throw new Error("Failed to create buffer");

    const aPos = gl.getAttribLocation(program, "a_pos");
    const uResolution = gl.getUniformLocation(program, "u_resolution");
    const uColor = gl.getUniformLocation(program, "u_color");

    if (uResolution === null || uColor === null) {
      throw new Error("Failed to locate uniforms");
    }

    const state: WebGLState = {
      gl,
      program,
      buffer,
      aPos,
      uResolution,
      uColor,
    };
    stateCache.set(canvas, state);
    return state;
  } catch {
    return null;
  }
}

// ─── Line segment builder ────────────────────────────────────────────────────

/** Scale factor applied to MV displacement for visual clarity. */
const MV_SCALE = 0.5;

function buildLines(
  mvArray: { dx_qpel: number; dy_qpel: number }[],
  gridW: number,
  gridH: number,
  blockW: number,
  blockH: number,
): Float32Array {
  // Pre-allocate worst case: 4 floats per line segment (2 vertices × vec2)
  const buf = new Float32Array(gridW * gridH * 4);
  let offset = 0;

  for (let row = 0; row < gridH; row++) {
    for (let col = 0; col < gridW; col++) {
      const mv = mvArray[row * gridW + col];
      if (!mv) continue;

      const x = mv.dx_qpel / 4;
      const y = mv.dy_qpel / 4;

      if (x === 0 && y === 0) continue;
      // Skip sentinel values
      if (mv.dx_qpel === 2147483647 || mv.dy_qpel === 2147483647) continue;

      const cx = (col + 0.5) * blockW;
      const cy = (row + 0.5) * blockH;

      buf[offset++] = cx;
      buf[offset++] = cy;
      buf[offset++] = cx + x * MV_SCALE;
      buf[offset++] = cy + y * MV_SCALE;
    }
  }

  return buf.subarray(0, offset);
}

// ─── Public API ──────────────────────────────────────────────────────────────

/**
 * Render motion vectors on `canvas` using WebGL2 line primitives.
 *
 * @returns `true` on success; `false` if WebGL2 is unavailable or there is no
 *          MV data to render.
 */
export function renderMVWebGL(
  canvas: HTMLCanvasElement,
  frame: FrameInfo,
  frameWidth: number,
  frameHeight: number,
): boolean {
  const mvGrid = frame.mv_grid;
  if (!mvGrid) return false;

  const { mv_l0, mv_l1, block_w, block_h, grid_w, grid_h } = mvGrid;

  const state = getOrCreateState(canvas);
  if (!state) return false;

  const { gl, program, buffer, aPos, uResolution, uColor } = state;

  // Resize canvas if necessary
  if (canvas.width !== frameWidth || canvas.height !== frameHeight) {
    canvas.width = frameWidth;
    canvas.height = frameHeight;
  }

  gl.viewport(0, 0, frameWidth, frameHeight);
  gl.clearColor(0, 0, 0, 0);
  gl.clear(gl.COLOR_BUFFER_BIT);

  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  gl.useProgram(program);
  gl.uniform2f(uResolution, frameWidth, frameHeight);

  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.enableVertexAttribArray(aPos);
  gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);

  // ── Draw L0 (cyan: rgba(100, 200, 255, 0.85)) ─────────────────────────────
  if (mv_l0 && mv_l0.length > 0) {
    const l0Verts = buildLines(mv_l0, grid_w, grid_h, block_w, block_h);
    if (l0Verts.length >= 4) {
      gl.bufferData(gl.ARRAY_BUFFER, l0Verts, gl.DYNAMIC_DRAW);
      gl.uniform4f(uColor, 100 / 255, 200 / 255, 255 / 255, 0.85);
      gl.drawArrays(gl.LINES, 0, l0Verts.length / 2);
    }
  }

  // ── Draw L1 (orange: rgba(255, 160, 50, 0.85)) ────────────────────────────
  if (mv_l1 && mv_l1.length > 0) {
    const l1Verts = buildLines(mv_l1, grid_w, grid_h, block_w, block_h);
    if (l1Verts.length >= 4) {
      gl.bufferData(gl.ARRAY_BUFFER, l1Verts, gl.DYNAMIC_DRAW);
      gl.uniform4f(uColor, 255 / 255, 160 / 255, 50 / 255, 0.85);
      gl.drawArrays(gl.LINES, 0, l1Verts.length / 2);
    }
  }

  return true;
}

/**
 * Clear the WebGL overlay canvas (transparent black).
 * Safe to call even when WebGL2 is not available — falls back to 2D clear.
 */
export function clearMVWebGL(canvas: HTMLCanvasElement): void {
  const state = stateCache.get(canvas);
  if (state) {
    const { gl } = state;
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    return;
  }

  // Fallback: try 2D context clear
  const ctx2d = canvas.getContext("2d");
  if (ctx2d) {
    ctx2d.clearRect(0, 0, canvas.width, canvas.height);
  }
}
